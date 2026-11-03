//! The seed of a dev server against a real SQLite file.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Result;

use crate::{
    auth::{
        dev_test::{dev_app, google, logged_in},
        store,
    },
    seed::{dev_users, run_if},
};

#[tokio::test]
async fn a_seeded_account_signs_in_with_the_dev_login() -> Result<()> {
    let (app, db) = dev_app().await?;
    let ids = dev_users(db.clone(), ["ana@test".to_owned(), " Bob@Test ".to_owned()]).await?;
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);

    let bob = logged_in(&app, "bob@test").await?;
    assert_eq!(
        bob.user.id,
        ids[1].to_string(),
        "the dev login made a second account"
    );

    // A second seed gives the same accounts back.
    let again = dev_users(db.clone(), ["ana@test".to_owned()]).await?;
    assert_eq!(again, vec![ids[0]]);
    Ok(())
}

#[tokio::test]
async fn a_start_that_was_not_asked_seeds_nothing() -> Result<()> {
    let (_app, db) = dev_app().await?;
    let ran = Arc::new(AtomicBool::new(false));
    let seen = ran.clone();
    run_if(false, db.clone(), || async move {
        seen.store(true, Ordering::SeqCst);
        Ok(())
    })
    .await?;
    assert!(!ran.load(Ordering::SeqCst));
    Ok(())
}

#[tokio::test]
async fn a_seed_runs_on_a_database_of_dev_accounts() -> Result<()> {
    let (_app, db) = dev_app().await?;
    dev_users(db.clone(), ["ana@test".to_owned()]).await?;
    let ran = Arc::new(AtomicBool::new(false));
    let seen = ran.clone();
    run_if(true, db.clone(), || async move {
        seen.store(true, Ordering::SeqCst);
        Ok(())
    })
    .await?;
    assert!(ran.load(Ordering::SeqCst));
    Ok(())
}

#[tokio::test]
async fn a_seed_refuses_a_database_with_a_real_account() -> Result<()> {
    let (_app, db) = dev_app().await?;
    store::login_user(&db, &google("ana@example.com")).await?;
    let ran = Arc::new(AtomicBool::new(false));
    let seen = ran.clone();
    let result = run_if(true, db.clone(), || async move {
        seen.store(true, Ordering::SeqCst);
        Ok(())
    })
    .await;
    assert!(
        result.is_err(),
        "the seed went into a database with a Google account"
    );
    assert!(!ran.load(Ordering::SeqCst));
    Ok(())
}
