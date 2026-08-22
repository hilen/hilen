use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::deps::refs::Own;

impl<T: Serialize> Serialize for Own<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        self.deref().serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de> + 'static> Deserialize<'de> for Own<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de> {
        let val = T::deserialize(deserializer)?;

        Ok(Self::new(val))
    }
}
