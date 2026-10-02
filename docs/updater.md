# Self update

`system::Updater` in `hilen/src/system/updater.rs` updates a desktop app in
place. Mobile goes through the stores and wasm updates by rehosting, so every
call outside the desktop is a no-op like `system::Router`.

## The app side

An app gives 1 thing, its public key:

```rust
fn update_key(&self) -> Option<&'static str> {
    Some(include_str!("../assets/update-key.pub"))
}
```

The key is the hex of a raw 32 byte ed25519 public key. Its private half signs
every release artifact in CI and never ships. With the key the engine does the
rest:

- The version is the one of the app's own package. `register_app!` puts it into
  the app as `hilen_app_version`, so an app with an update key needs
  `register_app!`.
- The manifest address is `<DEFAULT_UPDATE_HOST>/<project_name>/updater.json`,
  with `https://get.vladas.xyz` as the host and the `project_name` of the app's
  `hilen.toml`. An app that ships from another place returns the full address
  from `App::update_url`.
- The env var `<NAME>_UPDATE_URL`, the project name in capitals with `_` for
  `-`, wins over both. It points a build at a local file server with a hand
  made `updater.json`, the way to test the whole flow without a release.
- 3 seconds after launch the engine checks once.

`system::UpdateState` holds what a view shows. All of it is main thread only:

- `UpdateState::get()` gives `phase()`, one of `Idle`, `Checking`, `Available`
  and `Installing`, the found `version()`, the download `progress()` in whole
  percent, the `error()` of the last failed check or install, and the short
  forms `has_update()` and `busy()`.
- `UpdateState::on_change(view, action)` runs the action after every change for
  as long as the view lives. Any number of views can subscribe.
- `UpdateState::check()` asks again, `UpdateState::install()` downloads,
  verifies, swaps the binary and starts the new one. A failed install goes back
  to `Idle` with the error set.

`App::update_source` is still there for an app that builds its source by itself
or decides it at run time. Its default builds the source from `update_key` and
`update_url`. The first check after launch runs only for an app with
`update_key`. The manifest:

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

Linux has one more shape. An app running as an AppImage cannot swap the
executable inside its read only mount, the file to replace is the image
itself. The AppImage runtime exports its path as `APPIMAGE`, and when that
is set the platform key gets an `-appimage` suffix, `linux-x86_64-appimage`,
and the manifest entry under it is the signed AppImage, not the bare binary.
`install` writes the temp next to the image and renames over it keeping its
permissions, so the swap stays on one filesystem.

## The calls

- `Updater::check()` fetches the manifest and returns `Ok(Some(UpdateInfo))`
  only when the manifest version is newer by semver, has an entry for
  this platform, and the swap target's directory is writable. A binary the
  user cannot swap, like a deb install in `/usr/bin`, gets no offer and
  updates through its package manager.
- `Updater::install(info)` downloads, checks size, sha256 and signature,
  then swaps the running executable through `self_replace`. Nothing is
  written before every check passes.
- `Updater::install_with_progress(info, |done, total| ..)` is the same with
  the download reported as bytes so far and the Content-Length, `None`
  when the server sent no length. `install` is this with a no-op callback.
- `Updater::relaunch()` spawns the new binary at the same path and stops
  this one.

All three run on tokio. `UpdateState` wraps them with `spawn` and `on_main`, so
an app normally never calls them. An app that does wires them like any other
fetch. The progress callback runs on the download task, post to main before
touching a view.

## What the swap means for packaging

The updater replaces one file, so an app that self updates must ship as one
executable. That covers its assets too. An installed app has no `assets`
folder: the Mac bundle and the Windows installer pack only the binary. So a
desktop app that ships calls `hilen::embed_assets!();` once, next to
`register_app!`. It packs the `fonts`, `images`, `models` and `sounds` folders
of the app's `assets` into the binary at build time. The engine reads a file
from disk when it is there and from the packed copy when it is not, so a run
from the repo still sees an edited file at once. The start log says how many
files were packed when no folder was found. A file added to `assets` is packed
by the next build that compiles the app crate, a release build always does. A helper next to the binary is never updated, fold it into the
main binary behind an env var or an argument. On mac the swapped binary sits
inside the signed `.app`, so CI signs the bare binary with the same identity
before hashing it, and the bundle keeps launching. On Windows the installer
is only for the first install, later versions swap the exe directly.

Kukareker and Blackforge were wired before `UpdateState` existed. Each still
carries its own state module in `updater.rs` and overrides `update_source`,
which keeps working. Their `build/release/` scripts sign and publish the
manifest. They are not a template for other apps.
Each app keeps its own release pipeline, runners and download host, and a
game can ship in a completely different way.
