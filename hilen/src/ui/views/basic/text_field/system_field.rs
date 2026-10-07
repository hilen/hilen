//! The system text field an iPhone edits an engine field in.
//!
//! While a field is edited, a real `UITextField`, or a `UITextView` for a
//! multiline field, sits exactly over it and takes the keys. So the screen
//! keyboard, autocorrect, text composing and the selection handles are the
//! ones of iOS. The engine field draws no text and no caret for that time,
//! and gets every change of the text at once. The native half is
//! `hilen/native/ios/hilen_text.m`, which the build script compiles in.

use std::{
    ffi::{CStr, c_char},
    ptr::null,
    sync::Once,
};

use parking_lot::Mutex;
use zeroize::Zeroizing;

use super::{MULTILINE_TOP_INSET, TextField};
use crate::{
    deps::refs::Weak,
    ui::{
        Input, NamedKey, ScreenKeyboard, TextAlignment, Theme, UIManager,
        mobile::ios::{
            TextEdit, hilen_ios_keyboard_watch, hilen_ios_text_begin, hilen_ios_text_end,
            hilen_ios_text_move, hilen_ios_text_set,
        },
        view::ViewFrame,
    },
};

/// The field the system field stands in for now. The callbacks of the system
/// find it here. They cannot ask `UIManager` for the selected view, the last
/// change arrives while the selection is being changed and is locked.
static EDITED: Mutex<Weak<TextField>> = Mutex::new(Weak::const_default());

/// The text with a NUL at its end, what the system reads. It wipes itself,
/// the text can be a password.
fn c_text(text: &str) -> Zeroizing<Vec<u8>> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(text.len() + 1));
    // A NUL inside would cut the text short on the other side.
    bytes.extend(text.bytes().filter(|byte| *byte != 0));
    bytes.push(0);
    bytes
}

/// The frame of the field in the pixels of the window, which is what the
/// native side counts in.
fn pixel_frame(field: &TextField) -> [f32; 4] {
    let scale = UIManager::scale();
    let frame = field.absolute_frame();
    [
        frame.x() * scale,
        frame.y() * scale,
        frame.width() * scale,
        frame.height() * scale,
    ]
}

pub(super) fn begin(field: Weak<TextField>) {
    // Only this field brings the keyboard up, so nothing is missed before
    // the first one.
    static WATCH: Once = Once::new();
    // SAFETY: `keyboard_moved` is a plain function that lives for good.
    WATCH.call_once(|| unsafe { hilen_ios_keyboard_watch(keyboard_moved) });

    *EDITED.lock() = field;

    let scale = UIManager::scale();
    let [x, y, width, height] = pixel_frame(&field);

    let label = field.label;
    let font = label.font();
    let color = field.text_color.resolve();
    let text = c_text(field.text());

    let (font_data, variations) = font.system_source().unzip();
    let variations = variations.unwrap_or_default();
    let key = c_text(&font.name);
    let tags: Vec<u32> = variations.iter().map(|(tag, _)| u32::from_be_bytes(*tag)).collect();
    let values: Vec<f32> = variations.iter().map(|(_, value)| *value).collect();

    let edit = TextEdit {
        x,
        y,
        width,
        height,
        text: text.as_ptr().cast(),
        text_size: label.text_size() * scale,
        line_height: label.text_layout_for("").line_height * scale,
        red: color.r,
        green: color.g,
        blue: color.b,
        alpha: color.a,
        inset: label.text_inset() * scale,
        top_inset: MULTILINE_TOP_INSET * scale,
        alignment: match label.alignment {
            TextAlignment::Left => 0,
            TextAlignment::Center => 1,
            TextAlignment::Right => 2,
        },
        // A number can start with a minus, which the plain number pad
        // does not have.
        keyboard: i32::from(field.constraint.is_some()),
        secure: field.is_secure(),
        multiline: field.multiline,
        dark: Theme::current() == Theme::Dark,
        font: font_data.map_or(null(), <[u8]>::as_ptr),
        font_length: font_data.map_or(0, <[u8]>::len).try_into().unwrap_or(0),
        font_key: key.as_ptr().cast(),
        variation_tags: tags.as_ptr(),
        variation_values: values.as_ptr(),
        variation_count: tags.len().try_into().unwrap_or(0),
    };

    // SAFETY: every pointer in `edit` lives until the call returns, and the
    // native side copies what it keeps. The font bytes live for the whole
    // process.
    unsafe { hilen_ios_text_begin(&raw const edit, changed, returned, ended) };
}

pub(super) fn end() {
    // SAFETY: takes no argument and does nothing when no field is edited.
    unsafe { hilen_ios_text_end() };
    *EDITED.lock() = Weak::default();
}

/// The engine changed the text of the edited field itself.
pub(super) fn set_text(text: &str) {
    let text = c_text(text);
    // SAFETY: `text` ends in a NUL and lives until the call returns.
    unsafe { hilen_ios_text_set(text.as_ptr().cast()) };
}

/// Keeps the system field over the engine field, which can move while it
/// is edited, in a scroll view or when the screen turns.
pub(super) fn follow(field: &TextField) {
    let [x, y, width, height] = pixel_frame(field);
    // SAFETY: plain numbers.
    unsafe { hilen_ios_text_move(x, y, width, height) };
}

extern "C" fn changed(text: *const c_char) {
    let field = *EDITED.lock();
    if field.is_null() || text.is_null() {
        return;
    }
    // SAFETY: the system passes a NUL terminated string that lives for
    // this call.
    let text = unsafe { CStr::from_ptr(text) };
    field.system_text_changed(Zeroizing::new(text.to_string_lossy().into_owned()));
}

extern "C" fn returned() {
    Input::on_key(NamedKey::Enter);
}

/// The system moves the keyboard, see `ScreenKeyboard`.
extern "C" fn keyboard_moved(top: f32, duration: f32) {
    let top = (top >= 0.0).then(|| top / UIManager::scale());
    ScreenKeyboard::moves_to(top, duration);
}

/// The system ended the editing by itself.
extern "C" fn ended() {
    UIManager::unselect_view();
}
