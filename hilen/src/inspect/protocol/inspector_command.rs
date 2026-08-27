use serde::{Deserialize, Serialize};

use crate::{
    gm::color::Color,
    ui::{ModifiersState, NamedKey},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InspectorCommand {
    PlaySound,
    Screenshot,
    ListEdits,
    RunTests,
    GetBuildTime,
    GetStartTime,
    UI(UIRequest),
}

impl From<UIRequest> for InspectorCommand {
    fn from(value: UIRequest) -> Self {
        Self::UI(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UIRequest {
    SetScale(f32),
    GetUI,
    EditRule {
        view_id:    String,
        rule_index: usize,
        offset:     f32,
        enabled:    bool,
    },
    SetText {
        view_id: String,
        text:    String,
    },
    SetColor {
        view_id: String,
        color:   Color,
    },
    /// Touch began plus ended at the view center, with the modifiers held
    /// only for the tap, so a Cmd click drives multi selection. A right
    /// tap fires the secondary action, context menus open like from a
    /// real mouse.
    Tap {
        view_id:   String,
        #[serde(default)]
        modifiers: ModifiersState,
        #[serde(default)]
        right:     bool,
    },
    /// Plays the keys in order with the modifiers held, then releases them.
    Keys {
        keys:      Vec<Key>,
        modifiers: ModifiersState,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Key {
    Char(char),
    Named(NamedKey),
}
