//! What a video is opened from, and the one place that opens it. The picture
//! and the sound each keep their own demuxer, both come through here so they
//! ask the server the same way.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use ffmpeg_next::{
    Dictionary, Error,
    format::{self, context::Input},
};

/// A file path or a url, with the request headers an http or https source is
/// asked with. A path or a url alone converts into one, so `set_source` takes
/// a plain string too.
#[derive(Clone, Default)]
pub struct VideoSource {
    location: String,
    headers:  Vec<(String, String)>,
}

impl VideoSource {
    pub fn new(location: impl AsRef<str>) -> Self {
        Self {
            location: location.as_ref().to_string(),
            headers:  Vec::new(),
        }
    }

    /// Adds a header to every request for this source, like a session token
    /// that must not sit in the url. A file ignores it.
    #[must_use]
    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.headers.push((name.as_ref().to_string(), value.as_ref().to_string()));
        self
    }

    /// The path or url, for log lines. Never the headers, they carry secrets.
    pub fn location(&self) -> &str {
        &self.location
    }

    /// Opens a demuxer over the source. A read gives up once `stop` is set, so
    /// a stalled network stream does not hold its thread after the player is
    /// gone.
    pub(crate) fn open(&self, stop: &Arc<AtomicBool>) -> Result<Input, Error> {
        crate::video::init();

        let stop = Arc::clone(stop);
        let interrupt = move || stop.load(Ordering::Relaxed);
        if self.headers.is_empty() {
            return format::input_with_interrupt(&self.location, interrupt);
        }

        let mut lines = String::new();
        for (name, value) in &self.headers {
            lines.push_str(name);
            lines.push_str(": ");
            lines.push_str(value);
            lines.push_str("\r\n");
        }
        let mut options = Dictionary::new();
        options.set("headers", &lines);
        format::input_with_interrupt_and_dictionary(&self.location, interrupt, options)
    }
}

impl<T: AsRef<str>> From<T> for VideoSource {
    fn from(location: T) -> Self {
        Self::new(location)
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, atomic::AtomicBool};

    use ffmpeg_next::{Error, decoder::find_by_name};

    use crate::video::VideoSource;

    /// The prebuilt archive must carry the software AV1 decoder and zlib.
    /// The png decoder is the witness for zlib, ffmpeg builds it only with
    /// zlib in.
    #[test]
    fn dav1d_and_zlib_are_linked() {
        crate::video::init();
        assert!(
            find_by_name("libdav1d").is_some(),
            "the ffmpeg archive has no dav1d"
        );
        assert!(find_by_name("png").is_some(), "the ffmpeg archive has no zlib");
    }

    /// The prebuilt archive must carry TLS. Nothing listens on the port, so
    /// the open fails either way, with a connect error when the https protocol
    /// is linked and with `ProtocolNotFound` when it is not.
    #[test]
    fn https_protocol_is_linked() {
        let stop = Arc::new(AtomicBool::new(false));
        let Err(err) = VideoSource::new("https://127.0.0.1:9/video.mp4").open(&stop) else {
            panic!("nothing serves this url");
        };
        assert!(
            !matches!(err, Error::ProtocolNotFound),
            "the ffmpeg archive has no https protocol"
        );
    }
}
