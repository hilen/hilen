//! The audio session of a phone while a video with sound plays. With the
//! session iOS gives an app by default the silent switch mutes all sound,
//! which is right for a click and wrong for a film. So the first video that
//! starts a sound moves the session to playback, and the last one that goes
//! puts the default back. Every other platform has no such session and only
//! the count runs there.

use log::info;

use crate::deps::{
    hreads::{assert_main_thread, on_main},
    refs::main_lock::MainLock,
};

static HOLDERS: MainLock<Holders> = MainLock::new();

/// Keeps the session at playback while it lives. Several videos may hold
/// one each. Acquire on the main thread, dropping releases it.
pub(crate) struct PlaybackSession;

impl PlaybackSession {
    pub(crate) fn acquire() -> Self {
        assert_main_thread();
        apply(HOLDERS.get_mut().add());
        Self
    }
}

impl Drop for PlaybackSession {
    fn drop(&mut self) {
        on_main(|| apply(HOLDERS.get_mut().remove()));
    }
}

/// How many videos with sound there are.
#[derive(Default)]
struct Holders {
    count: usize,
}

impl Holders {
    /// Some when the session has to change, with playback on or off.
    fn add(&mut self) -> Option<bool> {
        self.count += 1;
        (self.count == 1).then_some(true)
    }

    fn remove(&mut self) -> Option<bool> {
        self.count -= 1;
        (self.count == 0).then_some(false)
    }
}

fn apply(change: Option<bool>) {
    let Some(playback) = change else {
        return;
    };
    info!("audio session: playback {playback}");
    #[cfg(ios)]
    set_category(playback);
}

#[cfg(ios)]
fn set_category(playback: bool) {
    use log::warn;
    use objc2_avf_audio::{
        AVAudioSession, AVAudioSessionCategoryPlayback, AVAudioSessionCategorySoloAmbient,
    };

    // SAFETY: the shared session and 2 constants of the framework, which is
    // linked on every iOS.
    let (session, category) = unsafe {
        (
            AVAudioSession::sharedInstance(),
            if playback {
                AVAudioSessionCategoryPlayback
            } else {
                AVAudioSessionCategorySoloAmbient
            },
        )
    };
    let Some(category) = category else {
        warn!("audio session: the system has no such category, playback {playback}");
        return;
    };
    // SAFETY: a category constant of the framework.
    if let Err(err) = unsafe { session.setCategory_error(category) } {
        warn!("audio session: the category did not change, playback {playback}: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::Holders;

    /// Only the first video turns playback on and only the last one turns
    /// it off, the ones in between change nothing.
    #[test]
    fn first_holder_turns_on_last_turns_off() {
        let mut holders = Holders::default();
        assert_eq!(holders.add(), Some(true));
        assert_eq!(holders.add(), None);
        assert_eq!(holders.remove(), None);
        assert_eq!(holders.remove(), Some(false));
        assert_eq!(holders.add(), Some(true));
    }
}
