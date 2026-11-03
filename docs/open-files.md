# Files that open the app

A file from Open With or a double click goes to the app. The code is in
`hilen/src/system/open_files.rs` and `hilen/src/system/links/macos.rs`, the packaging
in `build/py/shared/open_files.py`.

## The app

```rust
use hilen::system::OpenFiles;

OpenFiles::on_open(self, move |path: PathBuf| {
    // the path of the file, or of the folder
});
```

- The action runs on the main thread, in a turn of the loop of its own, while the
  subscriber lives.
- A path that comes before anybody listens is kept and goes to the first subscriber.
  That is how the file of a cold start arrives.
- Each path is given once.

## hilen.toml

```toml
[open_files]
types = ["public.movie", "public.folder"]
```

A type is an Apple uniform type. `build/release/mac.py` writes them into the
`Info.plist` as `CFBundleDocumentTypes`. The rank is Alternate, so the app shows in Open
With and the default app of the user stays.

## How a path arrives

- A Mac bundle gets the Apple Event of open documents, at the start and while it runs.
  It takes the place of the handler of `AppKit`, the same way as the link event, so the
  handler goes in when the app is about to finish its launch.
- Every desktop start also reads its arguments, and an argument that is an existing path
  counts. That is the way for a run from a terminal.

The rest is open, see the entry in [roadmap.md](roadmap.md).

## A look on this machine

Open With needs a bundle that the system knows. `build/release/mac_install.py`, run from
the app folder, makes a bundle of `target/release/<name>`, signs it for this machine and
puts it into `/Applications`. It reads `hilen.toml` and not cargo, so it also runs on a
machine with no cargo. A release goes through `mac.py`.

The bundle in `target` is unregistered and deleted. With it the system lists the app
twice in Open With.

To check the list without Finder, ask `NSWorkspace`, the same list Finder uses:

```sh
osascript -l JavaScript -e 'ObjC.import("AppKit"); var u=$.NSWorkspace.sharedWorkspace.URLsForApplicationsToOpenURL($.NSURL.fileURLWithPath("/path/to/film.mp4")); var o=[]; for (var i=0;i<u.count;i++){o.push(u.objectAtIndex(i).path.js)}; o.join("\n")'
```
