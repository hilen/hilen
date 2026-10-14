//! A page that loads on a link of the app, like
//! `https://shopping.vladas.xyz/invite/x7k2`, gives that address to the app
//! like a phone gives the link it was opened with. The root page with no
//! query is no link.

use crate::system::Router;

/// Runs once at the start of the page.
pub(crate) fn receive_start_url() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    let (Ok(href), Ok(search)) = (location.href(), location.search()) else {
        return;
    };
    let path = Router::current_path().unwrap_or_default();
    if path.is_empty() && search.is_empty() {
        return;
    }
    super::receive(href, "the address of the page");
}
