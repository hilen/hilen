use axum::{
    Form, Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    AppError,
    auth::{
        AppleConfig, AuthState, User, apple, google,
        hand_back::HandBack,
        identity::{Identity, Provider},
        session::{self, hash},
        store,
        user::bearer_token,
        wire::{CodeRequest, CodeResponse, PollRequest, PollResponse, UserInfo},
    },
};

/// How long the user has to get through the pages of the provider.
const PENDING_MINUTES: i32 = 10;

/// The login routes, with their state already in, so the result merges into
/// an app router of any state:
///
/// - `GET /auth/google?challenge=` sends the browser on to Google. With
///   `&port=` the end of the login goes back to an app that listens on that
///   port of `127.0.0.1`, with `&page=1` to a page of this server, see
///   `hand_back.rs`.
/// - `GET /auth/google/callback` is where Google sends it back.
/// - `GET /auth/apple?challenge=` sends the browser on to Apple, 404 when the
///   config has no Apple part.
/// - `POST /auth/apple/callback` is the form Apple posts on the way back.
/// - `GET /auth/done` is the last page a person sees.
/// - `POST /auth/code` gives a device with no keyboard a short code for its
///   login, 404 unless [`AuthConfig::with_code_login`] turned it on.
/// - `GET /auth/code` is the page where a person types that code on a phone,
///   with `?code=` it sends the phone on to the login.
/// - `POST /auth/poll` hands the waiting app its session, once, for its
///   verifier and the code that went back to it.
///
/// [`AuthConfig::with_code_login`]: crate::auth::AuthConfig::with_code_login
/// - `GET /auth/me` is the user of a session.
/// - `POST /auth/logout` ends a session.
/// - `POST /auth/delete` removes the user of a session for good.
/// - `POST /auth/dev` logs into a dev account with no password, only in a build
///   with the `dev-login` feature, see `dev.rs`.
pub fn auth_routes<S>(state: AuthState) -> Router<S> {
    let router = Router::new()
        .route("/auth/google", get(start_google))
        .route("/auth/google/callback", get(google_callback))
        .route("/auth/apple", get(start_apple))
        .route("/auth/apple/callback", post(apple_callback))
        .route("/auth/done", get(done_page))
        .route("/auth/code", post(make_code).get(code_page))
        .route("/auth/poll", post(poll))
        .route("/auth/me", get(me))
        .route("/auth/logout", post(logout))
        .route("/auth/delete", post(delete));
    #[cfg(feature = "dev-login")]
    let router = crate::auth::dev::add_route(router);
    router.with_state(state)
}

#[derive(Deserialize)]
struct StartQuery {
    challenge: String,
    /// The port of `127.0.0.1` a native app listens on.
    port:      Option<u16>,
    /// Set for an app that is a page of this server.
    page:      Option<String>,
}

async fn start_google(
    State(state): State<AuthState>,
    Query(query): Query<StartQuery>,
) -> Result<Redirect, AppError> {
    let oauth_state = start(&state, &query).await?;
    Ok(Redirect::to(&google::auth_url(&state.config, &oauth_state)?))
}

async fn start_apple(
    State(state): State<AuthState>,
    Query(query): Query<StartQuery>,
) -> Result<Redirect, AppError> {
    let apple = apple_config(&state)?;
    let oauth_state = start(&state, &query).await?;
    Ok(Redirect::to(&apple::auth_url(
        &state.config,
        apple,
        &oauth_state,
    )?))
}

fn apple_config(state: &AuthState) -> Result<&AppleConfig, AppError> {
    state.config.apple.as_ref().ok_or(AppError::NotFound)
}

/// Remembers the login that starts and gives the `state` the provider has to
/// bring back.
async fn start(state: &AuthState, query: &StartQuery) -> Result<String, AppError> {
    let challenge = &query.challenge;
    if !is_challenge(challenge) {
        return Err(AppError::BadRequest(
            "the challenge is not a SHA-256 in hex".to_owned(),
        ));
    }

    store::prune_pending(&state.db, PENDING_MINUTES).await?;

    let hand_back = match HandBack::from_query(query.port, query.page.is_some())
        .map_err(|error| AppError::BadRequest(error.to_string()))?
    {
        Some(hand_back) => hand_back,
        None if state.config.code_login && store::has_code(&state.db, challenge, PENDING_MINUTES).await? => {
            HandBack::Code
        }
        None => {
            tracing::warn!("a login link with no way back to its app was refused");
            return Err(AppError::BadRequest(
                "this sign in link names no app to go back to, update the app and start again".to_owned(),
            ));
        }
    };

    let oauth_state = session::new_token()?;

    if !store::start_pending(&state.db, challenge, &oauth_state, &hand_back.text()).await? {
        return Err(AppError::BadRequest("this login is already finished".to_owned()));
    }
    tracing::debug!("login started, it goes back by {hand_back:?}");

    Ok(oauth_state)
}

