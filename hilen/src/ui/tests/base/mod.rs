mod animated_gif;
mod animation_drives_frames;
mod bug_report_dialog;
mod bug_report_style;
mod double_tap;
mod frame_stepped_animation;
mod hidden_parent_touch;
mod hidden_touch;
mod manual_z_position;
mod offscreen_clip;
mod outline;
mod overlay_touch_layer;
mod screen_orientation;
mod script_wrap;
/// Typing goes through the screen keyboard on a phone, a field takes
/// injected chars on desktop only.
#[cfg(desktop)]
mod stale_selection;
mod switch_look;
/// A browser page cannot read the system fonts, `Font::set_system_fallback`
/// does nothing there.
#[cfg(not_wasm)]
mod system_font_fallback;
mod title_frames;
/// A phone and a page fill the screen, only a desktop window has an
/// initial size.
#[cfg(desktop)]
mod window_size;
