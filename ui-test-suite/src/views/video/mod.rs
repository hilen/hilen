// The ffmpeg tests pin decoded frames, ffmpeg tracks and the sound
// clock, none of which a browser has.
#[cfg(ffmpeg)]
mod av1;
#[cfg(ffmpeg)]
mod hdr;
#[cfg(ffmpeg)]
mod playback;
#[cfg(ffmpeg)]
mod slow_sound;
#[cfg(ffmpeg)]
mod speed;
#[cfg(ffmpeg)]
mod stalled_seek;
#[cfg(ffmpeg)]
mod stream;
#[cfg(ffmpeg)]
mod tracks;
/// A browser plays a video in a `<video>` element under the canvas, which
/// exists nowhere else.
#[cfg(wasm)]
mod web_video;
