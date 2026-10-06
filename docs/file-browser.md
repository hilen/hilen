# File browser

`FileBrowser` is a view that browses files like Finder or Explorer, on every platform.
`FilePicker` is the same view as a dialog. The code is
`hilen/src/ui/views/complex/file_browser/`, the sources are
`hilen/src/filesystem/source/`. It is always part of the engine, no cargo feature.

## Sources

The view never reads a disk itself. It asks a `FileSource`:

- `places()` gives the rows of the sidebar, the home folder and the roots. The first
  one is where a browser starts.
- `list(path, done)` gives the entries of a folder through a callback. It may be called
  from any thread and it may fail, the view shows the wait and the error with a
  Try again button. An answer that comes after the user moved on is dropped, the view
  counts its requests.
- `path_text` and `parse_path` turn a path into the text of the path bar and back.
- `can_write`, `make_folder`, `rename` and `delete` are optional. Without them the
  right click menu offers no change.

A path is a `FilePath`, a root and the names under it, not a `PathBuf`. A source can
be another machine with another separator, an archive or a cloud drive.

The engine brings 2 sources:

- `LocalFiles`, the disk of this machine, on desktop and on a phone. A browser has no
  disk, so it is not built for wasm. Every call runs on its own thread. It is read
  only unless made with `LocalFiles::writable()`, its delete is for good, nothing goes
  to a trash. `LocalFiles::path_buf` and `LocalFiles::file_path` convert to and from a
  `PathBuf`.
- `MemoryFiles`, a tree held in memory, for a test, a demo or the index of an
  archive. `set_holding`, `answer_held` and `fail_next_listing` let a test show the
  wait and the error.

A `FileEntry` carries its changed time as a wall clock time already in the zone the
user should read. `FileEntry::local_time` converts a time of this machine. The size and
date texts are fixed English, `1.2 MB` and `6 Oct 2026 14:05`, from `size_text` and
`date_text`.

## The view

```rust
self.browser.set_source(Arc::new(LocalFiles::new()));
self.browser.opened.val(move |path| ...);
```

- `set_mode` picks what it is for: `Browse`, `PickFolder`, `PickFile` or `PickFiles`.
  A pick mode shows the bar with Cancel and Choose and fires `chosen` and `cancelled`.
  A folder pick gives the picked folder, or the open one with none picked, and shows
  files faint. `set_extensions` leaves only the wanted files, folders always show.
- `open`, `go_back`, `go_forward`, `go_up`, `reload`. Going up picks the folder it
  came from.
- `set_sort`, `set_grid`, `set_show_hidden`, `set_search`, `set_sidebar_hidden`,
  `add_place`, `set_choose_title`.
- `set_menu_items` adds the app's entries to the right click menu. The closure gets the
  picked paths, none for a click on the empty area.
- Events: `path_changed`, `selection_changed`, `opened`, `chosen`, `cancelled`.

Folders always sort above files. A narrow view drops the date column, then the kind,
then the size, hides the sidebar below 480 points and puts the search on a second
toolbar row below 560.

`set_open_on_tap` is the touch screen way: a single tap opens a folder, and a file
while browsing, and in `PickFiles` a tap on a file adds it to the picked ones. It is on
by default on a phone and off on desktop, where a tap picks and a double tap opens.

## Keys

The list has the keys after a press on it and gives them back on a press anywhere
else. It takes them from the key focus ring with `Focus::hold_keys`, see
[focus.md](focus.md). Up and Down move, Shift adds rows, Enter opens, Backspace and
Cmd Up go to the parent, Home and End jump, typed letters jump to a name, Cmd A picks
all. In the grid Left and Right move too. In the list they give the keys back to the
ring, so a remote can leave the list.

## The dialog

```rust
FilePicker::pick(FilePick::folder(source).start(path), move |picked| {
    if let Some(paths) = picked { ... }
});
```

`FilePick::folder`, `file` and `files`, with `start`, `extensions` and `choose_title`.
The result is `None` on Cancel and on Escape. The dialog is 760 by 520 points and
shrinks to fit a small window.

## Tests

The UI tests are in `ui-test-suite/src/views/file_browser/`, 1 per part, all on one
`MemoryFiles` tree so they run on every platform. `row_center`, `crumb_center`,
`place_center` and `control_center` give a test the point to tap. The sources, the
sort, the filter and the selection have unit tests next to their code.
