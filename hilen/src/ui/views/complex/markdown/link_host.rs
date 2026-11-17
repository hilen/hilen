//! The host a link really goes to, shown after it, see `MarkdownLinkHosts`.
//! A text is drawn on a canvas, so a browser shows no address under the
//! mouse, and a link can say one address and open another.

use url::Url;

/// The host of a web address the way a browser reads it: lower case, the
/// name before an `@` left out, other letters than ASCII as punycode.
fn host_of(address: &str) -> Option<String> {
    let url = Url::parse(address.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.host_str().map(str::to_owned)
}

/// The host to show after a link to `url` whose text is `shown`. None for a
/// link to `own`, the host of the app itself, for a link that is no web
/// address, and for a text that is an address of the same host already.
pub(super) fn outside_host(url: &str, shown: &str, own: &str) -> Option<String> {
    let host = host_of(url)?;
    if host == own {
        return None;
    }
    let shown = shown.trim();
    let named = host_of(shown).or_else(|| host_of(&format!("https://{shown}")));
    (named.as_deref() != Some(host.as_str())).then_some(host)
}

/// The host of the page in a browser, empty anywhere else: an app with a
/// window has no site of its own.
pub(super) fn own_host() -> String {
    #[cfg(wasm)]
    {
        web_sys::window()
            .and_then(|window| window.location().hostname().ok())
            .unwrap_or_default()
    }
    #[cfg(not_wasm)]
    {
        String::new()
    }
}

#[cfg(test)]
mod test {
    use super::outside_host;

    const OWN: &str = "site.example";

    #[test]
    fn a_link_to_another_site_shows_its_host() {
        assert_eq!(
            outside_host("https://evil.example/a?b#c", "my guide", OWN).as_deref(),
            Some("evil.example")
        );
        assert_eq!(
            outside_host("http://Evil.Example:8080/", "https://site.example/settings", OWN).as_deref(),
            Some("evil.example"),
            "a text that names another address does not hide the real one"
        );
    }

    #[test]
    fn the_host_is_read_like_a_browser_reads_it() {
        assert_eq!(
            outside_host("https://site.example@evil.example/", "x", OWN).as_deref(),
            Some("evil.example"),
            "the part before the @ is a user name"
        );
        assert_eq!(
            outside_host("https://site.example.evil.example/", "x", OWN).as_deref(),
            Some("site.example.evil.example")
        );
        assert_eq!(
            outside_host("https:\\\\evil.example/", "x", OWN).as_deref(),
            Some("evil.example"),
            "a browser reads a backslash as a slash"
        );
    }

    #[test]
    fn no_host_where_the_text_says_it_already_or_the_link_stays() {
        assert_eq!(
            outside_host("https://site.example/thread/5", "a thread", OWN),
            None
        );
        assert_eq!(
            outside_host("https://evil.example/a", "https://evil.example/a", OWN),
            None
        );
        assert_eq!(
            outside_host("https://evil.example/a", " evil.example ", OWN),
            None
        );
        assert_eq!(outside_host("mailto:ann@evil.example", "write me", OWN), None);
        assert_eq!(outside_host("not an address", "x", OWN), None);
    }
}
