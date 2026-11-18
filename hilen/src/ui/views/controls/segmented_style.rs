use crate::{
    deps::refs::main_lock::MainLock,
    gm::color::Color,
    ui::{DynamicColor, UIColor},
};

static STYLE: MainLock<Option<SegmentedStyle>> = MainLock::new();

/// The look of every `SegmentedControl`. The default is a gray track with a
/// lighter picked segment, light and dark. An app with its own palette sets
/// one once at startup with `apply_globally`, before any control is built.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentedStyle {
    /// The rounded track behind the segments.
    pub track:       UIColor,
    /// The picked segment.
    pub picked:      UIColor,
    /// The text of the picked segment.
    pub picked_text: UIColor,
}

impl SegmentedStyle {
    pub const DEFAULT: Self = Self {
        track:       dynamic("#EBEBF0", "#2C2C30"),
        picked:      dynamic("#FFFFFF", "#5A5A60"),
        picked_text: dynamic("#1C1C1E", "#FFFFFF"),
    };

    pub fn apply_globally(self) {
        *STYLE.get_mut() = Some(self);
    }

    /// The style a segmented control is built with now.
    pub fn current() -> Self {
        STYLE.unwrap_or(Self::DEFAULT)
    }

    /// The test harness gives every test the default look and hands the
    /// app's own style back after the run.
    #[cfg(feature = "gpu")]
    pub(crate) fn take_global() -> Option<Self> {
        STYLE.get_mut().take()
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn restore_global(style: Option<&Self>) {
        *STYLE.get_mut() = style.copied();
    }
}

impl Default for SegmentedStyle {
    fn default() -> Self {
        Self::DEFAULT
    }
}

const fn dynamic(light: &str, dark: &str) -> UIColor {
    UIColor::Dynamic(DynamicColor::new(Color::hex(light), Color::hex(dark)))
}
