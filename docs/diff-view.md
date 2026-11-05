# Diff view

`DiffPanel` of the `diff-view` feature draws the diff of 1 file: in 1 column or split in
2, with syntax colors, a text selection, the 2 sides of an image and buttons that
discard a hunk or a line. It came from the git client kukareker, banda shows it too. The
code is `hilen/src/ui/views/complex/diff_view/`.

## The data

The diff is plain data from the [filediff](https://github.com/hilen/filediff) crate,
re-exported as `hilen::filediff`: `FileDiff`, `FileDiffBody`, `DiffHunk`, `DiffLine`,
`Blob` for a side of an image, `DiffTarget` for what the diff is against, and
`hunk_patch` and `line_patch`, the reverse patches of a discard. It has no UI and runs no
git, so a daemon that runs git builds the same types and sends them as JSON.

It is a repo of its own on purpose. A git dependency is 1 crate per commit of its repo.
In the hilen repo every hilen commit would make another copy of the types, and an app
would have to pin hilen at the same commit as every crate of its own that builds diffs.
The rev of `filediff` moves only when the types change. Every crate that names it pins
the same rev, hilen included.

## The host

The panel loads nothing itself. The app gives it a `DiffHost`: `load` a diff for a
`DiffShown`, `discard` a patch, `confirm` before a discard, `report` how it went, `close`,
`mono` for the font of the code and `mode_changed` when the user picks 1 or 2 columns.
`show(shown, version, can_discard)` shows a file, `set_split` picks the mode from code.
The colors are `hilen::ui::diff_colors`, light and dark pairs.

## Inside

- The syntax colors come from syntect with its default Sublime themes, so `diff-view`
  turns `code-colors` on. They run on a thread of their own, in a browser in the same
  turn, and the lines past `MAX_LINES` stay plain.
- The discard buttons show while the pointer is over a row, so a phone has none.

## Tests

`Diff panel look` shows 1 changed Rust file from a host in memory, in 1 column and in 2,
and waits for the syntax colors before its first check. The unit tests of the selection
are next to it, the patches have theirs in `filediff`.
