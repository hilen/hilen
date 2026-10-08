use serde::{Deserialize, Serialize};

use crate::{deps::refs::Own, inspect::protocol::ui::ViewRepr};

#[derive(Debug, Serialize, Deserialize)]
pub enum AppCommand {
    Ok,
    Error(String),
    Screenshot {
        width:      u32,
        height:     u32,
        png_base64: String,
    },
    /// The answer to a record, in the order the frames were drawn.
    Frames {
        frames: Vec<FrameRepr>,
        /// What the input said, like the view a hover landed on.
        #[serde(default)]
        note:   Option<String>,
    },
    /// The app is paused, with the frames drawn since the pause.
    Paused {
        frame: u64,
    },
    Edits(Vec<EditEntry>),
    TestResults {
        total:    usize,
        failures: Vec<TestFailureRepr>,
    },
    /// Pushed by a browser test at the moment it fails, since a page has
    /// nowhere to save the frame itself. The driver writes it under
    /// `target/web-test/failures/`.
    FailureScreenshot {
        test:       String,
        png_base64: String,
    },
    /// A log line of a browser page, pushed as it happens so the test
    /// driver prints the app's progress like the desktop lane does and a
    /// stuck run names the last thing the app said.
    Log {
        level:   String,
        message: String,
    },
    /// Unix seconds of when the app's Rust code was compiled, see
    /// `hilen/build.rs`.
    BuildTime(u64),
    /// Unix seconds of when this app process started.
    StartTime(u64),
    /// The library a hot build runs, none for the one packed in the loader.
    Hot {
        library: Option<String>,
    },
    HotFiles(Vec<HotFileRepr>),
    UI(UIResponse),
}

/// A file a hot build already has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotFileRepr {
    /// The path below the listed folder, with `/` between its parts.
    pub path: String,
    pub len:  u64,
    /// `file_hash` of its bytes.
    pub hash: u64,
}

/// The hash both sides of a hot send compare a file by, FNV-1a.
pub fn file_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameRepr {
    /// The count of the frame from the input, the frame of the input is 0.
    pub index:      u32,
    /// Milliseconds from the input to the draw of the frame.
    pub ms:         f32,
    pub width:      u32,
    pub height:     u32,
    pub png_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailureRepr {
    pub name:   String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditEntry {
    pub timestamp: String,
    pub view:      String,
    pub view_id:   String,
    pub what:      String,
    pub old:       String,
    pub new:       String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UIResponse {
    SendUI {
        scale: f32,
        root:  Own<ViewRepr>,
        /// A warning about the request, for example a tap point that is
        /// covered by another view.
        #[serde(default)]
        note:  Option<String>,
    },
}

impl From<UIResponse> for AppCommand {
    fn from(value: UIResponse) -> Self {
        Self::UI(value)
    }
}
