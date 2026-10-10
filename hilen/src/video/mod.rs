//! Video playback, see `docs/video.md`. On desktop and on iOS ffmpeg demuxes
//! and decodes on a thread, `VideoToolbox`, VAAPI or D3D11VA decodes when the
//! codec allows it, kira plays the sound and its position is the clock the
//! picture follows. In a browser the page plays the source in a `<video>`
//! element behind the canvas, see `web_player.rs`.

#[cfg(ffmpeg)]
mod audio;
#[cfg(ffmpeg)]
mod audio_session;
#[cfg(any(wasm, test))]
mod cues;
#[cfg(ffmpeg)]
mod decoder;
#[cfg(ffmpeg)]
mod export;
#[cfg(ffmpeg)]
mod frames;
#[cfg(ffmpeg)]
mod hw;
#[cfg(ffmpeg)]
mod levels;
#[cfg(ffmpeg)]
mod nv12;
#[cfg(ffmpeg)]
mod player;
mod source;
mod state;
#[cfg(ffmpeg)]
mod subtitles;
#[cfg(all(test, ffmpeg))]
mod test_server;
mod tracks;
#[cfg(wasm)]
mod web_player;

#[cfg(ffmpeg)]
use std::sync::Once;

#[cfg(ffmpeg)]
pub use export::{VideoExport, VideoExportEvent, VideoExportSettings};
#[cfg(ffmpeg)]
pub use frames::{VideoFrames, VideoFramesEvent, VideoFramesLoad, VideoInfo, VideoPicture};
#[cfg(ffmpeg)]
pub use levels::{VideoLevels, VideoLevelsEvent, VideoLevelsLoad};
#[cfg(ffmpeg)]
use log::error;
#[cfg(ffmpeg)]
pub(crate) use player::Player;
#[cfg(ffmpeg)]
pub use source::VideoPiece;
pub use source::VideoSource;
pub(crate) use state::PlayerEvent;
pub use state::{VideoState, VideoStats};
pub use tracks::{AudioTrack, SubtitleTrack};
#[cfg(wasm)]
pub(crate) use web_player::{Player, hide_unplaced};

#[cfg(ffmpeg)]
use crate::gm::LossyConvert;

/// ffmpeg's process wide init, once. Warnings only on its log, a broken file
/// reports through `on_error`, not through a wall of stderr.
#[cfg(ffmpeg)]
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
#[cfg(ffmpeg)]
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
