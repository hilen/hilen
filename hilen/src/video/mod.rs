//! Video playback, see `docs/video.md`. On desktop ffmpeg demuxes and
//! decodes on a thread, `VideoToolbox`, VAAPI or D3D11VA decodes when the
//! codec allows it, kira plays the sound and its position is the clock the
//! picture follows. In a browser the page plays the source in a `<video>`
//! element behind the canvas, see `web_player.rs`.

#[cfg(desktop)]
mod audio;
#[cfg(any(wasm, test))]
mod cues;
#[cfg(desktop)]
mod decoder;
#[cfg(desktop)]
mod hw;
#[cfg(desktop)]
mod nv12;
#[cfg(desktop)]
mod player;
mod source;
mod state;
#[cfg(desktop)]
mod subtitles;
mod tracks;
#[cfg(wasm)]
mod web_player;

#[cfg(desktop)]
use std::sync::Once;

#[cfg(desktop)]
use log::error;
#[cfg(desktop)]
pub(crate) use player::Player;
pub use source::VideoSource;
pub(crate) use state::PlayerEvent;
pub use state::{VideoState, VideoStats};
pub use tracks::{AudioTrack, SubtitleTrack};
#[cfg(wasm)]
pub(crate) use web_player::{Player, hide_unplaced};

#[cfg(desktop)]
use crate::gm::LossyConvert;

/// ffmpeg's process wide init, once. Warnings only on its log, a broken file
/// reports through `on_error`, not through a wall of stderr.
#[cfg(desktop)]
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
#[cfg(desktop)]
pub(crate) fn count_to_f64(count: u64) -> f64 {
    let count = i64::try_from(count).expect("a media count fits i64");
    count.lossy_convert()
}

/// A fixture of the video UI tests, by file name.
#[cfg(test)]
pub(crate) fn test_fixture(name: &str) -> VideoSource {
    VideoSource::new(format!(
        "{}/../ui-test-suite/src/views/video/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}
