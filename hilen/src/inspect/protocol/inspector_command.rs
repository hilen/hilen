use serde::{Deserialize, Serialize};

use crate::{
    gm::color::Color,
    ui::{ModifiersState, NamedKey},
    window::KeyCode,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InspectorCommand {
    PlaySound,
    Screenshot,
    ListEdits,
    RunTests,
    /// Runs only the tests of a comma separated list, in either spelling,
    /// like `run-tests --only` asks. The phone lane of a release uses it.
    RunTestsNamed {
        names: String,
    },
    GetBuildTime,
    GetStartTime,
    /// Logs the app into a dev account of its backend, with no password,
    /// see `Login::dev`. The answer is `LoggedIn`. An app with no `login`
    /// feature answers with an error.
    DevLogin {
        email: String,
    },
    /// Closes the app the way closing its window does. Desktop only, a
    /// phone or a browser page has no such exit.
    Quit,
    /// Saves every frame the app draws after an input, `frames` of them,
    /// the first one is the frame that holds what the input did. With an
    /// input, a tap, keys, a hover, a scroll or a drag, the app plays it
    /// and records. With none the record waits up to `wait_ms` for the
    /// next press, key or wheel turn of the user.
    Record {
        frames:  u32,
        input:   Option<UIRequest>,
        #[serde(default)]
        wait_ms: u32,
    },
    /// Starts a record of every frame the app draws. It runs until
    /// `RecordStop` and keeps its last 10 seconds, each frame at 1 pixel
    /// per point with no loss. No frame is left out: when the app cannot
    /// pack fast enough it waits, and the frame says so. It drops the
    /// record kept before it.
    RecordStart,
    /// Stops the running record. The app keeps its frames until the next
    /// start.
    RecordStop,
    /// Asks for the list of the kept frames, with no pictures.
    RecordInfo,
    /// Asks for the pictures of the kept frames from the index `from`
    /// on, at most `count` of them. The answer is `KeptFrames`.
    RecordFrames {
        from:  u32,
        count: u32,
    },
    /// Freezes the engine clock and the drawing until `Resume`.
    Pause,
    /// Draws the next frames of a paused app, each moves the clock by one
    /// frame of 60 a second.
    Step {
        frames: u32,
    },
    Resume,
    /// Starts a run of the speed test on this connection. With `to_app`
    /// the client then sends bytes and the app counts them, else the app
    /// sends bytes for `ms`. The bytes are no JSON, they go in pieces
    /// behind their length and a length of 0 ends the run. The answer
    /// after them is `Speed`.
    Speed {
        to_app: bool,
        ms:     u32,
    },
    /// Asks a hot build which library runs, see `docs/hot-reload.md`. Every
    /// other build answers with an error.
    HotInfo,
    /// Lists the files a hot build has under `root`, a folder below its hot
    /// folder, so only the changed assets of an app are sent.
    HotFiles {
        root: String,
    },
    /// A piece of a file for the hot folder, a library or an asset. The
    /// piece at offset 0 starts the file again.
    HotChunk {
        /// The path below the hot folder.
        file:        String,
        offset:      u64,
        data_base64: String,
    },
    /// The same piece with no base64: its `len` bytes follow this command
    /// on the connection as they are, and the transport puts them into
    /// `data`. The sender does not wait for the answer before the next
    /// piece. As base64 the 45 MB library of an app went at 12.7 MB a
    /// second over a link that carries 45.
    HotBytes {
        file:   String,
        offset: u64,
        len:    u32,
        #[serde(skip)]
        data:   Vec<u8>,
    },
    /// Makes the loader swap to a library that was sent. `root` is the
    /// folder below the hot folder that holds the `assets` of the app.
    HotSwap {
        library: String,
        root:    Option<String>,
    },
    /// A command of the app itself, see `hilen::app_commands`. The answer
    /// is `Answer` with its text.
    App {
        name: String,
        args: Vec<String>,
    },
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
        /// Send the touch although another view covers the tap point.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        force:     bool,
    },
    /// Move the cursor to a view center, or outside the window with no view.
    /// Waits on the inspector worker so delayed tooltips can appear before
    /// replying.
    Hover {
        view_id: Option<String>,
        #[serde(default)]
        wait_ms: u32,
    },
    /// Plays the keys in order with the modifiers held, then releases them.
    Keys {
        keys:      Vec<Key>,
        modifiers: ModifiersState,
    },
    /// Holds physical keys down for `ms`, as `Keys::held` reads them,
    /// then releases them. For input read every frame, like walking.
    Hold {
        keys: Vec<KeyCode>,
        ms:   u32,
    },
    /// A left drag from one window point to another: began, moved steps
    /// and ended through the real input pipeline. Window coordinates in
    /// points. For drag driven behavior like selecting text. Above 0,
    /// `step_ms` is the time between 2 touches, 0 sends all at once.
    Drag {
        from:    (f32, f32),
        to:      (f32, f32),
        #[serde(default)]
        steps:   usize,
        #[serde(default)]
        step_ms: u32,
    },
    /// A wheel scroll aimed at a view's center, or at the window center
    /// with no view. It lands on the deepest scroll view under that
    /// point, like a real wheel under a still cursor.
    Scroll {
        view_id: Option<String>,
        dx:      f32,
        dy:      f32,
    },
    /// Resizes the window, in points. Desktop only.
    Resize {
        width:  f32,
        height: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Key {
    Char(char),
    Named(NamedKey),
}
