//! What a video reports to its view, the same on every platform.

/// Playback counters for an overlay or a log line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VideoStats {
    /// Frames the decoder produced.
    pub decoded:              u64,
    /// Frames that reached the screen.
    pub presented:            u64,
    /// Frames skipped because a newer one was already due.
    pub dropped:              u64,
    /// Presented frames per second over the last second.
    pub presented_per_second: f64,
    /// The last frame came from the hardware decoder.
    pub hardware:             bool,
    /// The ffmpeg codec name.
    pub decoder:              String,
    pub width:                u32,
    pub height:               u32,
    /// The stream's own frame rate.
    pub frame_rate:           f64,
}

/// Where a video stands, read with `VideoView::state` and reported through
/// `VideoView::on_state`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VideoState {
    /// No source is set.
    #[default]
    Empty,
    /// The source is opening, no frame has reached the screen yet.
    Loading,
    Paused,
    Playing,
    /// Playing, but the stream has not delivered the next frame. The clock
    /// and the sound are held until it does.
    Buffering,
    /// Playback reached the end.
    Finished,
    /// The source could not be opened or decoded, `on_error` has the reason.
    Failed,
}

pub(crate) enum PlayerEvent {
    Finished,
    Error(String),
    /// The subtitle line to show, none clears it.
    Subtitle(Option<String>),
}
