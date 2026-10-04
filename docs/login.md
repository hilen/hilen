# Login with Google and Apple

The client is `hilen/src/login` behind the `login` feature, the server is
`hilen-server/src/auth`. How an app uses them is in the hilen skill, `google-login.md`.
This file is about the inside.

## The flow

The app holds no provider secret and never talks to Google or Apple. It makes a random
verifier and opens `<server>/auth/google?challenge=<sha256 hex of the verifier>` in the
browser, or `/auth/apple` with the same query. The server stores the challenge in
`pending_logins` with a random `state`, sends the browser to the provider, and on the way
back trades the code for the id token and sets `user_id` on the row. The app polls `POST /auth/poll` with the verifier. The poll deletes the finished row
and returns a new session token, so the hand over happens once even with two polls at the
same moment. The browser history only ever holds the hash.

- The id token signature is not checked. The token comes straight from the provider over
  TLS in answer to the server's own request, nobody else had a hand on it.
- A challenge that already has its user is refused on `/auth/google`, or a second person
  with the same link could put their account under the app that waits.
- A row nobody finished is dropped after 10 minutes, the client gives up after the same time.
- Sessions keep only the SHA-256 of the token. All times are the database clock.

## A device with no keyboard

A TV cannot open a browser for its user. `Login::start_with_code` posts the challenge
to `POST /auth/code` and gets a short code, 6 letters and digits with none that look
alike. The app shows it, as text and as a `QrCodeView` of `<server>/auth/code?code=`.
The user opens that page on a phone, `GET /auth/code` is a form for the code, and with
a known code the server sends the phone on to `/auth/google?challenge=` of the TV. From
there the flow is the one above, the TV polls with its verifier.

- Codes live in `login_codes`, the third migration, for 10 minutes like a pending
  login. A code is not used up, a page loaded twice works twice.
- A person who guesses a code in time logs their own account into the waiting device,
  the same risk as a second person with the login link.
- `QrCodeView` draws 1 texture pixel per module with the nearest filter, black on white
  whatever the theme. `Qr code view test` pins it, the pixels have a unit test.

## Apple

Apple is optional. `AuthConfig::from_env` turns it on when `APPLE_SERVICE_ID` is set, then
`APPLE_TEAM_ID`, `APPLE_KEY_ID` and `APPLE_PRIVATE_KEY` are required. Without them the
Apple routes answer 404. What differs from Google, all in `apple.rs`:

- Apple has no fixed client secret. It is a JWT the server signs for each request with
  the `.p8` key, ES256, through `ring`, which rustls already brings.
- With the `name email` scope Apple answers only by a form POST, so
  `/auth/apple/callback` is a POST. The login keeps no cookie, the cross site post is fine.
- The name comes in the `user` field of that form, on the first login of an account only.
  A later login with no name keeps the stored one. A picture never comes.
- The exchange gives a refresh token, kept in `identities.refresh_token` as plain text. It
  opens nothing without the private key. `POST /auth/delete` revokes it at Apple and then
  removes the user. A revoke that fails is logged and the user still goes.

Apple refuses `localhost` as a return address. `routes_test.rs` runs the whole way, page,
form post, exchange, poll, delete and revoke, against a small server that stands in for
Apple. The real Apple was never asked, see the leftovers in [roadmap.md](roadmap.md).

## Identities

Who a user is at a provider lives in `identities`, one row per provider and subject, so a
user can have a Google and an Apple row. `store::login_user` runs a login: a known
identity logs into its user, a new identity whose email already belongs to a user joins
that user, anything else makes a new user. Both providers hand out verified emails only.
An Apple account with a hidden email has a relay address and becomes a user of its own.

The second migration moves `users.google_sub` into `identities`. SQLite cannot drop a
UNIQUE column, so its file builds `users` again with foreign keys off and runs outside a
transaction, the `-- no-transaction` first line. With foreign keys on the drop of the old
table would delete every row that points at a user, in the app tables too.
`a_database_of_the_first_migration_moves_over` pins that on both databases.

