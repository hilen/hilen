use crate::{
    deps::refs::main_lock::MainLock,
    gm::color::{Color, WHITE},
    ui::{DynamicColor, UIColor},
};

static STYLE: MainLock<Option<BugReportStyle>> = MainLock::new();

/// The colors of the bug report screen. The default is a neutral light
/// and dark pair with a blue Send. An app with its own palette sets one
/// once at startup with `apply_globally`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BugReportStyle {
    /// The whole screen behind the form.
    pub page:           UIColor,
    /// The title, the field text and the key press caption.
    pub text:           UIColor,
    /// Captions, hints, the Cancel text and the close icon.
    pub muted:          UIColor,
    /// The text fields and the key press panel.
    pub panel:          UIColor,
    /// The lines under the title and over the buttons.
    pub line:           UIColor,
    /// A text field while it is edited.
    pub field_selected: UIColor,
    /// The Send button.
    pub accent:         UIColor,
    /// The text of the Send button.
    pub accent_text:    UIColor,
    /// Send while the form is not filled in yet, background and text.
    /// `None` keeps the gray of every disabled `Button`.
    pub disabled_send:  Option<(UIColor, UIColor)>,
    /// The key press checkbox, fill, border and dot. `None` keeps the
    /// look of every `CheckBox`.
    pub check_box:      Option<(UIColor, UIColor, UIColor)>,
}

impl BugReportStyle {
    pub const DEFAULT: Self = Self {
        page:           dynamic("#f7f8fa", "#1e2126"),
        text:           dynamic("#17191d", "#f2f4f7"),
        muted:          dynamic("#5d6570", "#a8b0bc"),
        panel:          dynamic("#ececef", "#2a2e35"),
        line:           dynamic("#e0e2e6", "#343941"),
        field_selected: dynamic("#e6edfa", "#3a4250"),
        accent:         UIColor::Plain(Color::hex("#3c78f0")),
        accent_text:    UIColor::Plain(WHITE),
        disabled_send:  None,
        check_box:      None,
    };

    pub fn apply_globally(self) {
        *STYLE.get_mut() = Some(self);
    }

    /// The style the screen opens with now.
    pub fn current() -> Self {
        STYLE.unwrap_or(Self::DEFAULT)
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

impl Default for BugReportStyle {
    fn default() -> Self {
        Self::DEFAULT
    }
}

const fn dynamic(light: &str, dark: &str) -> UIColor {
    UIColor::Dynamic(DynamicColor::new(Color::hex(light), Color::hex(dark)))
}
