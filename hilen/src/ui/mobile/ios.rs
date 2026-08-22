use std::ffi::{c_char, c_float};

unsafe extern "C" {
    pub fn hilen_ios_show_alert(message: *const c_char);
    pub fn hilen_ios_init_text_field();
    pub fn hilen_ios_open_keyboard(x: c_float, y: c_float, width: c_float, height: c_float);
    pub fn hilen_ios_close_keyboard() -> *const c_char;
    pub fn hilen_ios_get_icloud_storage_path() -> *const c_char;
}
