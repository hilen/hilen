# Google access

An app that reads or writes a Google API, like a calendar, needs tokens with
scopes, for more than 1 Google account, on every platform. This is that part. It
is not the login of [login.md](login.md): there the server knows the user and the
app holds a session. Here the app holds the Google tokens and the server stores
none of them.

The client half is `hilen::google_access` behind the cargo feature
`google-access`. The server half is `hilen_server::google_access`. The two only
share the wire, `wire.rs` exists in both crates, and a change in one needs the
same change in the other.

## Why a server at all

Google ties a refresh token to the client id that made it, and it wants a client
id per platform. A token made on a Mac then does not work on a phone. So the app
has 1 Google client of the type Web application, and that client lives on the
backend with its secret. Every platform signs in through the browser against the
backend, and every refresh token works on every device.

## The flow

1. The app makes a random verifier and an X25519 key pair, both for this one
   sign in. It opens `<server>/google/start` with the SHA-256 of the verifier,
   the public key and the role, `main` or `linked`.
2. The server sends the browser to Google with the scopes of that role, then
   trades the code Google sends back for tokens.
3. The server seals the tokens for the public key at once and keeps only the
   sealed text, until the app picks it up. The app polls `/google/poll` with the
   verifier and opens the text with its private key.
4. To renew an access token the app posts its refresh token to
   `/google/refresh`. The server adds the client secret, asks Google, answers
   and forgets both tokens. A 401 means Google no longer takes the refresh
   token, the account signs in again.
5. `/google/revoke` ends a refresh token at Google.

A sealed text is base64url of `server public key, 32 bytes | nonce, 12 bytes |
ciphertext and tag`. The message key is HKDF-SHA256 over the X25519 secret, with
both public keys as the salt. The cipher is AES-256-GCM with the challenge as
additional data. The server half is `seal.rs` on `ring`, the device half is
`DeviceKey` in `hilen-session` on pure Rust crates, so it also builds for a
browser. The server test opens with the real device code.

What this gives: a copy of the server database, its disk and its secrets holds
no token. What it does not give: a person who controls the running server can
read a token while it passes through a refresh.

## The server

```rust
google_access::migrate(&db).await?;
let config = GoogleAccessConfig::from_env("My App")?
    .main_scopes([CALENDAR, APP_DATA])
    .linked_scopes([CALENDAR]);
let app = base_routes("my-app").merge(google_access_routes(GoogleAccessState::new(&db, config)));
```

- `from_env` reads `PUBLIC_ORIGIN`, `GOOGLE_CLIENT_ID` and
  `GOOGLE_CLIENT_SECRET`. The redirect to allow at Google is
  `<PUBLIC_ORIGIN>/google/callback`.
- `openid email profile` is always asked. A sign in where the user unticked a
  permission fails.
- 2 tables, on Postgres or SQLite like the login. `google_pending` holds the
  sign ins on their way. `google_users` has 1 row per main account for metrics:
  email, name, platform, app version, count of linked accounts, first and last
  seen. A linked account never gets a row. A refresh updates the row only when
  Google names the account in its answer and that account has a row.
- Google ends every token of an account for this app when 1 of them is revoked,
  on every device. So a sign out must not revoke.
- While the Cloud project is in testing status, Google ends a refresh token
  with a scope beyond the basic profile after 7 days.

## The client

`GoogleAccess` is the plain calls: `set_server` at launch, `sign_in(role, done)`
from a tap, `cancel`, `renew` and `revoke`.

`GoogleAccounts` on top of it is 1 main account with linked accounts:

- The refresh token of the main account is in a `SecretStore` named
  `google-main`.
- The linked accounts are 1 JSON file, `accounts.json`, in the app data folder
  of the Google Drive of the main account. The main account needs the scope
  `https://www.googleapis.com/auth/drive.appdata` for it. A sign in with the
  main account on another device reads the file and has every account.
- The main account is read from the `SecretStore` once per run and kept in
  memory. A read of the Keychain can make macOS ask the user, so it must not
  happen per call.
- On a device the linked accounts are in memory only. `sync` reads the file,
  merges it with what the device knows and writes it back when the file was
  behind. Call it at launch when `main()` is some.
- The file is merged, never just replaced, see `vault.rs`. Per account the entry
  that changed last wins. A removed account stays as an entry with no token, or
  an old copy of the list on another device would bring it back. 2 devices that
  make the file at the same moment leave 2 files, every sync reads both and
  writes the first.
- `access_token(subject)` gives a token that is good for at least a minute.
  `fresh_access_token` asks a new one after a 401 from Google.
- `RenewError::SignInAgain` means `sign_in` again for the main account and
  `link` again for a linked one. `sync` keeps this error type inside its
  `anyhow` error.
- No error and no `Debug` print carries a token. A parse of a body with tokens
  never goes into an error text.

## The buttons

`GoogleSignInButton` signs the main account in and `GoogleLinkButton` links one
more, both in the look of the Google button guidelines, with the spinner and the
cancel of the login buttons. Their events are `signed_in` and `linked` with all
accounts, and `failed` with a message. The body is shared with
`GoogleLoginButton` and `AppleLoginButton`: `sign_in_button.rs` holds the looks
and a macro that writes the view out per button, with the fn that starts the sign
in and the fn that cancels it. A generic view could not sit in a UI test. The UI
test is `Google account buttons look`.

## SecretStore

`hilen_session::SecretStore`, re-exported as `hilen::store::SecretStore`, keeps
1 named secret. With the `keychain` feature of `hilen-session`, which
`google-access` turns on, it is in the credential store of the system: the
Keychain on a Mac through the login keychain, the protected store on iOS, the
Credential Manager on Windows, the Secret Service on Linux, the Keystore on
Android. They come from the `keyring-core` crates. In a browser, without the
feature, and on a Linux with no Secret Service it is a file sealed like the
session, with a warning in the log for the last case.

- The engine files the secrets under the name of the data folder, the project
  name on desktop.
- `SecretStore::keep_in_files()` is for tests, a test must not ask the real
  credential store. 1 ignored test does, see `secret_store.rs`.
- Windows takes about 2500 bytes per secret.
- macOS lets the program that made a Keychain entry read it with no dialog, and
  it knows a program by its code signature. A release signed with a Developer ID
  keeps its signature over updates and never asks. A debug build is a new
  program after every rebuild and would ask for the Keychain password each time,
  so the engine calls `keep_in_files()` in a debug build of an app with
  `google-access`. A debug and a release build so do not share a sign in.

## Tests

- `hilen-server`: `google_access::test` runs the whole flow against a stand in
  for Google, on SQLite and, with `HILEN_TEST_POSTGRES_URL`, on Postgres. It
  proves that no token is readable in any column.
- `hilen`: `client::test` runs the real client against the real routes, with
  `hilen-server` as a dev dependency. `accounts_test` plays 2 devices against
  a stand in for the Drive folder.
- No test asks the real Google. Run them with
  `cargo test -p hilen --features google-access google_access`.
