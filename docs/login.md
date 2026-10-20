# Login with Google and Apple

The client is `hilen/src/login` behind the `login` feature, the server is
`hilen-server/src/auth`. How an app uses them is in the hilen skill, `google-login.md`.
This file is about the inside.

## The flow

The app holds no provider secret and never talks to Google or Apple. It makes a random
verifier and opens `<server>/auth/google?challenge=<sha256 hex of the verifier>&port=<port>`
in the browser, or `/auth/apple` with the same query, a page in a browser sends `page=1`
in place of the port. The server stores the challenge and the way back in
`pending_logins` with a random `state`, sends the browser to the provider, and on the way
back trades the code for the id token and sets `user_id` on the row. Then it makes a one
time code, keeps its SHA-256 on the row, and gives the code back to the app, see below.
The app sends `POST /auth/poll` with the verifier and the code. The poll deletes the
finished row and returns a new session token, so the hand over happens once even with two
polls at the same moment. The browser history only ever holds the hash and the code.

## The way back

Whoever made a login link holds its verifier. If the verifier alone opened the session,
a person could make a link with a verifier of their own, get somebody else to log in
with it, poll, and hold a 90 day session of that somebody. A code shown in the app does
not stop this, a code can be phished too. So the end of the login goes back to the app
instance that started it, on a way that only a program on the device of the browser can
read, and the poll needs the code that came that way. The server half is
`hilen-server/src/auth/hand_back.rs`, the client half `hilen/src/login/hand_back.rs`.

- A native app, on desktop, iOS and Android, listens on a free port of `127.0.0.1`
  while the login runs and sends `port=` in the link. The last step of the login sends
  the browser to `http://127.0.0.1:<port>/?code=<code>`. The app takes the code and
  sends the browser on to `GET /auth/done`, the page that says the login is done. This
  is the loopback way of RFC 8252. Only a GET of the root with 64 hex digits counts, a
  request some web page sends to the port changes nothing, and a code the server does
  not take does not end the wait.
- A page in a browser sends `page=1`. The last page of the login writes the code into
  `localStorage` of the origin of the server under `hilen-login-<challenge>`, and the
  page of the app reads it there. So the page and the server need 1 origin, the page
  served by `web_mount` of the same backend. `Login::start` gives an error on a page of
  another origin.
- The person who logged in holds the code, the one who made the link does not. A code
  goes with its own challenge only, it opens no other login.
- A link with no way back is refused with a 400 before the provider is asked. An app of
  an engine before the way back cannot log in against a new server, app and backend
  update together.
- `a_login_started_by_another_client_never_gives_it_a_session` in `routes_test.rs`
  runs the attack on each way back. `the_login_polls_with_the_code_that_came_back` in
  `login/client_test.rs` runs the client half against a stand in server.
- Not proven on a device yet: the loopback page in the browser sheet of an iPhone and
  in the browser of an Android phone. Android can freeze an app in the background, the
  browser then waits on the port until the user switches back to the app.

- The id token signature is not checked. The token comes straight from the provider over
  TLS in answer to the server's own request, nobody else had a hand on it.
- A challenge that already has its user is refused on `/auth/google`, or a second person
  with the same link could put their account under the app that waits.
- A row nobody finished is dropped after 10 minutes, the client gives up after the same time.
- Sessions keep only the SHA-256 of the token. All times are the database clock.

## The browser sheet on iOS

On iOS the login page opens in an `SFSafariViewController` sheet over the app, not in
Safari, so the user never leaves the app. The code is `hilen/src/system/browser_sheet.rs`
with the native half in `hilen/native/ios/hilen_browser.m`. Every other platform, tvOS
too, keeps the plain `open_url`.

- The sheet closes by itself when the poll says the login is done, and when the wait
  ends for any other reason, a cancel or the 10 minutes.
- A user who closes the sheet by hand, with Done or a swipe, gets 1 more look for the
  code at once. A login whose code already came back still logs in. A pending one ends
  the waiting state of the button with no event, the same as its cancel.
- The app stays in front under the sheet, so the poll keeps running. With Safari the
  system could suspend the app.
