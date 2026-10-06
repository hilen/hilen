use std::collections::HashMap;

use anyhow::{Context, Result, anyhow, bail};
use hilen_session::SecretStore;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str, from_value, to_string};

use crate::{
    deps::{
        hreads::{now, on_main, spawn},
        netrun::rest::{Call, RequestError},
    },
    google_access::{
        GoogleAccess, GoogleAccount, RenewError, Role,
        vault::{Entry, Vault},
    },
};

const DRIVE_URL: &str = "https://www.googleapis.com";
const VAULT_FILE: &str = "accounts.json";
/// An access token this close to its end is renewed before it is handed out.
const RENEW_BEFORE_SECONDS: f64 = 60.0;

static MAIN: SecretStore = SecretStore::new("google-main");
static STATE: Mutex<State> = Mutex::new(State::new());

struct State {
    /// The linked accounts as the last sync left them, removed ones too. In
    /// memory only, the hidden Drive folder of the main account keeps them.
    vault:     Vault,
    /// Access tokens by subject, with the time each one is good until.
    tokens:    Option<HashMap<String, (String, f64)>>,
    drive_url: Option<String>,
    /// The main account, read from the `SecretStore` once per run. Every
    /// read of the store can make the system ask the user, so it must not
    /// happen per call.
    main:      Main,
}

enum Main {
    NotRead,
    SignedOut,
    SignedIn(Stored),
}

impl State {
    const fn new() -> Self {
        Self {
            vault:     Vault::new(),
            tokens:    None,
            drive_url: None,
            main:      Main::NotRead,
        }
    }
}

/// What the main account keeps on this device, as JSON in a `SecretStore`.
#[derive(Clone, Serialize, Deserialize)]
struct Stored {
    subject:       String,
    email:         String,
    name:          Option<String>,
    picture:       Option<String>,
    refresh_token: String,
}

/// A Google account of the user, without its tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountInfo {
    /// Google's own id of the account. It never changes, an email can.
    pub subject: String,
    pub email:   String,
    pub name:    Option<String>,
    /// A link to the profile picture.
    pub picture: Option<String>,
    pub main:    bool,
}

/// The Google accounts of one user: 1 main account and any number of linked
/// ones, each with a fresh access token on request.
///
/// The refresh token of the main account sits in a `SecretStore` on this
/// device. The linked accounts sit in a file in the hidden app folder of the
/// Google Drive of the main account, so a sign in with the main account on
/// another device brings all of them. The main account needs the scope
/// `https://www.googleapis.com/auth/drive.appdata` for it.
///
/// Call [`GoogleAccess::set_server`] at launch, then [`GoogleAccounts::sync`]
/// when [`GoogleAccounts::main`] is some.
pub struct GoogleAccounts;

impl GoogleAccounts {
    /// The main account of this device, `None` when nobody is signed in.
    pub fn main() -> Option<AccountInfo> {
        stored_main().map(Stored::info)
    }

    /// The main account and the linked ones the last sync knew, main first.
    pub fn all() -> Vec<AccountInfo> {
        let linked = STATE.lock().vault.live().map(Entry::info).collect::<Vec<_>>();
        Self::main().into_iter().chain(linked).collect()
    }

    /// Signs the main account in and brings its linked accounts. `done` gets
    /// all of them. Call it straight from a tap, see `GoogleAccess::sign_in`.
    pub fn sign_in(done: impl FnOnce(Result<Vec<AccountInfo>>) + Send + 'static) {
        GoogleAccess::sign_in(Role::Main, move |account| {
            spawn(async move {
                let result = async {
                    let account = account?;
                    save_main(&account)?;
                    keep_token(&account);
                    Self::sync().await
                }
                .await;
                on_main(move || done(result));
            });
        });
    }

    /// Signs another account in and links it to the main one, on every
    /// device. `done` gets all accounts.
    pub fn link(done: impl FnOnce(Result<Vec<AccountInfo>>) + Send + 'static) {
        GoogleAccess::sign_in(Role::Linked, move |account| {
            spawn(async move {
                let result = async {
                    let account = account?;
                    let main = stored_main().context("link an account after the main one signed in")?;
                    if account.subject == main.subject {
                        bail!("{} is the main account already", account.email);
                    }
                    keep_token(&account);
                    STATE.lock().vault.put(Entry::of(&account, now()));
                    log::info!("google account {} linked", account.email);
                    Self::sync().await
                }
                .await;
                on_main(move || done(result));
            });
        });
    }

