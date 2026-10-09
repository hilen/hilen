use super::ansi::AnsiColor;
use crate::{
    deps::refs::Weak,
    gm::color::Color,
    ui::{DynamicColor, UIColor},
    window::Font,
};

/// The look of a `LogView`, set with `LogView::set_style`. Every default
/// color is a light and dark pair.
#[derive(Clone, Copy)]
pub struct LogStyle {
    pub text_size:   f32,
    /// The font of the lines, `Font::mono()` when none.
    pub font:        Option<Weak<Font>>,
    /// The font of bold text, `Font::mono_bold()` when none.
    pub bold_font:   Option<Weak<Font>>,
    /// The space between the edges of the view and the lines.
    pub padding:     f32,
    /// More space between the prefix column and the text.
    pub prefix_gap:  f32,
    pub background:  UIColor,
    /// The text of a line with no color code.
    pub text:        UIColor,
    /// The prefix of a line with no color code.
    pub prefix:      UIColor,
    /// The 16 ANSI colors: black, red, green, yellow, blue, magenta,
    /// cyan, white, then the bright 8 in the same order.
    pub colors:      [UIColor; 16],
    /// The round button that jumps to the end.
    pub button:      UIColor,
    pub button_icon: UIColor,
}

impl LogStyle {
    pub const DEFAULT: Self = Self {
        text_size:   12.0,
        font:        None,
        bold_font:   None,
        padding:     4.0,
        prefix_gap:  0.0,
        background:  dynamic("#f6f8fa", "#0d1117"),
        text:        dynamic("#24292f", "#e6edf3"),
        prefix:      dynamic("#6e7781", "#8b949e"),
        colors:      [
            dynamic("#24292f", "#484f58"),
            dynamic("#cf222e", "#ff7b72"),
            dynamic("#116329", "#3fb950"),
            dynamic("#9a6700", "#d29922"),
            dynamic("#0969da", "#58a6ff"),
            dynamic("#8250df", "#bc8cff"),
            dynamic("#1b7c83", "#39c5cf"),
            dynamic("#6e7781", "#b1bac4"),
            dynamic("#57606a", "#6e7681"),
            dynamic("#a40e26", "#ffa198"),
            dynamic("#1a7f37", "#56d364"),
            dynamic("#7d4e00", "#e3b341"),
            dynamic("#218bff", "#79c0ff"),
            dynamic("#a475f9", "#d2a8ff"),
            dynamic("#3192aa", "#56d4dd"),
            dynamic("#8c959f", "#ffffff"),
        ],
        button:      dynamic("#ffffff", "#30363d"),
        button_icon: dynamic("#57606a", "#c9d1d9"),
    };

    pub(super) fn regular_font(&self) -> Weak<Font> {
        self.font.unwrap_or_else(Font::mono)
    }

    pub(super) fn strong_font(&self) -> Weak<Font> {
        self.bold_font.unwrap_or_else(Font::mono_bold)
    }

    pub(super) fn color(&self, color: AnsiColor) -> UIColor {
        match color {
            AnsiColor::Palette(index) => match self.colors.get(usize::from(index)) {
                Some(color) => *color,
                None => UIColor::Plain(table_color(index)),
            },
            AnsiColor::Rgb(red, green, blue) => UIColor::Plain(rgb(red, green, blue)),
        }
    }
}

impl Default for LogStyle {
    fn default() -> Self {
        Self::DEFAULT
    }
}

const fn dynamic(light: &str, dark: &str) -> UIColor {
    UIColor::Dynamic(DynamicColor::new(Color::hex(light), Color::hex(dark)))
}

fn rgb(red: u8, green: u8, blue: u8) -> Color {
    Color::rgb(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
    )
}

/// The colors 16 to 255 of the table every terminal has: a cube of 6
/// steps per channel, then 24 grays.
fn table_color(index: u8) -> Color {
    const STEPS: [u8; 6] = [0, 95, 135, 175, 215, 255];

    if index >= 232 {
        let gray = 8 + (index - 232) * 10;
        return rgb(gray, gray, gray);
    }
    let cube = usize::from(index.saturating_sub(16));
    rgb(STEPS[cube / 36], STEPS[cube / 6 % 6], STEPS[cube % 6])
}

/// A color at the strength of dim text.
pub(super) fn dimmed(color: UIColor) -> UIColor {
    const DIM: f32 = 0.6;

    let dim = |color: Color| color.with_alpha(color.a * DIM);
    match color {
        UIColor::Plain(color) => UIColor::Plain(dim(color)),
        UIColor::Dynamic(pair) => UIColor::Dynamic(DynamicColor::new(dim(pair.light), dim(pair.dark))),
    }
}

#[cfg(test)]
mod tests {
    use super::{rgb, table_color};

    #[test]
    fn the_table_of_256_colors() {
        assert_eq!(table_color(16), rgb(0, 0, 0));
        assert_eq!(table_color(196), rgb(255, 0, 0));
        assert_eq!(table_color(208), rgb(255, 135, 0));
        assert_eq!(table_color(231), rgb(255, 255, 255));
        assert_eq!(table_color(232), rgb(8, 8, 8));
        assert_eq!(table_color(255), rgb(238, 238, 238));
    }
}
