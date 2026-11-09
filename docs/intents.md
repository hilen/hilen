# Intents, Siri and the Shortcuts app

An intent is an action the system runs in the app, like "Hey Siri, add to Shopping".
iOS only for now. The engine half is the `intents` feature of `hilen`, in
`hilen/src/intents`. The Swift half is made by `hilen-mobile` from the `[[intent]]`
tables of `hilen.toml`. Shopping, `~/dev/apps/shopping`, is the first app with one.

## The table

```toml
[[intent]]
name    = "add_item"
title   = "Add an item"
locked  = true
phrases = ["Add to {app} list", "Add to {app}", "Add to {list} in {app}"]

[[intent.parameter]]
name   = "item"
title  = "Item"
kind   = "text"
prompt = "What do you want to add?"

[[intent.parameter]]
name     = "list"
title    = "List"
kind     = "choice"
optional = true
```

- `name` is the name of the Rust handler. The Swift type is `AddItemIntent`.
- `locked` lets the intent run on a locked phone with no Face ID first. Off by
  default.
- A parameter is a `text` or a `choice`. A choice takes 1 of the values the app gives
  with `Intents::options`, like the lists of the user.
- `prompt` is what Siri asks when the value is missing. An `optional` parameter is
  never asked for.

Apple sets the rules for the phrases, and `hilen-mobile` refuses a table that breaks
them, before the old project is deleted:

- Each phrase has `{app}`, the name of the app, the `CFBundleName` of its bundle.
- A phrase names at most 1 parameter, and only a choice. A text can not be part of a
  phrase, Siri asks for it after the phrase. So "add milk to Shopping" in 1 sentence
  is not possible, it is "add to Shopping", then "milk".
- Siri matches the words of a phrase closely. "Add to shopping list" does not match
  "Add to {app}", the phrase "Add to {app} list" is needed for it.
- An app has at most 10 intents.

## The app

```rust
use hilen::intents::{IntentOption, Intents};

impl App for MyApp {
    fn before_launch(&self) {
        Intents::on("add_item", |args| {
            let added = save(args.text("item"), args.text("list"));
            async move { added }
        });
        Intents::options("list", || lists().map(|l| IntentOption::new(l.id, l.name)).collect());
    }
}
```

- The handlers are registered in `before_launch`. iOS starts the app in the
  background for an intent, and winit sends `Resumed` only from
  `applicationDidBecomeActive`, so there is no window and no `setup` of a view.
  `before_launch` is the only hook that runs then.
- The closure runs on the main thread and does the work that needs the state of the
  app. The future may wait for the network. Its text is what Siri says, its error is
  said in its place.
- `IntentArgs::text` gives a value as text, the id of the option for a choice. A
  parameter left out is `None`.
- `Intents::options_changed` tells Siri that the values of a choice changed, so it
  knows the new names in the phrases. Call it when they change, not on every change
  of the data.
- An intent with no handler answers with an error and logs a warning.

## How it works on iOS

- `hilen-mobile` writes `HilenIntents.swift` and `HilenIntents.h` into the app
  folder of the project and compiles the Swift in the app target, with the header
  as its bridging header. Every type is for iOS 16 and later. The app keeps its own
  minimum, AppIntents is linked weak by itself, so an older iOS still starts it.
- Apple reads the intents from the Swift when it builds the app, the step
  `ExtractAppIntentsMetadata`. The result is `Metadata.appintents` in the app. A
  debug build has the phrases as text in its `root.ssu.yaml`, a release build packs
  them.
- `perform` calls `hilen_intent_run` on the main actor. UIKit drains the main queue
  with no window, so the call does not wait for a frame. The answer comes back
  through a C callback from the thread the future ended on.
- A choice is an `AppEntity` whose query asks `hilen_intent_options` for its values.
- `Intents::options_changed` calls `HilenIntents.updateParameters`, which calls
  `updateAppShortcutParameters` of Apple.
- `make device` runs the app inside a loader app, which has no intents. Only a real
  install, like TestFlight, has them.

## Checking it

- `cargo test -p hilen --features intents --lib intents::` tests the handlers, the
  parameters and the values of a choice on a desktop.
- `cargo test -p hilen-mobile intents::` in `~/dev/hilen-mobile` tests the table,
  the Swift and the project on the real template.
- A device build with signing off proves that the Swift compiles and that Apple
  took the phrases, read them in `Metadata.appintents/root.ssu.yaml`.
- The simulator has no command that starts an intent. Siri, a locked phone and a
  start in the background are proven only on a phone.
