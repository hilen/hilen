//! The accounts of one user on 2 devices, against the real routes of
//! `hilen-server` and a stand in for the app folder of Google Drive.

use std::{
    env::temp_dir,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    routing::{get, patch},
};
use hilen_session::{SecretStore, SessionStore};
use serde_json::{Value, json};
use serial_test::serial;

use crate::google_access::{
    GoogleAccess, GoogleAccount, GoogleAccounts,
    client::test::{backend, serve_on_a_free_port},
};

/// The accounts files of the folder, id and content, and how often one was
/// written.
#[derive(Default)]
struct Folder {
    files:  Vec<(String, Value)>,
    writes: u32,
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

async fn list(State(drive): State<Drive>, headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
    allowed(&headers)?;
    let folder = drive.lock().expect("the test lock");
    let files: Vec<Value> = folder.files.iter().map(|(id, _)| json!({ "id": id })).collect();
    Ok(Json(json!({ "files": files })))
}

async fn create(State(drive): State<Drive>, headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
    allowed(&headers)?;
    let mut folder = drive.lock().expect("the test lock");
    let id = format!("file-{}", folder.files.len() + 1);
    folder.files.push((id.clone(), Value::Null));
    Ok(Json(json!({ "id": id })))
}

async fn read(
    State(drive): State<Drive>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    allowed(&headers)?;
    let folder = drive.lock().expect("the test lock");
    let file = folder.files.iter().find(|(have, _)| *have == id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(file.1.clone()))
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
        .find(|(have, _)| *have == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    file.1 = content;
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
    let stored = drive.lock().expect("the test lock").files[0].1.to_string();
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
            .1
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
        drive.lock().expect("the test lock").files.push((id.to_owned(), file));
    }

    start_device(&drive_url);
    GoogleAccounts::put_main(&account("main", "refresh-main"))?;
    GoogleAccounts::sync().await?;
    assert_eq!(
        emails(),
        ["main@example.com", "club@example.com", "work@example.com"]
    );

    // The first file now holds both.
    let first = drive.lock().expect("the test lock").files[0].1.to_string();
    assert!(first.contains("refresh-work") && first.contains("refresh-club"));

    GoogleAccounts::sign_out()?;
    GoogleAccounts::use_drive(None);
    GoogleAccess::swap_server(before);
    Ok(())
}