/// The letters and digits of a short code, with none that look alike, no
/// `O` and `0`, no `I` and `1`. 32 of them, so a random byte maps evenly.
const CODE_ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const CODE_LENGTH: usize = 6;
/// A code that is taken is made again, 32^6 codes make that rare.
const CODE_TRIES: u32 = 5;

fn new_code() -> anyhow::Result<String> {
    let mut bytes = [0; CODE_LENGTH];
    getrandom::fill(&mut bytes).map_err(|error| anyhow::anyhow!("no random bytes for a code: {error}"))?;
    Ok(bytes
        .iter()
        .map(|byte| char::from(CODE_ALPHABET[usize::from(byte % 32)]))
        .collect())
}

/// What a person typed, as the code it means: `ab c-234` is `ABC234`.
fn typed_code(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// A TV cannot open a browser for its user. It asks for a short code, shows
/// it, and the user types it into `GET /auth/code` on a phone. The TV polls
/// with its verifier like after any other start.
async fn make_code(
    State(state): State<AuthState>,
    Json(request): Json<CodeRequest>,
) -> Result<Json<CodeResponse>, AppError> {
    if !state.config.code_login {
        tracing::warn!("a login code was asked for, AuthConfig::with_code_login turns the code login on");
        return Err(AppError::NotFound);
    }
    if !is_challenge(&request.challenge) {
        return Err(AppError::BadRequest(
            "the challenge is not a SHA-256 in hex".to_owned(),
        ));
    }
    let provider = match request.provider.as_str() {
        "google" => Provider::Google,
        "apple" => {
            apple_config(&state)?;
            Provider::Apple
        }
        _ => return Err(AppError::BadRequest("no such login provider".to_owned())),
    };

    store::prune_codes(&state.db, PENDING_MINUTES).await?;

    for _ in 0..CODE_TRIES {
        let code = new_code()?;
        if store::put_code(&state.db, &code, &request.challenge, provider.name()).await? {
            return Ok(Json(CodeResponse { code }));
        }
    }

    Err(anyhow::anyhow!("no free login code after {CODE_TRIES} tries").into())
}

#[derive(Deserialize)]
struct CodeQuery {
    code: Option<String>,
}

/// A person looks at this one. Without a code it is the form, with a known
/// code it sends the browser on to the login the code stands for.
async fn code_page(State(state): State<AuthState>, Query(query): Query<CodeQuery>) -> Response {
    if !state.config.code_login {
        return AppError::NotFound.into_response();
    }
    let app = &state.config.app_name;

    let Some(typed) = query.code else {
        return code_form(app, "");
    };

    match store::code_target(&state.db, &typed_code(&typed), PENDING_MINUTES).await {
        Ok(Some((challenge, provider))) => {
            Redirect::to(&format!("/auth/{provider}?challenge={challenge}")).into_response()
        }
        Ok(None) => code_form(
            app,
            "This code is not known or too old. Look at the screen again.",
        ),
        Err(error) => {
            tracing::error!("the login code lookup failed: {error:?}");
            code_form(app, "That did not work. Try again.")
        }
    }
}

#[derive(Deserialize)]
struct GoogleAnswer {
    code:  Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// A person looks at the answer of this one, so every outcome is a page.
async fn google_callback(State(state): State<AuthState>, Query(answer): Query<GoogleAnswer>) -> Response {
    let (Some(code), Some(oauth_state)) = (answer.code, answer.state) else {
        tracing::info!("google login stopped: {:?}", answer.error);
        return cancelled_page(&state);
    };

    let finished = finish(
        &state,
        &oauth_state,
        google::exchange(&state.http, &state.config, &code),
    )
    .await;
    outcome_page(&state, Provider::Google, finished)
}

#[derive(Deserialize)]
struct AppleAnswer {
    code:  Option<String>,
    state: Option<String>,
    /// The name as JSON, on the first login of an account only.
    user:  Option<String>,
    error: Option<String>,
}

/// A person looks at the answer of this one too.
async fn apple_callback(State(state): State<AuthState>, Form(answer): Form<AppleAnswer>) -> Response {
    let Ok(apple) = apple_config(&state) else {
        return AppError::NotFound.into_response();
    };
    let (Some(code), Some(oauth_state)) = (answer.code, answer.state) else {
        tracing::info!("apple login stopped: {:?}", answer.error);
        return cancelled_page(&state);
    };

    let finished = finish(
        &state,
        &oauth_state,
        apple::exchange(&state.http, &state.config, apple, &code, answer.user.as_deref()),
    )
    .await;
    outcome_page(&state, Provider::Apple, finished)
}

/// A login that has its user, with what goes back to its app.
struct Finished {
    hand_back: HandBack,
    challenge: String,
    /// The one time code the poll needs, `None` for `HandBack::Code`.
    code:      Option<String>,
}

/// Runs `exchange` and gives the waiting login its user. `None` when no login
/// waits for this `state`, it timed out or never was, and then the provider
/// is not asked at all.
async fn finish(
    state: &AuthState,
    oauth_state: &str,
    exchange: impl Future<Output = anyhow::Result<Identity>>,
) -> anyhow::Result<Option<Finished>> {
    let Some((challenge, hand_back)) = store::waiting(&state.db, oauth_state, PENDING_MINUTES).await? else {
        return Ok(None);
    };
    let hand_back = HandBack::parse(&hand_back)?;

    let identity = exchange.await?;
    let user_id = store::login_user(&state.db, &identity).await?;
    let code = match hand_back {
        HandBack::Code => None,
        HandBack::Loopback(_) | HandBack::Page => Some(session::new_token()?),
    };
    let code_hash = code.as_deref().map(hash);
    store::finish_pending(&state.db, oauth_state, user_id, code_hash.as_deref()).await?;
    tracing::info!(
        "{} login done for user {user_id}, it goes back by {hand_back:?}",
        identity.provider.name()
    );

    Ok(Some(Finished {
        hand_back,
        challenge,
        code,
    }))
}

fn cancelled_page(state: &AuthState) -> Response {
    page(
        &state.config.app_name,
        "Sign in was cancelled",
        "You can close this tab.",
    )
}

fn outcome_page(
    state: &AuthState,
    provider: Provider,
    finished: anyhow::Result<Option<Finished>>,
) -> Response {
    let app = &state.config.app_name;

    match finished {
        Ok(Some(finished)) => hand_back_page(app, &finished),
        Ok(None) => page(
            app,
            "This sign in link is too old",
            "Go back to the app and start again.",
        ),
        Err(error) => {
            tracing::error!("{} login failed: {error:?}", provider.name());
            page(app, "Sign in did not work", "Go back to the app and try again.")
        }
    }
}

/// Sends the code back to the app on the way it asked for.
fn hand_back_page(app: &str, finished: &Finished) -> Response {
    match (finished.hand_back, &finished.code) {
        (HandBack::Loopback(port), Some(code)) => {
            Redirect::to(&HandBack::loopback_url(port, code)).into_response()
        }
        (HandBack::Page, Some(code)) => {
            // The key and the code are hex, nothing in them needs escaping.
            let script = format!(
                r#"<script>try {{ localStorage.setItem("{}", "{code}"); }} catch (error) {{ document.querySelector("p").textContent = "This browser does not let the page hand the sign in over. Allow the site to store data and start again."; }}</script>"#,
                HandBack::storage_key(&finished.challenge)
            );
            page_with(
                app,
                &format!("You are signed in to {app}"),
                "You can close this tab and go back to the app.",
                &script,
            )
        }
        _ => done(app),
    }
}

/// Where a native app sends the browser after it took its code.
async fn done_page(State(state): State<AuthState>) -> Response {
    done(&state.config.app_name)
}

fn done(app: &str) -> Response {
    page(
        app,
        &format!("You are signed in to {app}"),
        "You can close this tab and go back to the app.",
    )
}

async fn poll(
    State(state): State<AuthState>,
    Json(request): Json<PollRequest>,
) -> Result<Json<PollResponse>, AppError> {
    let challenge = hex::encode(Sha256::digest(request.verifier.as_bytes()));
    let code_hash = request.code.as_deref().map(hash);

    let Some(user_id) =
        store::take_finished(&state.db, &challenge, code_hash.as_deref(), PENDING_MINUTES).await?
    else {
        return Ok(Json(PollResponse::Pending));
    };

    let token = session::create(&state.db, user_id).await?;
    let user = session::user_of(&state.db, &token).await?.ok_or(AppError::Unauthorized)?;

    Ok(Json(PollResponse::Done {
        token,
        user: user.into(),
    }))
}

async fn me(user: User) -> Json<UserInfo> {
    Json(user.into())
}

async fn logout(State(state): State<AuthState>, headers: HeaderMap) -> Result<(), AppError> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    session::delete(&state.db, token).await?;
    Ok(())
}

/// Removes the user for good. Apple is told first to forget what it gave out
/// for the user. A revoke that fails is logged and the user still goes, a
/// person must be able to leave also when Apple is down.
async fn delete(State(state): State<AuthState>, user: User) -> Result<(), AppError> {
    let tokens = store::apple_refresh_tokens(&state.db, user.id).await?;

    match &state.config.apple {
        Some(apple) => {
            for token in &tokens {
                if let Err(error) = apple::revoke(&state.http, apple, token).await {
                    tracing::error!("apple revoke failed for user {}: {error:?}", user.id);
                }
            }
        }
        None if !tokens.is_empty() => {
            tracing::error!(
                "user {} has an Apple token and the config has no Apple part, nothing revoked",
                user.id
            );
        }
        None => {}
    }

    store::delete_user(&state.db, user.id).await?;
    tracing::info!("user {} deleted", user.id);
    Ok(())
}

pub(crate) fn is_challenge(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn page(app: &str, title: &str, text: &str) -> Response {
    page_with(app, title, text, "")
}

/// `page` with a script of the server at the end of the body.
fn page_with(app: &str, title: &str, text: &str, script: &str) -> Response {
    let (app, title, text) = (escape(app), escape(title), escape(text));

    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{app}</title>
<style>
body {{ font-family: system-ui, sans-serif; background: #f7f8fa; color: #17191d; display: grid; place-items: center; min-height: 100vh; margin: 0; }}
main {{ text-align: center; padding: 24px; }}
h1 {{ font-size: 22px; font-weight: 600; margin: 0 0 8px; }}
p {{ color: #5f6368; margin: 0; }}
@media (prefers-color-scheme: dark) {{ body {{ background: #131314; color: #e3e3e3; }} p {{ color: #9aa0a6; }} }}
</style>
</head>
<body><main><h1>{title}</h1><p>{text}</p></main>{script}</body>
</html>"#
    ))
    .into_response()
}

/// The page with the field for the short code. `note` says why the last try
/// did not work.
fn code_form(app: &str, note: &str) -> Response {
    let (app, note) = (escape(app), escape(note));

    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{app}</title>
<style>
body {{ font-family: system-ui, sans-serif; background: #f7f8fa; color: #17191d; display: grid; place-items: center; min-height: 100vh; margin: 0; }}
main {{ text-align: center; padding: 24px; }}
h1 {{ font-size: 22px; font-weight: 600; margin: 0 0 8px; }}
p {{ color: #5f6368; margin: 0 0 16px; }}
input {{ font: inherit; font-size: 28px; letter-spacing: 6px; text-align: center; text-transform: uppercase; width: 220px; padding: 10px; border: 1px solid #c4c7cc; border-radius: 10px; background: transparent; color: inherit; }}
button {{ font: inherit; font-size: 18px; display: block; margin: 16px auto 0; padding: 10px 28px; border: 0; border-radius: 10px; background: #17191d; color: #f7f8fa; }}
.note {{ color: #c5221f; margin: 16px 0 0; }}
@media (prefers-color-scheme: dark) {{ body {{ background: #131314; color: #e3e3e3; }} p {{ color: #9aa0a6; }} input {{ border-color: #5f6368; }} button {{ background: #e3e3e3; color: #131314; }} .note {{ color: #f28b82; }} }}
</style>
</head>
<body><main>
<h1>Sign in to {app}</h1>
<p>Type the code from the screen.</p>
<form method="get" action="/auth/code">
<input name="code" maxlength="8" autocomplete="off" autocapitalize="characters" autofocus>
<button type="submit">Continue</button>
</form>
<p class="note">{note}</p>
</main></body>
</html>"#
    ))
    .into_response()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod test {
    use super::{CODE_ALPHABET, CODE_LENGTH, escape, is_challenge, new_code, typed_code};

    #[test]
    fn a_code_is_6_of_the_alphabet() -> anyhow::Result<()> {
        for _ in 0..100 {
            let code = new_code()?;
            assert_eq!(code.len(), CODE_LENGTH);
            assert!(code.bytes().all(|byte| CODE_ALPHABET.contains(&byte)));
        }
        Ok(())
    }

    #[test]
    fn a_typed_code_loses_case_and_separators() {
        assert_eq!(typed_code(" ab c-234 "), "ABC234");
        assert_eq!(typed_code("ABC234"), "ABC234");
    }

    #[test]
    fn challenge_shape() {
        assert!(is_challenge(&"a1".repeat(32)));
        assert!(!is_challenge(&"a1".repeat(31)));
        assert!(!is_challenge(&"zz".repeat(32)));
        assert!(!is_challenge(""));
    }

    #[test]
    fn page_text_is_escaped() {
        assert_eq!(
            escape(r#"<b>"A&B"</b>"#),
            "&lt;b&gt;&quot;A&amp;B&quot;&lt;/b&gt;"
        );
    }
}
