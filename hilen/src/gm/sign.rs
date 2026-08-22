use std::fmt::Display;

#[derive(Default, Debug)]
pub enum Sign {
    #[default]
    Plus,
    Minus,
}

impl Sign {
    pub fn multiplier(&self) -> f32 {
        match self {
            Self::Plus => 1.0,
            Self::Minus => -1.0,
        }
    }
}

impl Display for Sign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Plus => '+',
                Self::Minus => '-',
            }
        )
    }
}
