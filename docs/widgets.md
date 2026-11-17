# Widgets and Live Activities on iOS

A hilen app draws its pages in Rust, but Apple takes a widget and a Live Activity only
in SwiftUI, from a WidgetKit extension. So the app writes that Swift itself, and
`hilen-mobile` builds it into the generated Xcode project. Banda,
`~/dev/apps/banda`, is the first app with both, its Swift is in `mobile-src/`.

## The table

```toml
[widget]
extension = "mobile-src/widget"
app = "mobile-src/app"
shared = "mobile-src/shared"
minimum = "26.0"
live_activities = true
push = true
```

- `extension`: the Swift of the extension, the `@main` `WidgetBundle` and its widgets.
  It must hold at least 1 file.
- `app`: Swift compiled into the app target. `main.m` calls the static `start` of a
  class `HilenAppStart` before the engine starts, when the app has one, so the
  class needs `@objc(HilenAppStart)`. The app target runs on its own
  `ios_minimum_version`, so this Swift checks `#available` itself.
- `shared`: Swift compiled into both targets, like the `ActivityAttributes` of a Live
  Activity, which both must have.
- `minimum`: the iOS of the extension, 17.0 when left out. The app keeps its own.
- `live_activities`: `NSSupportsLiveActivities` and
  `NSSupportsLiveActivitiesFrequentUpdates` in the app, and a weak link of
  ActivityKit, so the app still starts on a system without it.
- `push`: `aps-environment` on both targets. An archive for the App Store turns
  `development` into `production`.

The files stay in the app, the project points at them by their paths. A new file in a
folder needs a new `make ios` or `make fly`, the project is made again each time.

## What the generator adds

The code is `hilen-mobile/src/widget/` in `~/dev/hilen-mobile`, with tests on the real
template files.

- A target `<App>Widget` with the bundle id `<bundle>.widget`, its `Info.plist` and
  entitlements in `mobile/iOS/<App>Widget/`, embedded into the app. Its version is the
  `version` of `hilen.toml`, App Store Connect refuses an extension whose version is
  not the one of its app.
- The minimum of the extension is written in quotes. `build/ios/build-project.py` of an
  app sets every unquoted `IPHONEOS_DEPLOYMENT_TARGET` to the minimum of the app, which
  is too old for WidgetKit.
- `SWIFT_ACTIVE_COMPILATION_CONDITIONS = DEBUG` in the Debug builds, so the Swift can
  tell a development build, whose push tokens belong to the sandbox of APNs.
- A keychain group `$(AppIdentifierPrefix)<bundle>.shared` on both targets, its full
  name is `HilenKeychainGroup` in both `Info.plist` files. The app lists its own group
  first, a new keychain item goes into the first group, so only what the app shares
  on purpose is shared.

## Sharing a secret with the extension

`SecretStore::shared("name")` of the `secrets` feature writes into that group. The
Swift reads it as a generic password with the service `hilen-shared`, the name as the
account and the group of `HilenKeychainGroup`. It is readable after the first unlock,
so a widget reads it while the phone is locked. On other platforms it is the same as
`SecretStore::new`.

The extension has no App Group yet, and no way to call Rust. Both are in the roadmap.

## Checking it

The simulator build of a hilen app is Intel only and runs on the iOS 16.4 simulator,
which has no push to start and no widget push. So the simulator proves that both
targets build. Build for a device with signing off to compile the arm64 Swift too:

```sh
xcodebuild -project mobile/iOS/<App>.xcodeproj -scheme <App> -configuration Release \
  -destination 'generic/platform=iOS' -sdk iphoneos CODE_SIGNING_ALLOWED=NO build
```

Push and the widget itself are proven only on a phone.
