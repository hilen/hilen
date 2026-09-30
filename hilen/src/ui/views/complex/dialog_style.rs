use crate::{
    deps::refs::main_lock::MainLock,
    gm::color::Color,
    ui::{DynamicColor, UIColor},
};

static STYLE: MainLock<Option<DialogStyle>> = MainLock::new();

/// The look of every `Alert` and `Question`. The default is the iOS
/// system alert, light and dark. An app with its own palette sets one
/// once at startup with `apply_globally`, before any dialog opens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogStyle {
    /// The box behind the message and the buttons.
    pub background:  UIColor,
    /// The message text.
    pub text:        UIColor,
    /// The lines above and between the buttons.
    pub separator:   UIColor,
    /// The text of every button.
    pub button:      UIColor,
    /// The text of a yes button marked with `Question::destructive`.
    pub destructive: UIColor,
}

impl DialogStyle {
    pub const IOS: Self = Self {
        background:  dynamic("#f9f9f9", "#2c2c2e"),
        text:        dynamic("#1c1c1e", "#ffffff"),
        separator:   dynamic("#c6c6c8", "#38383a"),
        button:      dynamic("#007aff", "#0a84ff"),
        destructive: dynamic("#ff3b30", "#ff453a"),
    };

    pub fn apply_globally(self) {
        *STYLE.get_mut() = Some(self);
    }

    /// The style dialogs open with now.
    pub fn current() -> Self {
        STYLE.unwrap_or(Self::IOS)
    }

    /// The test harness gives every test the default look and hands the
    /// app's own style back after the run.
    pub(crate) fn take_global() -> Option<Self> {
        STYLE.get_mut().take()
    }

    pub(crate) fn restore_global(style: Option<&Self>) {
        *STYLE.get_mut() = style.copied();
    }
}

impl Default for DialogStyle {
    fn default() -> Self {
        Self::IOS
    }
}

const fn dynamic(light: &str, dark: &str) -> UIColor {
    UIColor::Dynamic(DynamicColor::new(Color::hex(light), Color::hex(dark)))
}
