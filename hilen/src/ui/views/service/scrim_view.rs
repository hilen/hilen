use ui_proc::view;

/// The fullscreen backdrop behind a modal. A dedicated type because
/// the drawer flushes it after every other pipeline including text,
/// so its translucent color dims everything already on screen. A
/// plain rect would erase later pipelines through the depth buffer
/// instead of dimming them.
#[view]
pub struct ScrimView {}
