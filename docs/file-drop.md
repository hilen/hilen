# File drop

A file or a folder dragged from a file manager onto the window. The calls are on
`UIManager`, the code is `hilen/src/ui/ui_manager.rs`. The window events come from winit
in `hilen/src/window/app_handler.rs`.

## The calls

- `UIManager::on_drop_file(subscriber, |path| ..)` runs when an item is dropped. A drop
  of several items calls it once per item, all in the same frame.
- `UIManager::on_hover_file(subscriber, |path| ..)` runs when an item is held over the
  window, before a drop. It is called once per held item when it enters the window.
- `UIManager::on_hover_file_end(subscriber, || ..)` runs when the hold is over: the items
  left the window, or they were dropped. After a drop it comes before `on_drop_file`.

The subscriber is a weak reference to a view, the action stops with that view. A path of
a folder is the folder itself, the app walks it.

A view that shows a "drop here" mark turns it on in `on_hover_file` and off in
`on_hover_file_end`. It needs no extra off in `on_drop_file`.

## Platforms

Only a desktop window reports a drop, and only a desktop window reports a hold. On every
other platform the 3 actions are never called.

## Tests

`inject_file_hover(path)`, `inject_file_hover_end()` and `inject_file_drop(path)` are the
UI test helpers. The paths do not have to exist. `File hover` in
`ui-test-suite/src/base/file_hover.rs` is the test.
