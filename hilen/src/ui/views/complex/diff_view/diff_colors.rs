use crate::ui::{Color, DynamicColor};

// The colors the diff view draws with, light and dark pairs resolved by
// the theme of the system. kukareker takes them into its palette.

pub const BG: DynamicColor = DynamicColor::new(Color::hex("#f6f6f6"), Color::hex("#11131a"));

pub const FG: DynamicColor = DynamicColor::new(Color::hex("#1c1c1c"), Color::hex("#e4e6eb"));

pub const BG_BAR: DynamicColor = DynamicColor::new(Color::hex("#ededed"), Color::hex("#181b24"));

pub const BG_PANEL: DynamicColor = DynamicColor::new(Color::hex("#f3f3f3"), Color::hex("#1d2130"));

pub const BORDER: DynamicColor = DynamicColor::new(Color::hex("#dcdcdc"), Color::hex("#262a35"));

// The fg at opacity-60 over the panel, the empty state and hint texts.
pub const FG_60: DynamicColor = DynamicColor::new(Color::hex("#727272"), Color::hex("#94979f"));

pub const MUTED: DynamicColor = DynamicColor::new(Color::hex("#6e6e6e"), Color::hex("#7a7f8a"));

pub const HOVER: DynamicColor = DynamicColor::new(Color::hex("#f1f1f1"), Color::hex("#1c1f29"));

// Tailwind emerald-500 and red-500, the +N and -M diff stats.
pub const STAT_ADD: Color = Color::hex("#10b981");
pub const STAT_DEL: Color = Color::hex("#ef4444");
