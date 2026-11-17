//! The tests of `CodeEditor`. Keys come as injected key events, which
//! the editor reads on every platform, so these run everywhere but the
//! clipboard test. The fields of the find bar are typed into through
//! `system_input`, which types through the screen keyboard of a phone.

/// Not a test, what the code editor tests share.
mod common;

/// Copies and pastes. Only a desktop keeps the clipboard of a test inside
/// the process, `Clipboard::set_in_process`, a phone would read and
/// replace what the user copied.
#[cfg(desktop)]
mod code_editor_clipboard;
mod code_editor_find;
mod code_editor_find_next;
mod code_editor_long_file;
mod code_editor_look;
mod code_editor_replace;
mod code_editor_selection;
mod code_editor_typing;
