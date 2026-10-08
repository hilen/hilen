//! What a video is opened from, a file path or a url with its request
//! headers.

#[cfg(ffmpeg)]
use std::time::Duration;

#[cfg(ffmpeg)]
mod open;

#[cfg(ffmpeg)]
pub(crate) use open::{Interrupt, byte_position, transport_error};

/// A file path or a url, with the request headers an http or https source is
/// asked with. A path or a url alone converts into one, so `set_source` takes
/// a plain string too.
#[derive(Clone, Default)]
pub struct VideoSource {
    location:        String,
    headers:         Vec<(String, String)>,
    /// Set by a test that cannot wait for the real limit.
    #[cfg(ffmpeg)]
    reconnect_limit: Option<Duration>,
}

impl VideoSource {
    pub fn new(location: impl AsRef<str>) -> Self {
        Self {
            location:                       location.as_ref().to_string(),
            headers:                        Vec::new(),
            #[cfg(ffmpeg)]
            reconnect_limit:                None,
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

    #[cfg(wasm)]
    pub(crate) fn headers(&self) -> &[(String, String)] {
        &self.headers
    }
}

impl<T: AsRef<str>> From<T> for VideoSource {
    fn from(location: T) -> Self {
        Self::new(location)
    }
}
