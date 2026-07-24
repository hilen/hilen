#![cfg(target_arch = "wasm32")]

/// True when `name` is a key in the page query string. The browser test
/// driver uses query flags the way native lanes use env vars.
pub(crate) fn query_flag(name: &str) -> bool {
    page_search()
        .trim_start_matches('?')
        .split('&')
        .any(|pair| pair == name || pair.split_once('=').is_some_and(|(key, _)| key == name))
}

/// Value of `name` in the page query string.
pub(crate) fn query_param(name: &str) -> Option<String> {
    page_search().trim_start_matches('?').split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}

fn page_search() -> String {
    web_sys::window()
        .expect("Failed to get browser window")
        .location()
        .search()
        .expect("Failed to get location search")
}
