use hilen::ui::{Color, DynamicColor};

// A calm slate and cyan palette. Every entry carries a light and a dark
// value so the whole demo re-themes in place when the toggle flips.

pub const BG: DynamicColor = DynamicColor::new(Color::rgb(0.95, 0.96, 0.98), Color::rgb(0.07, 0.09, 0.13));

pub const SURFACE: DynamicColor = DynamicColor::new(Color::rgb(1.0, 1.0, 1.0), Color::rgb(0.13, 0.16, 0.22));

pub const SURFACE_ALT: DynamicColor =
    DynamicColor::new(Color::rgb(0.91, 0.93, 0.97), Color::rgb(0.17, 0.21, 0.28));

pub const TEXT: DynamicColor = DynamicColor::new(Color::rgb(0.11, 0.13, 0.18), Color::rgb(0.93, 0.95, 0.98));

pub const TEXT_DIM: DynamicColor =
    DynamicColor::new(Color::rgb(0.42, 0.46, 0.52), Color::rgb(0.60, 0.65, 0.72));

pub const BORDER: DynamicColor =
    DynamicColor::new(Color::rgb(0.85, 0.88, 0.92), Color::rgb(0.24, 0.28, 0.36));

// Accents read well on both themes, so they stay the same in each.
pub const ACCENT: DynamicColor = DynamicColor::new(Color::rgb(0.13, 0.55, 0.98), Color::rgb(0.24, 0.62, 1.0));

// Plain colors for gradients. set_gradient is not theme aware, so a
// scene that wants a themed gradient swaps these on theme_changed.
pub const ACCENT_START: Color = Color::rgb(0.24, 0.62, 1.0);
pub const ACCENT_END: Color = Color::rgb(0.55, 0.36, 0.96);