- `SFSafariViewController` was picked over `ASWebAuthenticationSession` because it
  needs no url scheme, the code comes back to the loopback port. Its cost: the sheet
  has its own cookies, so a user who is logged in to Google in Safari may type the
  password again.
- Google refuses a login in a plain `WKWebView`, never move the page into one.
- Not covered by a UI test. The sheet is a system view with no pixels in the engine,
  and the button tests never reach the open call. A real login is checked by hand on a
  phone.
- `GoogleAccess::sign_in` in `hilen/src/google_access` still opens Safari on iOS.

## Requests

The login client sends everything through `net::Call`, the one request path of the
engine, see `hilen/src/deps/netrun/rest/call.rs`. A 401 comes back as
`RequestError::Unauthorized`, which is how `current_user` learns that the server
forgot the session.

`Login::send(call)` is the same path for the app's own routes behind the login. It adds
the token of the active session as `Authorization: Bearer`. With no active session it
sends nothing and gives `Unauthorized`. It never clears the session, the URL may be
another server. `Call` does not log a request body, the poll carries the verifier in it.

## Several logins on 1 device

Several people can be logged in on 1 device at once, like the account switch of
Instagram, and 1 of them is active. Flixen uses it for its "Who is watching" screen on a
shared TV. Each login is a real session of its own on the server, the server needs
nothing new.

- `SessionStore` keeps a list of sessions as JSON, in the credential store or in the
  sealed file, see "Where the sessions are kept": each `Session` has its token and its
  `LoginUser`, and an index names the active one. `LoginUser` lives in `hilen-session`
  for that, `hilen::login` re-exports it.
- A login by `start` or `start_with_code` is added with `SessionStore::add` and becomes
  active. A login of a user who is in the list already gets the new token in the old
  place.
- A device that logs in another way, like with a pairing code over a connection its
  server opened, gets its token from `hilen_server::auth::create_session(db, user_id)`
  on the server. The app keeps it with `SessionStore::add(token, user)`, and from then on
  it is a session like any other.
- `load`, `token()`, `send` and `current_user` work on the active session, so an app
  with 1 login needs no change. `clear` and `logout` drop only the active session, and
  none of the others becomes active by itself, an app shows its profiles then.
  `current_user` drops only the active session on a 401, and puts a new name or picture
  from `/auth/me` into the list.
- `Login::accounts()` lists the users, `Login::active_user()` is the active one with no
  request, `Login::switch(id)` makes one active, and `Login::forget(id)` ends that
  session on the server and drops it here.
- A file of an engine with 1 login holds the bare sealed token. It loads as the only
  session, active, with no user, and the first `current_user` fills the user in. So an
  update logs nobody out. Seen on a Mac and an LG TV on 2026-10-15.
- An older engine that reads a file of this one takes the JSON as a token. The server
  answers 401 and the user logs in again.
- Every change reads the list and writes it back under 1 lock, and every load waits
  for a change that runs. The last session to go removes the entry or the file.
- `cargo test -p hilen-session` covers the list, the old file and the twin of a user
  whose old session had no user yet. The client tests in `login/client_test.rs` run
  `current_user`, `forget` and `logout` against a small stand in server.

## A device with no keyboard

A TV cannot open a browser for its user. `Login::start_with_code` posts the challenge
to `POST /auth/code` and gets a short code, 6 letters and digits with none that look
alike. The server answers 404 unless `AuthConfig::with_code_login()` turned the code
login on. The app shows it, as text and as a `QrCodeView` of `<server>/auth/code?code=`.
The user opens that page on a phone, `GET /auth/code` is a form for the code, and with
a known code the server sends the phone on to `/auth/google?challenge=` of the TV. From
there the flow is the one above, the TV polls with its verifier.

- Codes live in `login_codes`, the third migration, for 10 minutes like a pending
  login. A code is not used up, a page loaded twice works twice.
- Nothing can go back from the phone to the TV, so this login has no way back: the TV
  polls with the verifier alone. A start link with no way back is taken only for a
  challenge that a live code leads to, the row gets the way back `code`. The cost is
  the attack of "The way back": a person who sends the code of their own TV to
  somebody else gets the session of that somebody when they log in with it. That is
  why the code login is off by default, turn it on only for an app that runs on a TV.
