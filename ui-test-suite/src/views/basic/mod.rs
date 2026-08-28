mod background;
mod button;
mod button_disabled;
mod checkbox;
mod command_held;
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
/// The cursor icon rides on hover, the same gate as `hover`.
#[cfg(any(desktop, wasm))]
mod hover_cursor;
/// Hover re-pick on view removal, the same gate as `hover`.
#[cfg(any(desktop, wasm))]
mod hover_removal;
mod image_scissor;
mod inject_touch;
mod nine_segment;
mod rounded_clip;
mod secondary_click;
mod shadow;
mod slider;
mod switch;
mod theme_switch;
/// Hover tooltips need a pointer, the same gate as `hover`.
#[cfg(any(desktop, wasm))]
mod tooltip_hover;
mod tooltip_long_press;
