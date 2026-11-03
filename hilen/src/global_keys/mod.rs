//! A key combination that works in every app while this app runs, see
//! `docs/global-keys.md`. macOS has it, the other systems answer with an
//! error for now.

#[cfg(macos)]
mod mac;
#[cfg(test)]
mod tests;

use std::{fmt, str::FromStr};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize, de::value::StrDeserializer};
use winit::keyboard::KeyCode;

/// A key with the modifiers held with it, like Cmd+Shift+P. The text form
/// is what `Display` writes and `FromStr` reads, the way a settings file
/// keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlobalCombo {
    pub key:     KeyCode,
    /// Cmd on a Mac, the Windows key elsewhere.
    pub command: bool,
    pub shift:   bool,
    pub control: bool,
    /// Option on a Mac, Alt elsewhere.
    pub option:  bool,
}

impl GlobalCombo {
    pub const fn new(key: KeyCode) -> Self {
        Self {
            key,
            command: false,
            shift: false,
            control: false,
            option: false,
        }
    }

    pub const fn command(mut self) -> Self {
        self.command = true;
        self
    }

    pub const fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub const fn control(mut self) -> Self {
        self.control = true;
        self
    }

    pub const fn option(mut self) -> Self {
        self.option = true;
        self
    }

    fn has_modifier(self) -> bool {
        self.command || self.shift || self.control || self.option
    }

    /// The name of the key as winit writes it, like `KeyP` or `Digit2`.
    fn code_name(self) -> String {
        format!("{:?}", self.key)
    }
}

impl fmt::Display for GlobalCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        for (held, name) in [
            (self.control, "Ctrl"),
            (self.option, "Option"),
            (self.shift, "Shift"),
            (self.command, "Cmd"),
        ] {
            if held {
                parts.push(name.to_string());
            }
        }
        let code = self.code_name();
        let key = code
            .strip_prefix("Key")
            .or_else(|| code.strip_prefix("Digit"))
            .unwrap_or(&code)
            .to_string();
        parts.push(key);
        write!(f, "{}", parts.join("+"))
    }
}

impl FromStr for GlobalCombo {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let Some(key) = parts.pop().filter(|key| !key.is_empty()) else {
            bail!("No key in {text:?}");
        };
        let mut combo = Self::new(key_code(key)?);
        for part in parts {
            match part.to_lowercase().as_str() {
                "cmd" | "command" | "super" | "win" => combo.command = true,
                "shift" => combo.shift = true,
                "ctrl" | "control" => combo.control = true,
                "option" | "opt" | "alt" => combo.option = true,
                other => bail!("{other:?} in {text:?} is no modifier"),
            }
        }
        Ok(combo)
    }
}

impl Serialize for GlobalCombo {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for GlobalCombo {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// `P` or `KeyP`, `2` or `Digit2`, or any other winit name like `Space`.
fn key_code(name: &str) -> Result<KeyCode> {
    let mut chars = name.chars();
    let name = match (chars.next(), chars.next()) {
        (Some(letter), None) if letter.is_ascii_alphabetic() => format!("Key{}", letter.to_ascii_uppercase()),
        (Some(digit), None) if digit.is_ascii_digit() => format!("Digit{digit}"),
        _ => name.to_string(),
    };
    KeyCode::deserialize(StrDeserializer::<serde::de::value::Error>::new(&name))
        .map_err(|err| anyhow::anyhow!("{name:?} is no key: {err}"))
}

/// A key combination registered with the system. `action` runs on the main
/// thread in the next turn of the loop each time the combination is pressed,
/// in any app. The combination is given back when this is dropped.
pub struct GlobalKey {
    combo: GlobalCombo,
    #[cfg(macos)]
    inner: mac::Registered,
}

impl GlobalKey {
    pub fn register(combo: GlobalCombo, action: impl FnMut() + 'static) -> Result<Self> {
        crate::deps::hreads::assert_main_thread();
        if !combo.has_modifier() {
            bail!("{combo} has no modifier, it would take the key away from every app");
        }
        #[cfg(macos)]
        {
            let inner = mac::Registered::new(combo, Box::new(action))?;
            log::info!("Global key {combo} registered");
            Ok(Self { combo, inner })
        }
        #[cfg(not(macos))]
        {
            drop(action);
            bail!("Global keys are on macOS only for now, see docs/global-keys.md")
        }
    }

    pub fn combo(&self) -> GlobalCombo {
        self.combo
    }
}

impl fmt::Debug for GlobalKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlobalKey").field("combo", &self.combo).finish_non_exhaustive()
    }
}

#[cfg(macos)]
impl Drop for GlobalKey {
    fn drop(&mut self) {
        log::info!("Global key {} given back", self.combo);
        self.inner.unregister();
    }
}
