mod async_calls;
mod clipboard;
mod color_checker;
mod colors;
mod corner_radius;
mod css_colors;
mod cursor_capture;
mod dispatch;
mod global_styles;
// The engine compiles the inspector out on wasm, so its tests go with it.
#[cfg(not_wasm)]
mod inspect_keys;
#[cfg(not_wasm)]
mod inspect_tap_modifiers;
mod keymap;
mod keymap_combo;
mod keymap_named_key;
mod layout;
mod level_leak;
mod modal_test;
mod navigation;
mod on_tap_add;
mod out_bounds_test;
mod present;
mod present_rich;
/// Reload shortcuts exist only in a browser, everywhere else the OS owns
/// them and the engine installs nothing.
#[cfg(wasm)]
mod reload_shortcuts;
mod rest_request;
mod root_view;
/// Browser history is the production behavior, everywhere else the
/// router API is a no-op, so there is nothing to test.
#[cfg(wasm)]
mod router;
mod scale;
mod selection;
mod styles;
mod text_occlusion;
mod touch_order;
mod touch_stack;
mod transition;
mod transition_rich;
mod transparency;
mod view_order;
