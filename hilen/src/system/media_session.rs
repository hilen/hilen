//! What the system knows about the media the app plays: the Now Playing
//! panel, and the play, pause and next keys of the keyboard and of
//! headphones, and on a phone the lock screen and the control center. On an
//! Apple TV also the play and pause button of the remote. macOS, iOS and
//! tvOS so far, everywhere else the calls do nothing and no command ever
//! comes.

#[cfg(all(feature = "ui-tests", desktop))]
#[path = "media_session_test.rs"]
mod tests_ui;

use crate::{deps::refs::main_lock::MainLock, ui::UIEvent};

/// What plays, as the system shows it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NowPlaying {
    pub title:    String,
    pub artist:   String,
    pub album:    String,
    /// Seconds, 0 when it is not known.
    pub duration: f64,
}

/// What the user asked for with a media key or in the Now Playing panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MediaCommand {
    Play,
    Pause,
    /// The play and pause key of a keyboard.
    Toggle,
    Stop,
    Next,
    Previous,
    /// The scrubber of the Now Playing panel, in seconds.
    SeekTo(f64),
}

#[derive(Default)]
struct Session {
    commands:   UIEvent<MediaCommand>,
    /// The handlers of the system's commands are in place.
    registered: bool,
    /// What the app said last in `set_playback`.
    #[cfg(ios)]
    playing:    bool,
}

static SESSION: MainLock<Session> = MainLock::new();

/// The app's entry in the system's Now Playing panel and its media keys.
/// Main thread only.
pub struct MediaSession;

impl MediaSession {
    /// Tells the system what plays, none takes the app out of the panel.
    /// The first call also starts the media keys coming to `on_command`.
    pub fn set_now_playing(info: Option<&NowPlaying>) {
        Self::register();
        platform::set_now_playing(info);
    }

    /// Tells the system whether it plays and how far in, so the panel shows
    /// the right button and the scrubber moves. Call it on play, on pause
    /// and after a seek, the system counts on by itself in between.
    pub fn set_playback(playing: bool, position: f64) {
        Self::register();
        #[cfg(ios)]
        {
            SESSION.get_mut().playing = playing;
        }
        platform::set_playback(playing, position);
    }

    /// Fires with every command of a media key or of the Now Playing panel.
    pub fn on_command() -> &'static UIEvent<MediaCommand> {
        &SESSION.commands
    }

    /// Gives the media keys and the Now Playing panel back to the system.
    #[cfg(hot)]
    pub(crate) fn stop() {
        if SESSION.registered {
            platform::stop();
        }
    }

    fn register() {
        if SESSION.registered {
            return;
        }
        SESSION.get_mut().registered = true;
        platform::register();
    }
}

/// A command from the system arrives here, on the main thread.
#[cfg(any(macos, ios, all(feature = "ui-tests", desktop)))]
pub(crate) fn deliver(command: MediaCommand) {
    log::debug!("media command: {command:?}");
    // A phone and an Apple TV pick between the play and the pause command
    // by whether sound comes out of the app, not by the rate the app
    // reports. The sound output of the engine runs on while a video is
    // paused, so the button of a remote says pause on every press. A pause
    // for an app that is paused already is that button asking to play.
    #[cfg(ios)]
    let command = if command == MediaCommand::Pause && !SESSION.playing {
        log::debug!("media command: the app is paused, the pause plays");
        MediaCommand::Play
    } else {
        command
    };
    SESSION.commands.trigger(command);
}

#[cfg(any(macos, ios))]
mod platform {
    use std::{cell::RefCell, ptr::NonNull};

    use block2::RcBlock;
    use objc2::{Message, rc::Retained, runtime::AnyObject};
    use objc2_foundation::{NSMutableDictionary, NSNumber, NSString};
    #[cfg(macos)]
    use objc2_media_player::MPNowPlayingPlaybackState;
    use objc2_media_player::{
        MPChangePlaybackPositionCommandEvent, MPMediaItemPropertyAlbumTitle, MPMediaItemPropertyArtist,
        MPMediaItemPropertyPlaybackDuration, MPMediaItemPropertyTitle, MPNowPlayingInfoCenter,
        MPNowPlayingInfoPropertyElapsedPlaybackTime, MPNowPlayingInfoPropertyPlaybackRate, MPRemoteCommand,
        MPRemoteCommandCenter, MPRemoteCommandEvent, MPRemoteCommandHandlerStatus,
    };

    use crate::{
        deps::hreads::on_main,
        system::media_session::{MediaCommand, NowPlaying, deliver},
    };

    thread_local! {
        /// Each command with what it gave back for its handler, the only
        /// way to take that handler off again.
        static HANDLERS: RefCell<Vec<(Retained<MPRemoteCommand>, Retained<AnyObject>)>> =
            const { RefCell::new(Vec::new()) };
    }