## The database

The login runs on Postgres and on SQLite. `hilen_server::Db` is either pool,
`build_db` makes the Postgres one and `build_sqlite` the SQLite one, a file
with foreign keys on and a WAL journal, so a backend can ship as 1 binary with
1 data file. `AuthState::new` and `auth::migrate` take either pool. The `User`
extractor needs a `Db` from the router state: a state that is a `PgPool` or a
`SqlitePool` gives one, a state struct needs a `Db` field. Every query is in
`hilen-server/src/auth/store.rs`, once per database. SQLite keeps ids as 16
byte blobs the server makes and times as unix seconds, its schema is in
`migrations_sqlite`. `store_test.rs` runs the whole login on a SQLite file, and on a Postgres through tests that are ignored
until `HILEN_TEST_POSTGRES_URL` names a server, the top of the file has the commands.
`Config::redis_url` is none when `REDIS_URL` is not set.

## The wire

`hilen-server` never links `hilen`, so `wire.rs` exists twice, `hilen/src/login/wire.rs`
and `hilen-server/src/auth/wire.rs`. Both have a test that pins the JSON text. A change in
one needs the same change in the other.

## The session key

`SessionStore` lives in its own small crate, `hilen-session`, so a tool with no window,
like the headless `blackforge play`, reads the same login without the engine. `hilen`
re-exports it as `hilen::store::SessionStore` with the `login` feature, and
`OnDisk::set_root_path` also sets the folder of its file, `SessionStore::set_root`. A tool
with no engine calls `set_root` itself with the same folder.

`SessionStore` seals the token with AES-256-GCM, a fresh nonce per save in front of the
bytes. The key is SHA-256 over a context text, the built in key and the machine id.

`hilen-session/build.rs` reads `HILEN_SESSION_KEY` and writes
`session_key.rs` into `OUT_DIR`: the key xored with a random mask, the mask, and the shift
between the two. `hilen-session/src/session_key.rs` puts it back together behind `black_box`, without
it the optimizer folds the constants back into the plain key. Checked once by hand: an
optimized build with a known key does not hold that text.

No key means the public development key and a cargo warning. `HILEN_RELEASE` set means a
missing key fails the build. `build/release/with-secrets.sh` sets the mark, since every
shipped build runs through it, and the release scripts set it again on their own. The same
mark blocks the `inspect` feature, see [inspect.md](inspect.md). The cargo release profile cannot be the mark, `make run`
of an app builds with `--release` and must work without Infisical.

The build script of `hilen-session` prints `rerun-if-env-changed` for both names. A build
without the key after one with it rebuilds the crate, so an app run like `make run`
without Infisical takes the development key again.

## The buttons and their tests

`GoogleLoginButton` and `AppleLoginButton` are one body in `login_button.rs`, written
out twice by a macro with a `Look` each. A generic view would not do, it cannot sit in a
UI test. The Apple button is black on a light screen and white on a dark one, and swaps
its logo in `theme_changed`.

The 3 texts of a button are English by default. An app in another language sets them
with `set_sign_in_text`, `set_waiting_text` and `set_cancel_text`. The cancel button is
64 points wide whatever its text, and the waiting text has to end before it.
`Login button texts` pins both buttons with Russian texts.

A login button turns its own touch on before it wires the cancel button. The touch
view that signed up last is asked first and `Button::on_tap` is what signs a button up,
the other way round the view takes every tap meant for cancel.

`Google login button look` and `Apple login button look` must never open a browser, also not from inside `demo` on a
device, where a server may be set. Each swaps the server out for its tap, so the login
fails before `open_url`, and shows the waiting look through `show_waiting_frozen`,
which holds the spinner still, a turning ring has no pixels to pin. Both seams exist only
with `ui-tests`.
