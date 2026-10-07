use std::ffi::{c_char, c_float, c_int, c_uchar, c_uint, c_ulong};

/// What the system text field needs to stand in for an engine field, see
/// `text_field/system_field.rs`. Must match `HilenTextEdit` in
/// `hilen/native/ios/hilen_text.m`. Lengths are in the pixels of the window.
#[repr(C)]
pub struct TextEdit {
    pub x:      c_float,
    pub y:      c_float,
    pub width:  c_float,
    pub height: c_float,

    pub text:        *const c_char,
    pub text_size:   c_float,
    pub line_height: c_float,
    pub red:         c_float,
    pub green:       c_float,
    pub blue:        c_float,
    pub alpha:       c_float,

    pub inset:     c_float,
    pub top_inset: c_float,

    /// 0 left, 1 center, 2 right.
    pub alignment: c_int,
    /// 0 text, 1 numbers.
    pub keyboard:  c_int,

    pub secure:    bool,
    pub multiline: bool,
    pub dark:      bool,

    pub font:             *const c_uchar,
    pub font_length:      c_ulong,
    pub font_key:         *const c_char,
    pub variation_tags:   *const c_uint,
    pub variation_values: *const c_float,
    pub variation_count:  c_int,
}

unsafe extern "C" {
    pub fn hilen_ios_show_alert(message: *const c_char);
    pub fn hilen_ios_get_icloud_storage_path() -> *const c_char;
}

// The Apple TV shell has no system text field.
#[cfg(not(tvos))]
unsafe extern "C" {
    pub fn hilen_ios_text_begin(
        edit: *const TextEdit,
        changed: extern "C" fn(*const c_char),
        returned: extern "C" fn(),
        ended: extern "C" fn(),
    );
    pub fn hilen_ios_text_set(text: *const c_char);
    pub fn hilen_ios_text_move(x: c_float, y: c_float, width: c_float, height: c_float);
    pub fn hilen_ios_text_end();
    /// `moved` gets the top edge the screen keyboard moves to, in the
    /// pixels of the window, negative when it goes away, and the seconds
    /// the move takes.
    pub fn hilen_ios_keyboard_watch(moved: extern "C" fn(c_float, c_float));
}
