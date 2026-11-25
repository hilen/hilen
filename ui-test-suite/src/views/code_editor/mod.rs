//! The tests of `CodeEditor`. Keys come as injected key events, which
//! the editor reads on every platform, so these run everywhere but the
//! clipboard test. The fields of the find bar are typed into through
//! `system_input`, which types through the screen keyboard of a phone.

/// Not a test, what the code editor tests share.
mod common;

mod code_editor_app_replace;
/// Copies and pastes. Only a desktop keeps the clipboard of a test inside
/// the process, `Clipboard::set_in_process`, a phone would read and
/// replace what the user copied.
#[cfg(desktop)]
mod code_editor_clipboard;
mod code_editor_completion_kinds;
mod code_editor_completion_marked;
mod code_editor_completion_notes;
mod code_editor_completions;
/// The Ctrl letters of the text views of a Mac, which no other system
/// gives an editor.
#[cfg(macos)]
mod code_editor_control_keys;
mod code_editor_decor;
mod code_editor_editing_ended;
mod code_editor_find;
mod code_editor_find_next;
mod code_editor_inlays;
mod code_editor_joined_undo;
mod code_editor_keys_back;
/// Copies and pastes a whole line, the clipboard stays in the process
/// only on a desktop, like for `code_editor_clipboard`.
#[cfg(desktop)]
mod code_editor_line_clipboard;
mod code_editor_line_keys;
mod code_editor_long_file;
mod code_editor_look;
mod code_editor_menu;
mod code_editor_note;
mod code_editor_pairs;
/// Moves the pointer onto a panel with no button down.
#[cfg(any(desktop, wasm))]
mod code_editor_panel_rest;
/// Follows the pointer with no button down, which only a desktop and a
/// browser have.
#[cfg(any(desktop, wasm))]
mod code_editor_pointer;
mod code_editor_read_only;
mod code_editor_replace;
mod code_editor_selection;
mod code_editor_snippet;
mod code_editor_typing;
