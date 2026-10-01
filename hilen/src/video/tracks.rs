//! The sound and subtitle tracks of a source, as the app sees them.

use ffmpeg_next::{
    Stream,
    codec::{Id, context::Context},
    format::context::Input,
    media,
};

/// One sound track of a video. `index` is what `VideoView::set_audio_track`
/// takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioTrack {
    pub index:    usize,
    /// The language tag of the file, like `eng`, empty when it has none.
    pub language: String,
    pub title:    String,
    /// The ffmpeg codec name, like `aac`.
    pub codec:    String,
    pub channels: u16,
}

/// One subtitle track of a video. `index` is what
/// `VideoView::set_subtitle_track` takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleTrack {
    pub index:    usize,
    /// The language tag of the file, like `eng`, empty when it has none.
    pub language: String,
    pub title:    String,
    /// The ffmpeg codec name, like `subrip`.
    pub codec:    String,
    /// A text track. A picture track like PGS is listed but shows nothing yet.
    pub text:     bool,
}

fn tag(stream: &Stream, key: &str) -> String {
    stream.metadata().get(key).unwrap_or_default().to_string()
}

pub(crate) fn audio_tracks(input: &Input) -> Vec<AudioTrack> {
    input
        .streams()
        .filter(|stream| stream.parameters().medium() == media::Type::Audio)
        .map(|stream| {
            let channels = Context::from_parameters(stream.parameters())
                .and_then(|context| context.decoder().audio())
                .map_or(0, |decoder| decoder.channels());
            AudioTrack {
                index: stream.index(),
                language: tag(&stream, "language"),
                title: tag(&stream, "title"),
                codec: stream.parameters().id().name().to_string(),
                channels,
            }
        })
        .collect()
}

pub(crate) fn subtitle_tracks(input: &Input) -> Vec<SubtitleTrack> {
    input
        .streams()
        .filter(|stream| stream.parameters().medium() == media::Type::Subtitle)
        .map(|stream| {
            let id = stream.parameters().id();
            SubtitleTrack {
                index:    stream.index(),
                language: tag(&stream, "language"),
                title:    tag(&stream, "title"),
                codec:    id.name().to_string(),
                text:     !matches!(
                    id,
                    Id::HDMV_PGS_SUBTITLE | Id::DVD_SUBTITLE | Id::DVB_SUBTITLE | Id::DVB_TELETEXT | Id::XSUB
                ),
            }
        })
        .collect()
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, atomic::AtomicBool};

    use crate::video::{
        source::Interrupt,
        test_fixture,
        tracks::{audio_tracks, subtitle_tracks},
    };

    #[test]
    fn tracks_of_the_fixture_are_listed() {
        let stop = Arc::new(AtomicBool::new(false));
        let input = test_fixture("tracks.mkv")
            .open(&Interrupt::new(&stop))
            .expect("the fixture opens");

        let audio = audio_tracks(&input);
        let seen: Vec<_> = audio
            .iter()
            .map(|track| {
                (
                    track.index,
                    track.language.as_str(),
                    track.title.as_str(),
                    track.codec.as_str(),
                    track.channels,
                )
            })
            .collect();
        assert_eq!(
            seen,
            [(1, "eng", "English", "aac", 1), (2, "ger", "Deutsch", "aac", 1)]
        );

        let subtitles = subtitle_tracks(&input);
        let seen: Vec<_> = subtitles
            .iter()
            .map(|track| {
                (
                    track.index,
                    track.language.as_str(),
                    track.codec.as_str(),
                    track.text,
                )
            })
            .collect();
        assert_eq!(seen, [(3, "eng", "subrip", true), (4, "ger", "subrip", true)]);
    }
}
