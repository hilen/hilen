# Push

A backend sends a push through APNs, the push service of Apple, and the phone
shows it on the lock screen. A tap opens the app on the page the push is about.
iOS only for now, the other systems are in the roadmap. The engine half is the
`push` feature of `hilen`, in `hilen/src/push`. The server half is the `push`
module of `hilen-server`.

## The app

`hilen.toml` asks for the entitlement:

```toml
[ios]
push = true
```

`hilen-mobile` writes `aps-environment` into the entitlements of the app, as
`development`. An archive for the App Store turns it into `production`.
Automatic signing turns the push service on for the App ID by itself.

```rust
use hilen::push::{Push, PushMessage, PushToken};

Push::on_token(weak, |token: PushToken| send_to_backend(token.hex, token.sandbox));
Push::on_open(weak, |message: PushMessage| open_page(message.data::<Target>()));
spawn(async { Push::request().await });
```

- `Push::request` asks the user once with the dialog of iOS. A later call
  answers at once with the choice of the user. `Ok(true)` registers the app
  with APNs, the token then comes to `on_token`. Call it on every start, APNs
  can change the token.
- `PushToken::hex` is the device token in hex. `sandbox` says which APNs the
  token belongs to. The simulator and a development build are the sandbox. A
  TestFlight and an App Store build are production, they have no
  `embedded.mobileprovision` in the app. A device build reads `aps-environment`
  from that file.
- `Push::on_open` gets every push the user taps, with the whole payload as
  JSON. `PushMessage::data` reads the keys next to `aps` into a type of the
  app. The push that started the app waits for the first subscriber, like a
  link does.
- `Push::token` is the token of this run, for the call at a logout that tells
  the backend to forget it. `Push::is_supported` is `true` only on iOS.
- A push that comes while the app is in front shows as a banner too.
- A log line shows only the first 8 characters of a token.
- On any other system `request` answers with an error and nothing else happens,
  so the same app code runs everywhere.

## How it works on iOS

- The token comes to the delegate of the app, which the engine sets in
  `hilen/src/system/ios_delegate.rs`, the same delegate that takes the links.
  Its 2 push methods are only there with the feature, so an app with no push
  carries no call to register for it.
- The tap comes to `HilenPushDelegate`, the delegate of
  `UNUserNotificationCenter`. It is set when the launch ends, which is early
  enough for the tap that started the app.
- A hot build runs inside the loader app, which has its own delegate. It gets
  no push, the same as no link.

## The backend

```rust
use hilen_server::push::{Apns, ApnsAlert, ApnsConfig, Delivery};

let apns = ApnsConfig::from_env("vladas.pirkti")?.map(Apns::new).transpose()?;

let alert = ApnsAlert::new("Weekly", "Dave added milk").thread("list-7");
match apns.send(&token, sandbox, &alert, &Target { list: 7 }).await? {
    Delivery::Sent => {}
    Delivery::TokenGone => forget_token(&token).await?,
}
```

- `from_env` reads `APNS_TEAM_ID`, `APNS_KEY_ID` and `APNS_KEY`, the text of the
  `.p8` file from the Keys page of Apple Developer. With no `APNS_KEY` it gives
  `None` and the server sends no push. 1 key serves every app of the team.
- `APNS_KEY` may be the whole PEM text or only its base64 body on 1 line. A
  value with line breaks can break in a `.env` file, so a compose file takes
  the 1 line form.
- `Apns::new` signs a token once, so a wrong key stops the start.
- The provider token is signed with the key, ES256, the same way as the client
  secret of Sign in with Apple, in `hilen-server/src/apple_key.rs`. It is kept
  for 50 minutes. APNs takes one for up to 1 hour and refuses a new one more
  often than every 20 minutes.
- `send` goes to `api.sandbox.push.apple.com` for a sandbox token and to
  `api.push.apple.com` for the others. reqwest speaks HTTP/2 there by itself,
  APNs takes nothing else.
- `Delivery::TokenGone` is the answer for a token that never takes a push
  again: the app was removed, or the token is of another app or of the other
  APNs. The backend forgets it.
- `ApnsAlert::thread` groups pushes on the lock screen, `collapse` puts a new
  push in the place of the one shown before.

## Checking it

- `cargo test -p hilen --features push --lib push::` tests the payload, the
  profile and the token on a desktop.
- `cargo test -p hilen-server --lib push::` sends to a stand in for APNs.
- In the simulator, `xcrun simctl push <device> <bundle id> payload.json`
  puts a push into the app with no APNs. It proves the banner and the tap.
- The simulator of an Apple Silicon Mac gets a real sandbox token from APNs.
- Production APNs is proven only by a TestFlight build on a phone.