    /// Takes a linked account away on every device and ends its token at
    /// Google.
    pub async fn unlink(subject: &str) -> Result<Vec<AccountInfo>> {
        let token = {
            let mut state = STATE.lock();
            let token = state.vault.remove(subject, now());
            if let Some(tokens) = &mut state.tokens {
                tokens.remove(subject);
            }
            token
        };
        let token = token.with_context(|| format!("no linked account {subject}"))?;

        let all = Self::sync().await?;
        if let Err(error) = GoogleAccess::revoke(&token).await {
            log::warn!("the token of the unlinked account {subject} was not ended at Google: {error:#}");
        }
        log::info!("google account {subject} unlinked");
        Ok(all)
    }

    /// Forgets every account on this device. The linked accounts stay in the
    /// Drive folder for the next sign in. Nothing is ended at Google, a
    /// revoke would sign the other devices of the user out too.
    pub fn sign_out() -> Result<()> {
        let drive_url = STATE.lock().drive_url.clone();
        *STATE.lock() = State {
            drive_url,
            main: Main::SignedOut,
            ..State::new()
        };
        log::info!("google accounts signed out on this device");
        MAIN.clear()
    }

    /// Brings the linked accounts of this device and of the Drive folder
    /// together, and writes the result back when the folder was behind.
    pub async fn sync() -> Result<Vec<AccountInfo>> {
        let main = stored_main().context("nobody is signed in")?;
        // The error keeps its type, an app asks it if the main account has to
        // sign in again.
        let token = Self::access_token(&main.subject).await.map_err(anyhow::Error::new)?;
        let drive = Drive::new(token);

        let files = drive.find().await?;
        let mut remote = Vault::new();
        // What the file that is kept holds now. The other files, made when 2
        // devices started at the same moment, are only read.
        let mut first = Vault::new();
        for (index, file) in files.iter().enumerate() {
            match drive.read(file).await {
                Ok(vault) => {
                    if index == 0 {
                        first = vault.clone();
                    }
                    remote.merge(vault);
                }
                Err(error) => {
                    log::warn!("the accounts file {file} in the Drive folder does not read: {error:#}");
                }
            }
        }

        let merged = {
            let mut state = STATE.lock();
            state.vault.merge(remote);
            state.vault.clone()
        };

        if merged != first {
            let file = match files.first() {
                Some(file) => file.clone(),
                None => drive.create().await?,
            };
            drive.write(&file, &merged).await?;
            log::info!("the accounts file in the Drive folder is updated");
        }

        let all = Self::all();
        log::info!("google accounts synced, {} linked", all.len().saturating_sub(1));
        Ok(all)
    }

    /// An access token of this account that is good for at least a minute.
    /// `RenewError::SignInAgain` means the account has to sign in again:
    /// `sign_in` for the main one, `link` for a linked one.
    pub async fn access_token(subject: &str) -> Result<String, RenewError> {
        if let Some((token, good_until)) = STATE.lock().tokens.as_ref().and_then(|tokens| tokens.get(subject))
            && *good_until > now() + RENEW_BEFORE_SECONDS
        {
            return Ok(token.clone());
        }

        let (refresh_token, linked_accounts) = match stored_main() {
            Some(main) if main.subject == subject => {
                let linked = STATE.lock().vault.live().count();
                (
                    main.refresh_token,
                    Some(u32::try_from(linked).unwrap_or(u32::MAX)),
                )
            }
            _ => {
                let token = STATE
                    .lock()
                    .vault
                    .live()
                    .find(|entry| entry.subject == subject)
                    .map(|entry| entry.refresh_token.clone());
                (
                    token.with_context(|| format!("no Google account {subject}"))?,
                    None,
                )
            }
        };

        let renewed = GoogleAccess::renew(&refresh_token, linked_accounts).await?;
        log::debug!("google access token renewed for {subject}");
        #[allow(clippy::cast_precision_loss)]
        let good_until = now() + renewed.expires_in as f64;
        STATE
            .lock()
            .tokens
            .get_or_insert_default()
            .insert(subject.to_owned(), (renewed.token.clone(), good_until));
        Ok(renewed.token)
    }

    /// A newly made access token, for after Google answered 401 to the one
    /// `access_token` gave. A token can end before its time, when the user
    /// took the permission away and gave it back.
    pub async fn fresh_access_token(subject: &str) -> Result<String, RenewError> {
        if let Some(tokens) = &mut STATE.lock().tokens {
            tokens.remove(subject);
        }
        Self::access_token(subject).await
    }
}

#[cfg(test)]
impl GoogleAccounts {
    pub(super) fn use_drive(url: Option<String>) {
        *STATE.lock() = State {
            drive_url: url,
            ..State::new()
        };
    }

