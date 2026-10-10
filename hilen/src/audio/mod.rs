// mod android_sound;
// use android_sound as sound;
mod clock;
mod clock_feed;
#[cfg(all(test, not_wasm))]
mod clock_feed_test;
mod clock_sound;
#[cfg(test)]
mod clock_test;
pub(crate) mod manager;
mod sound;

pub use self::{
    clock::SoundClock,
    sound::{Playing, Sound},
};
use crate::managed;

managed!(Sound);
