# Global keys

The `global-keys` feature: a key combination that works in every app while this app
runs, through [global-hotkey](https://github.com/tauri-apps/global-hotkey). macOS has it,
Windows and Linux answer with an error, see the roadmap. The code is `hilen/src/global_keys/`.

```rust
let key = GlobalKey::register("Shift+Cmd+P".parse()?, || open_overlay())?;
```

- `GlobalCombo` is the physical key and the 4 modifiers. Its text form, like
  `Shift+Cmd+P`, is what `Display` writes, `FromStr` reads and serde keeps. A key with
  no modifier is refused, it would take the key from every app.
- The action runs on the main thread in the next turn of the loop, the system calls the
  handler inside its own event. A covered main window runs the queued work in
  `about_to_wait`.
- Dropping the `GlobalKey` gives the combination back. A combination another app holds
  answers with an error.
- Not the same as `KeyCombo` of the keymap, which is a typed key inside the app.
