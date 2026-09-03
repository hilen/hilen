use crate::{
    deps::{refs::main_lock::MainLock, vents::Event},
    gm::{flat::Point, volume::GyroData},
    ui::{Theme, Touch, UIEvent},
    window::NamedKey,
};

static UI_EVENTS: MainLock<UIEvents> = MainLock::new();

#[derive(Default)]
pub struct UIEvents {
    on_touch:       Event<Touch>,
    touch_began:    UIEvent<Touch>,
    on_scroll:      UIEvent<Point>,
    on_debug_touch: Event<Touch>,
    size_changed:   UIEvent<()>,
    theme_changed:  UIEvent<Theme>,
    gyro:           UIEvent<GyroData>,
    keyboard_input: UIEvent<char>,
    keyboard_key:   UIEvent<NamedKey>,
}

impl UIEvents {
    pub(crate) fn on_touch() -> &'static Event<Touch> {
        &UI_EVENTS.on_touch
    }

    /// Every began touch of the app, before view dispatch. Public so a
    /// view can notice a click landing outside itself, the way a
    /// selection clears when the user clicks elsewhere.
    pub fn touch_began() -> &'static UIEvent<Touch> {
        &UI_EVENTS.touch_began
    }

    /// Every wheel delta of the app, before the scroll views get it.
    /// Public so a page can zoom a scene camera with the wheel.
    pub fn on_scroll() -> &'static UIEvent<Point> {
        &UI_EVENTS.on_scroll
    }

    /// Always triggered
    pub(crate) fn on_debug_touch() -> &'static Event<Touch> {
        &UI_EVENTS.on_debug_touch
    }

    pub(crate) fn size_changed() -> &'static UIEvent<()> {
        &UI_EVENTS.size_changed
    }

    /// Triggered after the effective theme changes, with the new theme.
    pub fn theme_changed() -> &'static UIEvent<Theme> {
        &UI_EVENTS.theme_changed
    }

    /// Every typed character of the app, pressed keys only, with the
    /// system's key repeat. Public so a page can walk a scene camera
    /// from the keyboard.
    pub fn keyboard_input() -> &'static UIEvent<char> {
        &UI_EVENTS.keyboard_input
    }

    /// Every pressed named key, arrows, enter and the like.
    pub fn keyboard_key() -> &'static UIEvent<NamedKey> {
        &UI_EVENTS.keyboard_key
    }

    pub fn gyro() -> &'static UIEvent<GyroData> {
        &UI_EVENTS.gyro
    }
}
