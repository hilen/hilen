mod animated_gif;
mod animation_drives_frames;
/// The bug report dialog rides Sentry, which does not run on wasm.
#[cfg(not_wasm)]
mod bug_report_dialog;
mod frame_stepped_animation;
mod hidden_parent_touch;
mod hidden_touch;
mod manual_z_position;
mod offscreen_clip;
mod outline;
mod overlay_touch_layer;
mod script_wrap;
mod switch_look;
mod title_frames;
/// A phone and a page fill the screen, only a desktop window has an
/// initial size.
#[cfg(desktop)]
mod window_size;
