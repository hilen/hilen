# Google login

The client is `hilen/src/login` behind the `login` feature, the server is
`hilen-server/src/auth`. How an app uses them is in the hilen skill, `google-login.md`.
This file is about the inside.

## The flow

The app holds no Google secret and never talks to Google. It makes a random verifier and
opens `<server>/auth/google?challenge=<sha256 hex of the verifier>` in the browser. The
server stores the challenge in `pending_logins` with a random `state`, sends the browser
to Google, and on the way back trades the code for the id token and sets `user_id` on the
row. The app polls `POST /auth/poll` with the verifier. The poll deletes the finished row
and returns a new session token, so the hand over happens once even with two polls at the
same moment. The browser history only ever holds the hash.

- The id token signature is not checked. The token comes straight from Google over TLS in
  answer to the server's own request, nobody else had a hand on it.
- A challenge that already has its user is refused on `/auth/google`, or a second person
  with the same link could put their account under the app that waits.
- A row nobody finished is dropped after 10 minutes, the client gives up after the same time.
- Sessions keep only the SHA-256 of the token. All times are the database clock.

## The wire

`hilen-server` never links `hilen`, so `wire.rs` exists twice, `hilen/src/login/wire.rs`
and `hilen-server/src/auth/wire.rs`. Both have a test that pins the JSON text. A change in
one needs the same change in the other.

## The session key

`SessionStore` seals the token with AES-256-GCM, a fresh nonce per save in front of the
bytes. The key is SHA-256 over a context text, the built in key and the machine id.

`hilen/build.rs` reads `HILEN_SESSION_KEY` when the `login` feature is on and writes
`session_key.rs` into `OUT_DIR`: the key xored with a random mask, the mask, and the shift
between the two. `store/session_key.rs` puts it back together behind `black_box`, without
it the optimizer folds the constants back into the plain key. Checked once by hand: an
optimized build with a known key does not hold that text.

No key means the public development key and a cargo warning. `HILEN_RELEASE` set means a
missing key fails the build. `build/release/with-secrets.sh` sets the mark, since every
shipped build runs through it, and the release scripts set it again on their own. The same
mark blocks the `inspect` feature, see [inspect.md](inspect.md). The cargo release profile cannot be the mark, `make run`
of an app builds with `--release` and must work without Infisical.

The build script prints `rerun-if-env-changed` for both names. One such line turns off
the default rerun on any package file, which the build time stamp needs, so it also
prints `rerun-if-changed` for `src`, `build.rs` and `Cargo.toml`. All of that only with
the feature on.

## The button and its test

`GoogleLoginButton` turns its own touch on before it wires the cancel button. The touch
view that signed up last is asked first and `Button::on_tap` is what signs a button up,
the other way round the view takes every tap meant for cancel.

`Google login button look` must never open a browser, also not from inside `demo` on a
device, where a server may be set. It swaps the server out for its tap, so the login
fails before `open_url`, and it shows the waiting look through `show_waiting_frozen`,
which holds the spinner still, a turning ring has no pixels to pin. Both seams exist only
with `ui-tests`.
