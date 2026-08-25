mod background;
mod button;
mod button_disabled;
mod checkbox;
mod corner_radii;
mod dynamic_clear_color;
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
mod nine_segment;
mod shadow;
mod slider;
mod switch;
mod theme_switch;
