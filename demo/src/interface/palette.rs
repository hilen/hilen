use hilen::ui::{Color, DynamicColor};

// A slate and cyan palette. Every entry carries a light and a dark value
// so the whole demo re-themes in place when the theme changes.

pub const BG: DynamicColor = DynamicColor::new(Color::rgb(0.96, 0.97, 0.99), Color::rgb(0.05, 0.06, 0.09));

pub const SURFACE: DynamicColor = DynamicColor::new(Color::rgb(1.0, 1.0, 1.0), Color::rgb(0.09, 0.11, 0.16));

pub const SURFACE_ALT: DynamicColor =
    DynamicColor::new(Color::rgb(0.92, 0.94, 0.97), Color::rgb(0.13, 0.16, 0.22));

pub const TEXT: DynamicColor = DynamicColor::new(Color::rgb(0.10, 0.12, 0.17), Color::rgb(0.93, 0.95, 0.98));

pub const TEXT_DIM: DynamicColor =
    DynamicColor::new(Color::rgb(0.42, 0.46, 0.52), Color::rgb(0.58, 0.63, 0.71));

pub const BORDER: DynamicColor =
    DynamicColor::new(Color::rgb(0.86, 0.89, 0.93), Color::rgb(0.18, 0.22, 0.30));

// Accents read well on both themes, so they stay the same in each.
pub const ACCENT: DynamicColor = DynamicColor::new(Color::rgb(0.13, 0.55, 0.98), Color::rgb(0.24, 0.62, 1.0));

// A faint accent wash for a selected row or a hovered card.
pub const ACCENT_SOFT: DynamicColor =
    DynamicColor::new(Color::rgb(0.87, 0.93, 1.0), Color::rgb(0.13, 0.20, 0.34));

// Plain colors for gradients. set_gradient is not theme aware, so a
// scene that wants a themed gradient swaps these on theme_changed.
pub const ACCENT_START: Color = Color::rgb(0.13, 0.80, 0.98);
pub const ACCENT_END: Color = Color::rgb(0.55, 0.36, 0.96);
