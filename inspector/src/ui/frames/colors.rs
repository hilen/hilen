use hilen::{gm::color::Color, ui::DynamicColor};

/// What the frames screen says about a frame with a color.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Mark {
    /// Nothing to say. A picture that stays on screen is normal for a
    /// screen where nothing moves, so a long stay has no color.
    #[default]
    Plain,
    /// The picture stayed because the app waited for the record, the
    /// record itself made that gap.
    Waited,
}

/// The selected frame.
pub const SELECTED: DynamicColor = DynamicColor::new(Color::hex("#1F6FEB"), Color::hex("#58A6FF"));
pub const WAITED: DynamicColor = DynamicColor::new(Color::hex("#BC4C00"), Color::hex("#F0883E"));
/// 2 grays that take turns in the time line, so the frames can be told
/// apart.
pub const EVEN: DynamicColor = DynamicColor::new(Color::hex("#D0D7DE"), Color::hex("#484F58"));
pub const ODD: DynamicColor = DynamicColor::new(Color::hex("#8C959F"), Color::hex("#6E7681"));

impl Mark {
    /// The color of a frame in the time line, `place` is its place in the
    /// record.
    pub fn time_line(self, place: usize) -> DynamicColor {
        match self {
            Self::Waited => WAITED,
            Self::Plain if place.is_multiple_of(2) => EVEN,
            Self::Plain => ODD,
        }
    }
}
