use crate::{
    deps::refs::main_lock::MainLock,
    gm::{
        ToF32,
        color::{Color, U8Color},
    },
    ui::{UIEvents, UIManager, View, WeakView, view::ViewSubviews},
    window::Theme as OsTheme,
};

static THEME: MainLock<ThemeState> = MainLock::new();

#[derive(Default)]
struct ThemeState {
    mode:   ThemeMode,
    system: Theme,
}

/// Light or dark look. The effective theme is picked by `ThemeMode`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

/// How the effective theme is chosen. `System` follows the OS.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn current() -> Theme {
        match THEME.mode {
            ThemeMode::System => THEME.system,
            ThemeMode::Light => Theme::Light,
            ThemeMode::Dark => Theme::Dark,
        }
    }

    pub fn mode() -> ThemeMode {
        THEME.mode
    }

    /// Force light or dark, or follow the OS with `ThemeMode::System`.
    pub fn set_mode(mode: ThemeMode) {
        let before = Self::current();
        THEME.get_mut().mode = mode;
        Self::on_change(before);
    }

    /// The OS theme. The engine calls this on startup and on the system
    /// theme change event. Takes effect only in `ThemeMode::System`.
    pub fn set_system(theme: Theme) {
        let before = Self::current();
        THEME.get_mut().system = theme;
        Self::on_change(before);
    }

    fn on_change(before: Theme) {
        if Self::current() == before {
            return;
        }

        reapply_theme(UIManager::root_view().weak_view());
        UIManager::reapply_clear_color();
        UIEvents::theme_changed().trigger(Self::current());
    }
}

impl From<OsTheme> for Theme {
    fn from(theme: OsTheme) -> Self {
        match theme {
            OsTheme::Light => Self::Light,
            OsTheme::Dark => Self::Dark,
        }
    }
}

fn reapply_theme(mut view: WeakView) {
    let base = view.__base_view();

    if let Some(color) = base.dynamic_color {
        base.color = color.resolve();
    }

    if let Some(color) = base.dynamic_border_color {
        base.border_color = color.resolve();
    }

    view.theme_changed();

    for subview in view.subviews_weak() {
        reapply_theme(subview);
    }
}

/// A pair of colors, one per theme. A view keeps the pair next to the
/// resolved color, so a theme switch can re-resolve it in place. The
/// draw path still reads plain resolved colors and pays nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicColor {
    pub light: Color,
    pub dark:  Color,
}

impl DynamicColor {
    pub const fn new(light: Color, dark: Color) -> Self {
        Self { light, dark }
    }

    pub fn resolve(&self) -> Color {
        match Theme::current() {
            Theme::Light => self.light,
            Theme::Dark => self.dark,
        }
    }
}

/// What color setters accept: a plain color applied once, or a dynamic
/// pair re-resolved on every theme switch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UIColor {
    Plain(Color),
    Dynamic(DynamicColor),
}

impl Default for UIColor {
    fn default() -> Self {
        Self::Plain(Color::default())
    }
}

impl From<Color> for UIColor {
    fn from(color: Color) -> Self {
        Self::Plain(color)
    }
}

impl From<DynamicColor> for UIColor {
    fn from(color: DynamicColor) -> Self {
        Self::Dynamic(color)
    }
}

impl From<&str> for UIColor {
    fn from(hex: &str) -> Self {
        Self::Plain(hex.into())
    }
}

impl From<U8Color> for UIColor {
    fn from(color: U8Color) -> Self {
        Self::Plain(color.into())
    }
}

impl<R: ToF32, G: ToF32, B: ToF32> From<(R, G, B)> for UIColor {
    fn from(rgb: (R, G, B)) -> Self {
        Self::Plain(rgb.into())
    }
}
