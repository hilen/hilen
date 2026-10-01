use std::fmt::Debug;

use reflected::{Field, Type};
use zeroize::Zeroizing;

use crate::wipe::filtered;

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

    /// The chars of `text` this constraint accepts. The copy never grows
    /// and wipes itself when dropped, the text may be that of a secure
    /// field.
    pub(crate) fn filter(&self, text: &str) -> Zeroizing<String> {
        let symbols = self.accepted_symbols();
        filtered(text, |char| symbols.contains(char))
    }

    fn accepted_symbols(&self) -> &str {
        match self {
            Self::Integer => "-0123456789",
            Self::Float => "-0.123456789",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TextFieldConstraint;

    #[test]
    fn a_constraint_keeps_only_its_own_chars() {
        assert_eq!(TextFieldConstraint::Integer.filter("-12a.5 6").as_str(), "-1256");
        assert_eq!(TextFieldConstraint::Float.filter("-12a.5 6").as_str(), "-12.56");
        assert_eq!(TextFieldConstraint::Integer.filter("").as_str(), "");
    }
}
