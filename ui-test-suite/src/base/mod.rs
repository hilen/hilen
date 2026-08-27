mod async_calls;
mod clipboard;
mod color_checker;
mod colors;
mod corner_radius;
mod css_colors;
mod dispatch;
mod global_styles;
mod inspect_keys;
mod inspect_tap_modifiers;
mod keymap;
mod keymap_combo;
mod keymap_named_key;
mod layout;
mod modal_test;
mod navigation;
mod on_tap_add;
mod out_bounds_test;
mod present;
mod present_rich;
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
