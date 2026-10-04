// The desktop tests pin decoded frames, ffmpeg tracks and the sound
// clock, none of which a browser has.
#[cfg(desktop)]
mod av1;
#[cfg(desktop)]
mod hdr;
#[cfg(desktop)]
mod playback;
#[cfg(desktop)]
mod slow_sound;
#[cfg(desktop)]
mod speed;
#[cfg(desktop)]
mod stalled_seek;
#[cfg(desktop)]
mod stream;
#[cfg(desktop)]
mod tracks;
/// A browser plays a video in a `<video>` element under the canvas, which
/// exists nowhere else.
#[cfg(wasm)]
mod web_video;
