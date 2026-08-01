mod background;
mod button;
mod checkbox;
mod corner_radii;
/// A text field is a different thing on a phone. Typing goes through the screen
/// keyboard, not through injected key events, so these drive a field that never
/// receives the text and then probe for glyphs that were never drawn.
#[cfg(desktop)]
mod custom_text_field;
mod font_zoo;
mod gradient;
/// Hover needs a pointer, and there is no such thing on a touch screen. `Input`
/// only calls `Hover::update` under `#[cfg(any(desktop, wasm))]`, so on a phone
/// this test waits for an event the engine never sends, asserts on the main
/// thread and takes the whole run down with it. Gated where the feature is
/// gated.
#[cfg(any(desktop, wasm))]
mod hover;
mod image_scissor;
mod inject_touch;
mod label;
mod label_fit_text;
mod label_font;
mod label_image;
mod label_measure;
mod label_stress;
mod letter_spacing;
mod multiline_label;
mod nine_segment;
mod shadow;
mod slider;
mod switch;
/// Desktop only for the same reason as [`custom_text_field`].
#[cfg(desktop)]
mod text_field;
mod theme_switch;
