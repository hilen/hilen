/// A text field is a different thing on a phone. Typing goes through the screen
/// keyboard, not through injected key events, so these drive a field that never
/// receives the text and then probe for glyphs that were never drawn.
#[cfg(desktop)]
mod custom_text_field;
mod font_zoo;
mod label;
mod label_color_runs;
mod label_fit_text;
mod label_font;
mod label_image;
mod label_measure;
mod label_stress;
mod label_vertical_alignment;
mod letter_spacing;
mod multiline_label;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod multiline_text_field;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod text_field;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod text_field_focus;
/// Sets colors and switches themes without typing, so unlike the other
/// text field tests it runs everywhere.
mod text_field_theme;
