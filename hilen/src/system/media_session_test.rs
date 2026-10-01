use anyhow::{Result, ensure};

use super::{MediaCommand, MediaSession, NowPlaying, deliver};
use crate::{
    self as hilen,
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
};

/// A player page tells the system what plays and hears its media keys.
/// Proves the title, the length, the position and the play state reach the
/// system's Now Playing record, read back from the system on macOS, that
/// none clears it, and that a command the system sends comes out of
/// `on_command`. No test can press a real media key, so the commands enter
/// where the system's handler hands them over.
#[view]
struct MediaSessionTest {
    /// Every command of `on_command`, in order.
    commands: Vec<MediaCommand>,
    #[init]
    title:    Label,
    state:    Label,
}

impl Setup for MediaSessionTest {
    fn setup(mut self: Weak<Self>) {
        self.title.set_frame((20, 40, 560, 40));
        self.title.set_text("Now Playing and the media keys");
        self.state.set_frame((20, 100, 560, 40));
        self.state.set_text("no command yet");
        MediaSession::on_command().val(self, move |command| {
            self.commands.push(command);
            self.state.set_text(format!("command: {command:?}"));
        });
    }
}

impl ViewTest for MediaSessionTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(|| {
            MediaSession::set_now_playing(Some(&NowPlaying {
                title:    "A test film".to_string(),
                artist:   "hilen".to_string(),
                album:    String::new(),
                duration: 5400.0,
            }));
            MediaSession::set_playback(true, 12.0);
        });
        #[cfg(macos)]
        {
            let seen = from_main(system::read);
            ensure!(
                seen == Some(("A test film".to_string(), 5400.0, 12.0, true)),
                "the system has {seen:?}"
            );
        }
        checkpoint("the system shows A test film, playing, 12 s in")?;

        from_main(|| MediaSession::set_playback(false, 30.0));
        #[cfg(macos)]
        {
            let seen = from_main(system::read);
            ensure!(
                seen == Some(("A test film".to_string(), 5400.0, 30.0, false)),
                "after a pause the system has {seen:?}"
            );
        }

        for command in [
            MediaCommand::Toggle,
            MediaCommand::Next,
            MediaCommand::SeekTo(90.0),
        ] {
            from_main(move || deliver(command));
        }
        let commands = from_main(move || view.commands.clone());
        ensure!(
            commands
                == [
                    MediaCommand::Toggle,
                    MediaCommand::Next,
                    MediaCommand::SeekTo(90.0)
                ],
            "on_command gave {commands:?}"
        );
        checkpoint("3 commands came through, the last is a seek to 90 s")?;

        from_main(|| MediaSession::set_now_playing(None));
        #[cfg(macos)]
        ensure!(
            from_main(system::read).is_none(),
            "none clears the system's record"
        );
        Ok(())
    }
}

/// The system's own Now Playing record of this app.
#[cfg(macos)]
mod system {
    use objc2_foundation::{NSNumber, NSString};
    use objc2_media_player::{
        MPMediaItemPropertyPlaybackDuration, MPMediaItemPropertyTitle, MPNowPlayingInfoCenter,
        MPNowPlayingInfoPropertyElapsedPlaybackTime, MPNowPlayingPlaybackState,
    };

    /// The title, the length, the position and whether it plays.
    pub(super) fn read() -> Option<(String, f64, f64, bool)> {
        // SAFETY: plain getters, and the values under these keys are the
        // strings and numbers the engine put there.
        unsafe {
            let center = MPNowPlayingInfoCenter::defaultCenter();
            let info = center.nowPlayingInfo()?;
            let number = |key: &NSString| {
                info.objectForKey(key)
                    .and_then(|value| value.downcast::<NSNumber>().ok())
                    .map_or(0.0, |value| value.as_f64())
            };
            let title = info
                .objectForKey(MPMediaItemPropertyTitle)?
                .downcast::<NSString>()
                .ok()?
                .to_string();
            Some((
                title,
                number(MPMediaItemPropertyPlaybackDuration),
                number(MPNowPlayingInfoPropertyElapsedPlaybackTime),
                center.playbackState() == MPNowPlayingPlaybackState::Playing,
            ))
        }
    }
}
