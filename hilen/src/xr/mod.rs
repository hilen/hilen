//! VR through `OpenXR`, behind the `xr` feature: the running scene in a
//! headset, the head and the controllers.
//!
//! An app built with the feature starts VR by itself when a headset is
//! there. Each frame that a scene runs, it is drawn a second time for each
//! eye. Its camera is where the head of the player starts and the way the
//! player faces, so a scene moves the player by moving its camera. The
//! views are not in the headset, they stay in the window.
//!
//! `SteamVR` on Windows is the only system so far, through DX12. Every other
//! system answers `false` from `Xr::running` and `None` from the rest.

#[cfg(windows)]
mod input;
mod pose;
#[cfg(windows)]
mod runtime;

pub use self::pose::{Hand, HandState, Pose};
use crate::deps::refs::main_lock::MainLock;

static STATE: MainLock<State> = MainLock::new();

#[derive(Default)]
struct State {
    running: bool,
    head:    Option<Pose>,
    hands:   [Option<HandState>; 2],
}

/// The headset and its controllers. Every call is for the main thread.
pub struct Xr;

impl Xr {
    /// Whether the headset shows the scene now. False while `SteamVR` waits
    /// for the headset to be put on, and on a system with no VR.
    pub fn running() -> bool {
        STATE.running
    }

    /// The head of the player in the world, read at the last frame.
    pub fn head() -> Option<Pose> {
        STATE.head
    }

    /// A controller in the world, read at the last frame. None while it is
    /// off or out of sight of the tracking.
    pub fn hand(hand: Hand) -> Option<HandState> {
        STATE.hands[hand.index()]
    }
}

#[cfg(windows)]
pub(crate) use self::runtime::{adapter_for, boot, frame, start};
