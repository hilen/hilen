use std::fmt::Debug;

use reflected::{Field, Type};

use crate::ui::ToLabel;

#[derive(Debug)]
pub enum TextFieldConstraint {
    Integer,
    Float,
}

impl TextFieldConstraint {
    pub fn from_field<T: Send>(field: &Field<T>) -> Option<Self> {
        if matches!(field.tp, Type::Integer) {
            Self::Integer.into()
        } else if matches!(field.tp, Type::Float) {
            Self::Float.into()
        } else {
            None
        }
    }

    pub(crate) fn filter(&self, string: impl ToLabel) -> String {
        let string = string.to_label();
        let symbols = self.accepted_symbols(&string);
        string.clone().chars().filter(|c| symbols.contains(*c)).collect()
    }

    pub(crate) fn accept_char(&self, char: char, string: &str) -> bool {
        self.accepted_symbols(string).contains(char)
    }

    fn accepted_symbols(&self, _str: &str) -> &str {
        match self {
            Self::Integer => "-0123456789",
            Self::Float => "-0.123456789",
        }
    }
}

pub(crate) trait AcceptChar {
    fn accept_char(&self, char: char, string: &str) -> bool;
}

impl AcceptChar for Option<TextFieldConstraint> {
    fn accept_char(&self, char: char, string: &str) -> bool {
        match self {
            Some(constraint) => constraint.accept_char(char, string),
            None => true,
        }
    }
}
