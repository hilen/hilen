# Self update

`system::Updater` in `hilen/src/system/updater.rs` updates a desktop app in
place. Mobile goes through the stores and wasm updates by rehosting, so every
call outside the desktop is a no-op like `system::Router`.

## The contract

The app opts in from `App::update_source`:

```rust
fn update_source(&self) -> PinnedFuture<Option<UpdateSource>> {
    Box::pin(async {
        Ok(Some(UpdateSource {
            manifest_url:    "https://host/app/download/updater.json".into(),
            current_version: env!("CARGO_PKG_VERSION").into(),
            verify_key:      VERIFY_KEY.into(),
        }))
    })
}
```

`verify_key` is the hex of a raw 32 byte ed25519 public key. Its private half
signs every release artifact in CI and never ships. The manifest:

```json
{
  "version": "0.2.0",
  "notes": "",
  "platforms": {
    "macos-aarch64":  { "url": "...", "size": 1, "sha256": "..", "sig": ".." },
    "macos-x86_64":   { "url": "...", "size": 1, "sha256": "..", "sig": ".." },
    "windows-x86_64": { "url": "...", "size": 1, "sha256": "..", "sig": ".." }
  }
}
```

A platform key is `std::env::consts::OS` plus `-` plus `ARCH`, so `macos`,
`windows` and `linux` with `aarch64` or `x86_64`. The artifact is the bare
executable, not an installer or a bundle. `sig` is the hex ed25519 signature
over the whole file, `sha256` its hex digest.

## The calls

- `Updater::check()` fetches the manifest and returns `Ok(Some(UpdateInfo))`
  only when the manifest version is newer by semver and has an entry for
  this platform.
- `Updater::install(info)` downloads, checks size, sha256 and signature,
  then swaps the running executable through `self_replace`. Nothing is
  written before every check passes.
- `Updater::install_with_progress(info, |done, total| ..)` is the same with
  the download reported as bytes so far and the Content-Length, `None`
  when the server sent no length. `install` is this with a no-op callback.
- `Updater::relaunch()` spawns the new binary at the same path and stops
  this one.

All three run on tokio, so an app wires them with `spawn` and `on_main` like
any other fetch. The callback runs on the download task, post to main
before touching a view.

## What the swap means for packaging

The updater replaces one file, so an app that self updates must ship as one
executable. A helper next to the binary is never updated, fold it into the
main binary behind an env var or an argument. On mac the swapped binary sits
inside the signed `.app`, so CI signs the bare binary with the same identity
before hashing it, and the bundle keeps launching. On Windows the installer
is only for the first install, later versions swap the exe directly.

The kukareker port at `~/dev/apps/kukareker` is the reference wiring, its
`src/updater.rs` holds the app state and its `build/release/` scripts sign
and publish the manifest.
