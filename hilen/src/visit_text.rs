#![cfg(any(target_arch = "wasm32", test))]

//! What the visit record of a page leaves out, see `web_visit.rs`. The
//! record goes to the server of the app, so it must hold nothing a visitor
//! typed. Plain text code, so the tests run on every target.

/// The request code logs the address of every request, with a search word
/// or a name in it.
pub(crate) fn holds_requests(target: &str) -> bool {
    target.split("::").any(|part| part == "netrun")
}

/// `url` with no query and no fragment, a search word stands there.
pub(crate) fn without_query(url: &str) -> &str {
    url.split(['?', '#']).next().unwrap_or(url)
}

#[cfg(test)]
mod test {
    use super::{holds_requests, without_query};

    #[test]
    fn the_lines_of_requests_stay_out() {
        assert!(holds_requests("hilen::deps::netrun::rest::call"));
        assert!(!holds_requests("hilen::window::state"));
        assert!(!holds_requests("blackforge_web::ui::forum"));
        assert!(!holds_requests("netrunner::game"));
    }

    #[test]
    fn an_address_loses_its_query() {
        assert_eq!(
            without_query("https://a.example/feedback?q=my+secret#top"),
            "https://a.example/feedback"
        );
        assert_eq!(without_query("https://a.example/#x"), "https://a.example/");
        assert_eq!(
            without_query("https://a.example/thread/5"),
            "https://a.example/thread/5"
        );
    }
}