- A person who guesses a code in time logs their own account into the waiting device.
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
A user with a `dev` identity is left out of that email join, see the next section.

## The dev login

A dev server, a backend for testing an app with no Google account, logs in with no
password. It exists only in a server built with the `dev-login` cargo feature of
`hilen-server`, the code is `hilen-server/src/auth/dev.rs`. A build without the feature
has no `/auth/dev` route at all, so no config line can turn it on for a server with real
users. `the_dev_login_is_not_there_without_the_feature` pins that, and the server logs a
warning at start when it is built with the feature.

- `POST /auth/dev` with `{"email": "..."}` answers like a finished poll, the token and
  the user. The email is trimmed and lowercased.
- The first login of an email makes a new user with a `dev` identity, its subject is the
  email. The next login of that email logs back into it.
- A dev login never joins a user of the same email, `Provider::joins_by_email`. And a
  Google or Apple login never joins a user that has a `dev` identity. So the route
  reaches only the accounts it made itself. `dev_test.rs` pins both ways.

The client half is `Login::dev(email, done)`, only in a build with the `inspect`
feature. `hilen-inspect dev-login <email>` calls it in a running app. The session is
kept like after any login, and the app sees it at its next start.

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

## Where the sessions are kept

`SessionStore` lives in its own small crate, `hilen-session`, so a tool with no window,
like the headless `blackforge play`, reads the same login without the engine. `hilen`
re-exports it as `hilen::store::SessionStore` with the `login` feature, and
`OnDisk::set_root_path` also sets the folder of its file, `SessionStore::set_root`. A tool
with no engine calls `set_root` itself with the same folder.

With the `keychain` feature of `hilen-session`, which the `secrets` feature of `hilen`
turns on, the list lives in the credential store of a phone through `SecretStore`,
under the name `session`, the Keychain on iOS and tvOS and the Keystore on Android. On
iOS and tvOS the entry is `AfterFirstUnlockThisDeviceOnly`, so a backup that goes to
another phone holds no session. The rest of this section is the sealed file, which stays
for desktop, the browser and a build without the feature.

A desktop never asks the credential store of its system, `FILES_BY_DEFAULT` in
`secret_store.rs`. A Mac ties the right to read a Keychain entry to the signature of the
binary. An app built by hand is signed only ad hoc, so each build is a new program to
macOS and asked for the password of the login keychain at its first start. That ran from
`b94ccc55` on 2026-10-19 until the next day. A login that went into the Keychain then is
not read back, the user logs in once more.

- A sealed file of an engine before the credential store moves in at the first load and
  is deleted, so an update logs nobody out. A bare token of an engine with 1 login moves
  the same way. When the store does not take the list, the file stays and is read again
  next time.
- Every write into the store also deletes a file that is still there, so an old list
  never comes back once the store is empty again.
- A test never asks the store of its machine, `hilen-session` builds its tests with a
  mock store in memory. `a_sealed_file_of_today_moves_into_the_credential_store`,
  `a_bare_token_of_an_old_engine_moves_into_the_credential_store` and
  `a_desktop_never_asks_the_credential_store` run with
  `cargo test -p hilen-session --features keychain`. A test of the engine that touches
  `SessionStore` calls `SecretStore::keep_in_files`, a workspace build has the feature
  on through `ui-test`.
- Not proven on a device yet: the entry on a real iPhone.

## The session key

`SessionStore` seals the token with AES-256-GCM, a fresh nonce per save in front of the
bytes. The key is SHA-256 over a context text, the built in key and the machine id.

`hilen-session/build.rs` reads `HILEN_SESSION_KEY` and writes
`session_key.rs` into `OUT_DIR`: the key xored with a random mask, the mask, and the shift
between the two. `hilen-session/src/session_key.rs` puts it back together behind `black_box`, without
it the optimizer folds the constants back into the plain key. Checked once by hand: an
optimized build with a known key does not hold that text.

No key means the public development key and a cargo warning. Such a build also writes a
warning into the log of the app, 1 time per start and not on every use of the key, an
app asks for the key on every request to its backend. `HILEN_RELEASE` set means a
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
