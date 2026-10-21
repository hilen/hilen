//! The link the app was opened with. A tap on a link of the app on a
//! phone, a link of its url scheme on a desktop, the address a page loaded
//! on in a browser. Each platform hands the link to `receive`, and
//! `Links::on_open` gives it to the app on the main thread.
//!
//! A link that comes before anybody listens is kept, the link of a cold
//! start most of all. It goes to the first view that subscribes.

#[cfg(target_os = "android")]
mod android;
#[cfg(desktop)]
mod desktop;
#[cfg(target_os = "ios")]
pub(crate) mod ios;
#[cfg(macos)]
mod macos;
#[cfg(wasm)]
mod web;

use log::info;
use parking_lot::Mutex;

#[cfg(target_os = "android")]
pub(crate) use self::android::check_intent;
#[cfg(desktop)]
pub(crate) use self::desktop::start as start_desktop;
#[cfg(wasm)]
pub(crate) use self::web::receive_start_url;
use crate::{
    deps::{hreads::on_main_later, refs::Weak},
    ui::UIEvent,
};

static ON_OPEN: UIEvent<OpenedLink> = UIEvent::const_new();

/// Links that came while nobody listened, oldest first.
static PENDING: Mutex<Vec<OpenedLink>> = Mutex::new(Vec::new());

/// A link that opened the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedLink {
    /// The whole link as the system gave it, like
    /// `https://shopping.vladas.xyz/invite/x7k2` on a phone or
    /// `shopping://invite/x7k2` on a desktop.
    pub url: String,
}

impl OpenedLink {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    /// The path of the link with no slash at the start or the end, and with
    /// no query and no fragment. It is the same for the https link of a
    /// phone and the scheme link of a desktop, both of the links above give
    /// `invite/x7k2`. A scheme link has no host, what stands after the
    /// `//` is the first part of the path.
    pub fn path(&self) -> String {
        self.parts().2
    }

    /// The scheme, the host of a web link and the path.
    fn parts(&self) -> (&str, &str, String) {
        let url = self.url.as_str();
        let (scheme, rest) = url.split_once(':').unwrap_or(("", url));
        let rest = rest.split(['?', '#']).next().unwrap_or_default();
        let web = scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http");
        let (host, path) = match rest.strip_prefix("//") {
            Some(after) if web => after.split_once('/').unwrap_or((after, "")),
            Some(after) => ("", after),
            None => ("", rest),
        };
        (scheme, host, path.trim_matches('/').to_string())
    }

    /// The link for a log line: the scheme, the host and the first part of
    /// the path. The rest of a link can be a secret, like the code of an
    /// invite.
    fn for_log(&self) -> String {
        let (scheme, host, path) = self.parts();
        let (first, more) = match path.split_once('/') {
            Some((first, _)) => (first, "/..."),
            None => (path.as_str(), ""),
        };
        if host.is_empty() {
            format!("{scheme}://{first}{more}")
        } else {
            format!("{scheme}://{host}/{first}{more}")
        }
    }
}

pub struct Links;

impl Links {
    /// Calls `action` with every link that opens the app, while `subscriber`
    /// lives, on the main thread and in a turn of the loop of its own. A
    /// link that came before, like the one of a cold start, goes to the
    /// first subscriber. An app subscribes once, in the view that decides
    /// which screen a link opens.
    pub fn on_open<U: ?Sized>(subscriber: Weak<U>, action: impl FnMut(OpenedLink) + Send + 'static) {
        ON_OPEN.val(subscriber, action);
        if !PENDING.lock().is_empty() {
            on_main_later(deliver_pending);
        }
    }
}

/// Takes a link from a platform, from any thread.
pub(crate) fn receive(url: impl Into<String>, source: &str) {
    let link = OpenedLink::new(url);
    info!("link from {source}: {}", link.for_log());
    PENDING.lock().push(link);
    on_main_later(deliver_pending);
}

fn deliver_pending() {
    if !ON_OPEN.has_subscribers() {
        let waiting = PENDING.lock().len();
        if waiting > 0 {
            info!("{waiting} link(s) wait for a subscriber of Links::on_open");
        }
        return;
    }
    let links: Vec<OpenedLink> = PENDING.lock().drain(..).collect();
    for link in links {
        ON_OPEN.trigger(link);
    }
}

/// Hands `url` to the app the way a platform does, for a test.
pub(crate) fn fake(url: &str) {
    receive(url, "a test");
}

/// Forgets the links that wait for a subscriber, before a test.
pub(crate) fn drop_pending() {
    PENDING.lock().clear();
}

#[cfg(test)]
mod test {
    use super::OpenedLink;

    fn path(url: &str) -> String {
        OpenedLink::new(url).path()
    }

    #[test]
    fn a_phone_link_and_a_desktop_link_have_the_same_path() {
        assert_eq!(path("https://shopping.vladas.xyz/invite/x7k2"), "invite/x7k2");
        assert_eq!(path("shopping://invite/x7k2"), "invite/x7k2");
        assert_eq!(path("shopping:///invite/x7k2"), "invite/x7k2");
        assert_eq!(path("shopping:invite/x7k2"), "invite/x7k2");
        assert_eq!(path("HTTPS://Shopping.vladas.xyz/invite/x7k2/"), "invite/x7k2");
    }

    #[test]
    fn the_path_has_no_query_and_no_fragment() {
        assert_eq!(path("https://a.b/invite/x?from=qr#top"), "invite/x");
        assert_eq!(path("shopping://invite/x?from=qr"), "invite/x");
        assert_eq!(path("https://a.b/?q=1"), "");
    }

    #[test]
    fn a_link_with_no_path_has_an_empty_one() {
        assert_eq!(path("https://shopping.vladas.xyz"), "");
        assert_eq!(path("https://shopping.vladas.xyz/"), "");
        assert_eq!(path("shopping://"), "");
    }

    #[test]
    fn the_log_shows_no_more_than_the_first_part_of_the_path() {
        let log = |url: &str| OpenedLink::new(url).for_log();
        assert_eq!(
            log("https://shopping.vladas.xyz/invite/x7k2"),
            "https://shopping.vladas.xyz/invite/..."
        );
        assert_eq!(
            log("https://shopping.vladas.xyz/invite?code=x7k2"),
            "https://shopping.vladas.xyz/invite"
        );
        assert_eq!(log("shopping://invite/x7k2"), "shopping://invite/...");
        assert_eq!(log("shopping://settings"), "shopping://settings");
    }
}
