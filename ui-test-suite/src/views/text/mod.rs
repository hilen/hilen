mod clipped_text_batches;
mod code_highlighter_lines;
mod color_emoji;
mod color_glyph_clip;
/// A text field is a different thing on a phone. Typing goes through the screen
/// keyboard, not through injected key events, so these drive a field that never
/// receives the text and then probe for glyphs that were never drawn.
#[cfg(desktop)]
mod custom_text_field;
mod font_zoo;
mod glyph_fallback;
mod label;
mod label_color_runs;
mod label_effect_made_again;
mod label_effect_over_new_image;
mod label_ellipsize;
mod label_ellipsize_head;
mod label_fit_text;
mod label_font;
mod label_font_runs;
mod label_image;
mod label_line_height;
mod label_max_lines;
mod label_measure;
mod label_outline;
mod label_selection;
mod label_soft_shadow;
mod label_stress;
mod label_strikethrough;
mod label_tab;
mod label_vertical_alignment;
mod label_wide_effect;
mod letter_spacing;
mod markdown_plain_height;
mod markdown_selection;
mod markdown_spacing;
mod markdown_view;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod modal_escape_text_field;
mod multiline_label;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod multiline_submit;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod multiline_text_field;
/// The `screen_keyboard` tests type through the real keyboard of the
/// platform, so unlike the other typing tests they run on a phone too.
mod screen_keyboard_input;
/// The system text field this one compares the engine field with exists
/// only on an iPhone, nowhere else is there a switch to hide.
#[cfg(all(ios, not(tvos)))]
mod screen_keyboard_look;
mod screen_keyboard_multiline;
mod screen_keyboard_secure;
mod screen_keyboard_submit;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod secure_text_field;
mod stem_darkening;
/// Apple Color Emoji ships with Apple hardware only, `Font::system_emoji`
/// is `None` everywhere else.
#[cfg(apple)]
mod system_emoji;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod tab_focus;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod text_field;
/// Probes the caret the engine draws in a focused field. On an iPhone the
/// system text field draws the caret, outside the engine frame, and
/// [`screen_keyboard_look`] checks there that it follows the scale.
#[cfg(not(all(ios, not(tvos))))]
mod text_field_caret_scale;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod text_field_focus;
/// Sets the font and text programmatically, no typing, so it runs
/// everywhere too.
mod text_field_font;
/// Pastes with an injected key shortcut, which a phone and a browser do
/// not have, and only a desktop clipboard hands over a picture.
#[cfg(desktop)]
mod text_field_image_paste;
/// Enters and clears text programmatically, no typing, so it runs
/// everywhere like [`text_field_theme`].
mod text_field_placeholder_color;
/// Reads the text of empty and filled fields, no typing, so it runs
/// everywhere like [`text_field_theme`].
mod text_field_placeholder_text;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod text_field_submit;
/// Sets colors and switches themes without typing, so unlike the other
/// text field tests it runs everywhere.
mod text_field_theme;
