use crate::{
    deps::refs::main_lock::MainLock,
    gm::color::Color,
    ui::{DynamicColor, UIColor},
};

static STYLE: MainLock<Option<DateFieldStyle>> = MainLock::new();

/// The look of every `DateField` and `TimeField`, light and dark. An app
/// with its own palette sets one once at startup with `apply_globally`,
/// before any field is built.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DateFieldStyle {
    /// The box of the field and of the open calendar.
    pub fill:        UIColor,
    /// The border of the box.
    pub border:      UIColor,
    /// The border while the typed text is no date or no time.
    pub bad_border:  UIColor,
    /// The typed text, the month and the days.
    pub text:        UIColor,
    /// The names of the weekdays over the days.
    pub faint:       UIColor,
    /// The text of the Calendar, Previous and Next buttons and of the days.
    pub button_text: UIColor,
    /// The fill of a button or a day under the pointer.
    pub hover:       UIColor,
    /// The fill of the picked day.
    pub picked:      UIColor,
    /// The number of the picked day.
    pub picked_text: UIColor,
}

impl DateFieldStyle {
    pub const DEFAULT: Self = Self {
        fill:        dynamic("#FFFFFF", "#1C1C1E"),
        border:      dynamic("#D1D1D6", "#48484E"),
        bad_border:  dynamic("#FF3B30", "#FF453A"),
        text:        dynamic("#1C1C1E", "#F2F2F4"),
        faint:       dynamic("#8E8E93", "#8E8E93"),
        button_text: dynamic("#1C1C1E", "#F2F2F4"),
        hover:       dynamic("#ECECF0", "#323236"),
        picked:      dynamic("#E1ECFF", "#1F3352"),
        picked_text: dynamic("#0A6CFF", "#5AA5FF"),
    };

    pub fn apply_globally(self) {
        *STYLE.get_mut() = Some(self);
    }

    /// The style a date or time field is built with now.
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

impl Default for DateFieldStyle {
    fn default() -> Self {
        Self::DEFAULT
    }
}

const fn dynamic(light: &str, dark: &str) -> UIColor {
    UIColor::Dynamic(DynamicColor::new(Color::hex(light), Color::hex(dark)))
}
