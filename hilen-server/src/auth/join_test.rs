//! A login through a second provider: refused when the app turned the join
//! by email off, and added to its user when that user asked for the link
//! while signed in. On a SQLite file and, when one is at hand, on a
//! Postgres, see the top of `store_test.rs`.

use anyhow::{Context, Result};

use crate::{
    Db,
    auth::{
        identity::Provider,
        store::{self, LoginRules},
        store_test::{count, discord_identity, identity, on_postgres, sqlite},
    },
};

// The failure this prevents: whoever took over the Discord account of a
// user got into the user of the app too, though that user only ever signed
// in with Google.
async fn with_no_join_a_new_provider_does_not_open_the_user_of_its_email(db: Db) -> Result<()> {
    let ana = store::login_user(&db, &identity("Ana")).await?;
    let rules = LoginRules {
        limits:         &[],
        joins_by_email: false,
    };

    let refused = store::login_user_limited(
        &db,
        &discord_identity("discord-1", "Ana@Example.com"),
        rules,
        None,
    )
    .await;
    let error = refused.expect_err("Discord must not join the user of Google");
    let refused = error.downcast_ref::<store::LoginRefused>().context("a refused login")?;
    assert_eq!(refused.provider, Provider::Discord);
    assert_eq!(
        refused.allowed,
        vec![Provider::Google],
        "the page names the way in"
    );
    assert_eq!(
        count(&db, "SELECT count(*) FROM identities").await?,
        1,
        "nothing of the refused login is saved"
    );

    assert_eq!(
        store::login_user_limited(&db, &identity("Ana"), rules, None).await?,
        ana,
        "Google still works"
    );
    let bob = store::login_user_limited(
        &db,
        &discord_identity("discord-2", "bob@example.com"),
        rules,
        None,
    )
    .await?;
    assert_ne!(bob, ana, "an email with no user makes a new one");
    assert_eq!(
        store::login_user_limited(
            &db,
            &discord_identity("discord-2", "bob@example.com"),
            rules,
            None
        )
        .await?,
        bob,
        "a known identity signs in as before"
    );
    Ok(())
}

// Users joined before the rule keep both ways in.
async fn with_no_join_a_user_joined_before_keeps_both_providers(db: Db) -> Result<()> {
    let ana = store::login_user(&db, &identity("Ana")).await?;
    let discord = discord_identity("discord-1", "ana@example.com");
    assert_eq!(
        store::login_user(&db, &discord).await?,
        ana,
        "joined by the email"
    );

    let rules = LoginRules {
        limits:         &[],
        joins_by_email: false,
    };
    assert_eq!(store::login_user_limited(&db, &discord, rules, None).await?, ana);
    assert_eq!(
        store::login_user_limited(&db, &identity("Ana"), rules, None).await?,
        ana
    );
    Ok(())
}

// A signed in user adds Discord to an account of Google. The link lands only
// in the user who asked, and only with the same email.
async fn a_link_adds_a_provider_to_the_user_who_asked(db: Db) -> Result<()> {
    let ana = store::login_user(&db, &identity("Ana")).await?;
    let bob = store::login_user(&db, &discord_identity("discord-bob", "bob@example.com")).await?;
    let rules = LoginRules {
        limits:         &[],
        joins_by_email: false,
    };

    assert!(store::link_pending(&db, "challenge", "placeholder", ana).await?);
    assert_eq!(
        store::waiting(&db, "placeholder", 10).await?,
        None,
        "a link with no start cannot finish"
    );
    assert!(store::start_pending(&db, "challenge", "state", "page").await?);
    assert_eq!(
        store::waiting(&db, "state", 10).await?,
        Some(("challenge".to_owned(), "page".to_owned(), Some(ana))),
        "the start keeps the link"
    );

    let other_email = discord_identity("discord-x", "other@example.com");
    let refused = store::login_user_limited(&db, &other_email, rules, Some(ana)).await;
    let error = refused.expect_err("another email is no link");
    let refused = error.downcast_ref::<store::LoginRefused>().context("a refused login")?;
    assert_eq!(refused.reason, store::Refusal::OtherEmail);

    let of_bob = discord_identity("discord-bob", "bob@example.com");
    let refused = store::login_user_limited(&db, &of_bob, rules, Some(ana)).await;
    let error = refused.expect_err("the account of bob is no link for ana");
    let refused = error.downcast_ref::<store::LoginRefused>().context("a refused login")?;
    assert_eq!(refused.reason, store::Refusal::OtherUser);
    assert_eq!(
        store::providers_of(&db, ana).await?,
        ["google"],
        "nothing was added"
    );

    let hers = discord_identity("discord-ana", "Ana@Example.com");
    assert_eq!(
        store::login_user_limited(&db, &hers, rules, Some(ana)).await?,
        ana,
        "the same email links"
    );
    assert_eq!(store::providers_of(&db, ana).await?, ["discord", "google"]);
    assert_eq!(
        store::login_user_limited(&db, &hers, rules, None).await?,
        ana,
        "and signs in from then on"
    );
    assert_eq!(store::providers_of(&db, bob).await?, ["discord"]);
    Ok(())
}

#[tokio::test]
async fn sqlite_with_no_join_a_new_provider_does_not_open_the_user_of_its_email() -> Result<()> {
    with_no_join_a_new_provider_does_not_open_the_user_of_its_email(sqlite().await?.1).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of store_test.rs"]
async fn postgres_with_no_join_a_new_provider_does_not_open_the_user_of_its_email() -> Result<()> {
    on_postgres(with_no_join_a_new_provider_does_not_open_the_user_of_its_email).await
}

#[tokio::test]
async fn sqlite_with_no_join_a_user_joined_before_keeps_both_providers() -> Result<()> {
    with_no_join_a_user_joined_before_keeps_both_providers(sqlite().await?.1).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of store_test.rs"]
async fn postgres_with_no_join_a_user_joined_before_keeps_both_providers() -> Result<()> {
    on_postgres(with_no_join_a_user_joined_before_keeps_both_providers).await
}

#[tokio::test]
async fn sqlite_a_link_adds_a_provider_to_the_user_who_asked() -> Result<()> {
    a_link_adds_a_provider_to_the_user_who_asked(sqlite().await?.1).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of store_test.rs"]
async fn postgres_a_link_adds_a_provider_to_the_user_who_asked() -> Result<()> {
    on_postgres(a_link_adds_a_provider_to_the_user_who_asked).await
}
