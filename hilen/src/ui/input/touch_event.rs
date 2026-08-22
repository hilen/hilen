use std::{fmt::Display, str::FromStr};

use anyhow::bail;

use crate::window::ElementState;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TouchEvent {
    Began,
    Moved,
    Ended,
}

impl Display for TouchEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TouchEvent::Began => "b",
                TouchEvent::Moved => "m",
                TouchEvent::Ended => "e",
            }
        )
    }
}

impl FromStr for TouchEvent {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "b" => Ok(TouchEvent::Began),
            "m" => Ok(TouchEvent::Moved),
            "e" => Ok(TouchEvent::Ended),
            _ => bail!("Failed to parse TouchEvent from {s}"),
        }
    }
}

impl From<ElementState> for TouchEvent {
    fn from(value: ElementState) -> Self {
        match value {
            ElementState::Pressed => Self::Began,
            ElementState::Released => Self::Ended,
        }
    }
}