    pub(super) fn put_main(account: &GoogleAccount) -> Result<()> {
        save_main(account)?;
        keep_token(account);
        Ok(())
    }

    pub(super) fn put_linked(account: &GoogleAccount, at: f64) {
        STATE.lock().vault.put(Entry::of(account, at));
    }
}

impl Stored {
    fn info(self) -> AccountInfo {
        AccountInfo {
            subject: self.subject,
            email:   self.email,
            name:    self.name,
            picture: self.picture,
            main:    true,
        }
    }
}

fn stored_main() -> Option<Stored> {
    match &STATE.lock().main {
        Main::NotRead => {}
        Main::SignedOut => return None,
        Main::SignedIn(main) => return Some(main.clone()),
    }

    // A parse error would carry the token into a log.
    let main = MAIN.load().and_then(|json| {
        from_str(&json)
            .inspect_err(|_| log::warn!("the stored main Google account does not parse"))
            .ok()
    });
    STATE.lock().main = main.clone().map_or(Main::SignedOut, Main::SignedIn);
    main
}

fn save_main(account: &GoogleAccount) -> Result<()> {
    let stored = Stored {
        subject:       account.subject.clone(),
        email:         account.email.clone(),
        name:          account.name.clone(),
        picture:       account.picture.clone(),
        refresh_token: account.refresh_token.clone(),
    };
    MAIN.save(&to_string(&stored)?)?;
    STATE.lock().main = Main::SignedIn(stored);
    Ok(())
}

/// A sign in comes with an access token, no renewal is needed right after.
fn keep_token(account: &GoogleAccount) {
    #[allow(clippy::cast_precision_loss)]
    let good_until = now() + account.expires_in as f64;
    STATE.lock().tokens.get_or_insert_default().insert(
        account.subject.clone(),
        (account.access_token.clone(), good_until),
    );
}

/// The 4 calls to the app folder of Google Drive, with the access token of
/// the main account.
struct Drive {
    url:   String,
    token: String,
}

#[derive(Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<File>,
}

#[derive(Deserialize)]
struct File {
    id: String,
}

impl Drive {
    fn new(token: String) -> Self {
        let url = STATE.lock().drive_url.clone().unwrap_or_else(|| DRIVE_URL.to_owned());
        Self { url, token }
    }

    /// The ids of the accounts files, oldest first. There is 1, or 2 right
    /// after 2 devices made it at the same moment.
    async fn find(&self) -> Result<Vec<String>> {
        let call = Call::get(format!(
            "{}/drive/v3/files?spaces=appDataFolder&orderBy=createdTime&fields=files(id)&q=name%3D%27{VAULT_FILE}%27",
            self.url
        ));
        let list: FileList = self.send(call).await?;
        Ok(list.files.into_iter().map(|file| file.id).collect())
    }

    async fn read(&self, file: &str) -> Result<Vault> {
        let call = Call::get(format!("{}/drive/v3/files/{file}?alt=media", self.url));
        let json: Value = self.send(call).await.map_err(without_body)?;
        // The error of a parse straight from the body would carry the tokens.
        from_value(json).ok().context("the accounts file is not a vault")
    }

    async fn create(&self) -> Result<String> {
        let call = Call::post(format!("{}/drive/v3/files?fields=id", self.url)).body(serde_json::json!({
            "name": VAULT_FILE,
            "parents": ["appDataFolder"],
        }));
        let file: File = self.send(call).await?;
        Ok(file.id)
    }

    async fn write(&self, file: &str, vault: &Vault) -> Result<()> {
        let call = Call::patch(format!(
            "{}/upload/drive/v3/files/{file}?uploadType=media&fields=id",
            self.url
        ))
        .body(vault);
        let _: File = self.send(call).await?;
        Ok(())
    }

    async fn send<Out: serde::de::DeserializeOwned>(&self, call: Call) -> Result<Out> {
        match call.bearer(&self.token).send().await {
            Ok(out) => Ok(out),
            Err(RequestError::Unauthorized) => bail!("Google Drive refused the access token"),
            Err(error) => Err(error.into()),
        }
    }
}

/// An error of a request names the body it could not parse, and the body of
/// the accounts file holds tokens.
fn without_body(error: anyhow::Error) -> anyhow::Error {
    match error.downcast::<RequestError>() {
        Ok(RequestError::Parse { expected, .. }) => anyhow!("the accounts file is not {expected}"),
        Ok(error) => error.into(),
        Err(error) => error,
    }
}
