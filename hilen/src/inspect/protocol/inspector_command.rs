use serde::{Deserialize, Serialize};

use crate::gm::color::Color;

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
    Tap {
        view_id: String,
    },
}