    /// Hooks one system command up to `deliver`. The system calls the handler
    /// on a queue of its own choice, so the command hops to the main thread.
    fn handle(
        command: &MPRemoteCommand,
        make: impl Fn(NonNull<MPRemoteCommandEvent>) -> MediaCommand + 'static,
    ) {
        let handler = RcBlock::new(move |event: NonNull<MPRemoteCommandEvent>| {
            let command = make(event);
            on_main(move || deliver(command));
            MPRemoteCommandHandlerStatus::Success
        });
        // SAFETY: the block takes the event the system hands it and returns a
        // status, the signature the method wants. The system keeps the block.
        let target = unsafe {
            command.setEnabled(true);
            command.addTargetWithHandler(&handler)
        };
        HANDLERS.with_borrow_mut(|handlers| handlers.push((command.retain(), target)));
    }

    /// Takes the handlers off the commands again and the app out of the Now
    /// Playing panel. The command center is shared by the process, and a hot
    /// build that is stopped must not be called through it, see
    /// `docs/hot-reload.md`.
    #[cfg(hot)]
    pub(super) fn stop() {
        HANDLERS.with_borrow_mut(|handlers| {
            for (command, target) in handlers.drain(..) {
                // SAFETY: the target is the one this command gave out.
                unsafe { command.removeTarget(Some(&target)) };
            }
        });
        set_now_playing(None);
    }

    pub(super) fn register() {
        // SAFETY: plain getters of the shared command center.
        unsafe {
            let center = MPRemoteCommandCenter::sharedCommandCenter();
            handle(&center.playCommand(), |_| MediaCommand::Play);
            handle(&center.pauseCommand(), |_| MediaCommand::Pause);
            handle(&center.togglePlayPauseCommand(), |_| MediaCommand::Toggle);
            handle(&center.stopCommand(), |_| MediaCommand::Stop);
            handle(&center.nextTrackCommand(), |_| MediaCommand::Next);
            handle(&center.previousTrackCommand(), |_| MediaCommand::Previous);
            handle(&center.changePlaybackPositionCommand(), |event| {
                // SAFETY: this command only ever sends its own event type.
                let seconds = event.cast::<MPChangePlaybackPositionCommandEvent>().as_ref().positionTime();
                MediaCommand::SeekTo(seconds)
            });
        }
    }

    pub(super) fn set_now_playing(info: Option<&NowPlaying>) {
        // SAFETY: the dictionary holds strings and numbers under the keys the
        // framework documents for them.
        unsafe {
            let center = MPNowPlayingInfoCenter::defaultCenter();
            let Some(info) = info else {
                center.setNowPlayingInfo(None);
                // Only macOS has a playback state, a phone reads the rate.
                #[cfg(macos)]
                center.setPlaybackState(MPNowPlayingPlaybackState::Stopped);
                return;
            };

            let entries = NSMutableDictionary::<NSString, AnyObject>::new();
            let text = |key: &NSString, value: &str| {
                if !value.is_empty() {
                    entries.insert(key, NSString::from_str(value).as_ref());
                }
            };
            text(MPMediaItemPropertyTitle, &info.title);
            text(MPMediaItemPropertyArtist, &info.artist);
            text(MPMediaItemPropertyAlbumTitle, &info.album);
            if info.duration > 0.0 {
                entries.insert(
                    MPMediaItemPropertyPlaybackDuration,
                    NSNumber::new_f64(info.duration).as_ref(),
                );
            }
            center.setNowPlayingInfo(Some(&entries));
        }
    }

    pub(super) fn set_playback(playing: bool, position: f64) {
        // SAFETY: as in `set_now_playing`. The info is copied, changed and
        // set again, the framework has no setter for a single key.
        unsafe {
            let center = MPNowPlayingInfoCenter::defaultCenter();
            let entries = NSMutableDictionary::<NSString, AnyObject>::new();
            if let Some(current) = center.nowPlayingInfo() {
                entries.addEntriesFromDictionary(&current);
            }
            entries.insert(
                MPNowPlayingInfoPropertyElapsedPlaybackTime,
                NSNumber::new_f64(position).as_ref(),
            );
            let rate = if playing { 1.0 } else { 0.0 };
            entries.insert(
                MPNowPlayingInfoPropertyPlaybackRate,
                NSNumber::new_f64(rate).as_ref(),
            );
            center.setNowPlayingInfo(Some(&entries));
            #[cfg(macos)]
            center.setPlaybackState(if playing {
                MPNowPlayingPlaybackState::Playing
            } else {
                MPNowPlayingPlaybackState::Paused
            });
        }
    }
}

#[cfg(not(any(macos, ios)))]
mod platform {
    use crate::system::media_session::NowPlaying;

    pub(super) fn register() {}

    pub(super) fn set_now_playing(info: Option<&NowPlaying>) {
        log::trace!("Now Playing is not reported on this platform: {info:?}");
    }

    pub(super) fn set_playback(playing: bool, position: f64) {
        log::trace!("playback is not reported on this platform: {playing} at {position}");
    }
}
