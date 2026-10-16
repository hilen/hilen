//! The accounts of one user on 2 devices, against the real routes of
//! `hilen-server` and a stand in for the app folder of Google Drive.

use std::{
    collections::HashMap,
    env::temp_dir,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    routing::{get, patch},
};
use hilen_session::{SecretStore, SessionStore};
use serde::Deserialize;
use serde_json::{Value, json};
use serial_test::serial;

use crate::google_access::{
    GoogleAccess, GoogleAccount, GoogleAccounts,
    client::test::{backend, serve_on_a_free_port},
};

/// The files of the folder, id, name and content, and how often one was
/// written.
#[derive(Default)]
struct Folder {
    files:  Vec<(String, String, Value)>,
    writes: u32,
}

impl Folder {
    fn content(&self, name: &str) -> Vec<String> {
        self.files
            .iter()
            .filter(|(_, have, _)| have == name)
            .map(|(_, _, content)| content.to_string())
            .collect()
    }
}

#[derive(Deserialize)]
struct NewFile {
    name: String,
}

type Drive = Arc<Mutex<Folder>>;

/// Drive answers 401 to a token it does not take. The main account has the
/// token of its sign in, and `access-2` after a renewal.
fn allowed(headers: &HeaderMap) -> Result<(), StatusCode> {
    let bearer = headers.get(AUTHORIZATION).and_then(|value| value.to_str().ok());
    if matches!(bearer, Some("Bearer access-main" | "Bearer access-2")) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Drive takes a query like `name='accounts.json'` and lists only those.
async fn list(
    State(drive): State<Drive>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    allowed(&headers)?;
    let name = query
        .get("q")
        .and_then(|q| q.strip_prefix("name='")?.strip_suffix('\''))
        .ok_or(StatusCode::BAD_REQUEST)?;
    let folder = drive.lock().expect("the test lock");
    let files: Vec<Value> = folder
        .files
        .iter()
        .filter(|(_, have, _)| have == name)
        .map(|(id, _, _)| json!({ "id": id }))
        .collect();
    Ok(Json(json!({ "files": files })))
}

async fn create(
    State(drive): State<Drive>,
    headers: HeaderMap,
    Json(file): Json<NewFile>,
) -> Result<Json<Value>, StatusCode> {
    allowed(&headers)?;
    let mut folder = drive.lock().expect("the test lock");
    let id = format!("file-{}", folder.files.len() + 1);
    folder.files.push((id.clone(), file.name, Value::Null));
    Ok(Json(json!({ "id": id })))
}

async fn read(
    State(drive): State<Drive>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    allowed(&headers)?;
    let folder = drive.lock().expect("the test lock");
    let file = folder
        .files
        .iter()
        .find(|(have, _, _)| *have == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(file.2.clone()))
}

async fn write(
    State(drive): State<Drive>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(content): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    allowed(&headers)?;
    let mut folder = drive.lock().expect("the test lock");
    folder.writes += 1;
    let file = folder
        .files
        .iter_mut()
        .find(|(have, _, _)| *have == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    file.2 = content;
    Ok(Json(json!({ "id": id })))
}

async fn fake_drive(drive: Drive) -> Result<String> {
    serve_on_a_free_port(
        Router::new()
            .route("/drive/v3/files", get(list).post(create))
            .route("/drive/v3/files/{id}", get(read))
            .route("/upload/drive/v3/files/{id}", patch(write))
            .with_state(drive),
    )
    .await
}

fn account(subject: &str, refresh_token: &str) -> GoogleAccount {
    GoogleAccount {
        subject:       subject.to_owned(),
        email:         format!("{subject}@example.com"),
        name:          None,
        picture:       None,
        refresh_token: refresh_token.to_owned(),
        access_token:  format!("access-{subject}"),
        expires_in:    3599,
    }
}

fn emails() -> Vec<String> {
    GoogleAccounts::all().into_iter().map(|account| account.email).collect()
}

/// A device that just started: it has the main account in its secret store
/// and nothing else.
fn start_device(drive_url: &str) {
    GoogleAccounts::use_drive(Some(drive_url.to_owned()));
}

#[tokio::test]
#[serial(google_access_server)]
async fn linked_accounts_follow_the_main_account_to_another_device() -> Result<()> {
    SessionStore::set_root(temp_dir().join("hilen-google-accounts-test"));
    SecretStore::keep_in_files();

    let before = GoogleAccess::swap_server(Some(backend().await?));
    let drive = Drive::default();
    let drive_url = fake_drive(drive.clone()).await?;

    // Device 1: the main account signs in and links 2 accounts.
    start_device(&drive_url);
    GoogleAccounts::put_main(&account("main", "refresh-main"))?;
    // With no linked account there is nothing to keep, and no file is made.
    assert_eq!(GoogleAccounts::sync().await?.len(), 1);
    assert_eq!(drive.lock().expect("the test lock").files.len(), 0);

    GoogleAccounts::put_linked(&account("work", "refresh-work"), 100.0);
    GoogleAccounts::put_linked(&account("club", "refresh-club"), 110.0);
    GoogleAccounts::sync().await?;
    assert_eq!(
        emails(),
        ["main@example.com", "club@example.com", "work@example.com"]
    );

    // A sync with nothing new writes nothing.
    let writes = drive.lock().expect("the test lock").writes;
    GoogleAccounts::sync().await?;
    assert_eq!(drive.lock().expect("the test lock").writes, writes);

    // Device 2: the same main account, and the linked ones come by themselves.
    start_device(&drive_url);
    assert_eq!(emails(), ["main@example.com"]);
    GoogleAccounts::put_main(&account("main", "refresh-main"))?;
    GoogleAccounts::sync().await?;
    assert_eq!(
        emails(),
        ["main@example.com", "club@example.com", "work@example.com"]
    );

    // A linked account gets its access token through the backend.
    assert_eq!(
        GoogleAccounts::access_token("work")
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        "access-2"
    );
    assert!(GoogleAccounts::access_token("nobody").await.is_err());

    // The token of the sign in is kept, a fresh one is asked from the backend.
    assert_eq!(
        GoogleAccounts::access_token("main")
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        "access-main"
    );
    assert_eq!(
        GoogleAccounts::fresh_access_token("main")
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        "access-2"
    );

    // Device 2 unlinks one. Its token is gone from the folder.
    GoogleAccounts::unlink("work").await?;
    assert_eq!(emails(), ["main@example.com", "club@example.com"]);
    let stored = drive.lock().expect("the test lock").files[0].2.to_string();
    assert!(!stored.contains("refresh-work"), "{stored}");
    assert!(stored.contains("refresh-club"));

    // Device 1 still has the old list in memory. Its sync must not bring
    // the unlinked account back.
    start_device(&drive_url);
    GoogleAccounts::put_main(&account("main", "refresh-main"))?;
    GoogleAccounts::put_linked(&account("work", "refresh-work"), 100.0);
    GoogleAccounts::put_linked(&account("club", "refresh-club"), 110.0);
    GoogleAccounts::sync().await?;
    assert_eq!(emails(), ["main@example.com", "club@example.com"]);

    // Signing out forgets the device, the folder keeps the accounts.
    GoogleAccounts::sign_out()?;
    assert!(GoogleAccounts::main().is_none());
    assert_eq!(emails(), Vec::<String>::new());
    assert!(GoogleAccounts::sync().await.is_err());
    assert!(
        drive.lock().expect("the test lock").files[0]
            .2
            .to_string()
            .contains("refresh-club")
    );

    GoogleAccounts::use_drive(None);
    GoogleAccess::swap_server(before);
    Ok(())
}

#[tokio::test]
#[serial(google_access_server)]
async fn two_files_from_two_devices_become_one_list() -> Result<()> {
    SessionStore::set_root(temp_dir().join("hilen-google-accounts-test"));
    SecretStore::keep_in_files();

    let before = GoogleAccess::swap_server(Some(backend().await?));
    let drive = Drive::default();
    let drive_url = fake_drive(drive.clone()).await?;

    // Both devices made the file at the same moment, each with its account.
    for (id, subject) in [("file-1", "work"), ("file-2", "club")] {
        let file = json!({ "version": 1, "accounts": [{
            "subject": subject, "email": format!("{subject}@example.com"), "name": null, "picture": null,
            "refresh_token": format!("refresh-{subject}"), "updated_at": 100.0, "removed": false,
        }]});
        drive
            .lock()
            .expect("the test lock")
            .files
            .push((id.to_owned(), "accounts.json".to_owned(), file));
    }

    start_device(&drive_url);
    GoogleAccounts::put_main(&account("main", "refresh-main"))?;
    GoogleAccounts::sync().await?;
    assert_eq!(
        emails(),
        ["main@example.com", "club@example.com", "work@example.com"]
    );

    // The first file now holds both.
    let first = drive.lock().expect("the test lock").files[0].2.to_string();
    assert!(first.contains("refresh-work") && first.contains("refresh-club"));

    GoogleAccounts::sign_out()?;
    GoogleAccounts::use_drive(None);
    GoogleAccess::swap_server(before);
    Ok(())
}

#[tokio::test]
#[serial(google_access_server)]
async fn app_data_follows_the_main_account_to_another_device() -> Result<()> {
    SessionStore::set_root(temp_dir().join("hilen-google-accounts-test"));
    SecretStore::keep_in_files();

    let before = GoogleAccess::swap_server(Some(backend().await?));
    let drive = Drive::default();
    let drive_url = fake_drive(drive.clone()).await?;

    // No main account, no folder to read.
    start_device(&drive_url);
    assert!(GoogleAccounts::app_data::<Vec<String>>("feeds.json").await.is_err());

    // Device 1 reads a file nobody saved, then saves it twice.
    GoogleAccounts::put_main(&account("main", "refresh-main"))?;
    assert_eq!(GoogleAccounts::app_data::<Vec<String>>("feeds.json").await?, None);
    GoogleAccounts::save_app_data("feeds.json", &["a"]).await?;
    GoogleAccounts::save_app_data("feeds.json", &["a", "b"]).await?;
    assert_eq!(
        drive.lock().expect("the test lock").content("feeds.json").len(),
        1
    );

    // The accounts file and the app file live side by side.
    GoogleAccounts::put_linked(&account("work", "refresh-work"), 100.0);
    GoogleAccounts::sync().await?;
    {
        let folder = drive.lock().expect("the test lock");
        assert_eq!(folder.files.len(), 2);
        assert!(folder.content("feeds.json")[0].contains("\"b\""));
        assert!(folder.content("accounts.json")[0].contains("refresh-work"));
    }

    // The app can not name the accounts file, nor a name Drive would read
    // as more of the query.
    assert!(GoogleAccounts::app_data::<Value>("accounts.json").await.is_err());
    assert!(GoogleAccounts::save_app_data("x' or name='y", &1).await.is_err());
    assert!(GoogleAccounts::app_data::<Value>("").await.is_err());

    // Device 2 has the same file after a sign in with the main account.
    start_device(&drive_url);
    GoogleAccounts::put_main(&account("main", "refresh-main"))?;
    assert_eq!(
        GoogleAccounts::app_data::<Vec<String>>("feeds.json").await?,
        Some(vec!["a".to_owned(), "b".to_owned()])
    );

    // A file that does not hold the asked type is an error without its
    // content.
    let error = GoogleAccounts::app_data::<u32>("feeds.json")
        .await
        .err()
        .map_or_default(|error| format!("{error:#}"));
    assert!(error.contains("not a u32") && !error.contains("\"b\""), "{error}");

    GoogleAccounts::sign_out()?;
    GoogleAccounts::use_drive(None);
    GoogleAccess::swap_server(before);
    Ok(())
}
