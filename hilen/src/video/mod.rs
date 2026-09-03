//! Video playback, see `docs/video.md`. Desktop only for now. ffmpeg demuxes
//! and decodes on a thread, `VideoToolbox`, VAAPI or D3D11VA decodes when the
//! codec allows it, kira plays the sound and its position is the clock the
//! picture follows.

mod audio;
mod decoder;
mod hw;
mod nv12;
mod player;

use std::sync::Once;

use log::error;
pub use player::VideoStats;
pub(crate) use player::{Player, PlayerEvent};

use crate::gm::LossyConvert;

/// ffmpeg's process wide init, once. Warnings only on its log, a broken file
/// reports through `on_error`, not through a wall of stderr.
pub(crate) fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if let Err(err) = ffmpeg_next::init() {
            error!("ffmpeg init failed: {err}");
        }
        ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Warning);
    });
}

/// A frame or sample count as seconds math input. Counts stay far below
/// 2^53, so nothing is lost.
pub(crate) fn count_to_f64(count: u64) -> f64 {
    let count = i64::try_from(count).expect("a media count fits i64");
    count.lossy_convert()
}
