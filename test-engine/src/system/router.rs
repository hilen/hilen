//! Browser URL routing hook. The engine owns the history API: it reads the
//! path the page loaded on, pushes an entry per navigation and surfaces
//! browser back and forward as an event. The app maps paths to pages in both
//! directions. Outside the browser every call is a no-op, so app navigation
//! code carries no platform cfg.
//!
//! Paths are relative to the document base and carry no leading slash, the
//! root is an empty string. Trunk writes the base href, so a dist mounted at
//! `/te/` and a dev serve at `/` both resolve untouched.

use refs::main_lock::MainLock;

use crate::ui::UIEvent;

static ON_POP: MainLock<UIEvent<String>> = MainLock::new();

#[cfg(wasm)]
static POPSTATE_LISTENER: MainLock<Option<web_sys::wasm_bindgen::closure::Closure<dyn FnMut()>>> =
    MainLock::new();

pub struct Router;

impl Router {
    /// The current path relative to the document base. At startup this is
    /// the path the page loaded on. `None` outside the browser.
    pub fn current_path() -> Option<String> {
        #[cfg(wasm)]
        {
            let pathname = web_sys::window()
                .expect("Failed to get browser window")
                .location()
                .pathname()
                .expect("Failed to get location pathname");
            Some(strip_base(base_path(), &pathname))
        }
        #[cfg(not_wasm)]
        {
            None
        }
    }

    /// Push a history entry for `path`. The browser URL changes, the page
    /// does not reload.
    pub fn push(path: &str) {
        #[cfg(wasm)]
        set_url(path, false);
        #[cfg(not_wasm)]
        log::trace!("Router::push({path}) outside the browser is a no-op");
    }

    /// Replace the current history entry with `path`. The boot time page
    /// swap goes through this, a push there would break the first back.
    pub fn replace(path: &str) {
        #[cfg(wasm)]
        set_url(path, true);
        #[cfg(not_wasm)]
        log::trace!("Router::replace({path}) outside the browser is a no-op");
    }

    /// Fired with the new relative path when the browser walks history,
    /// back or forward. Never fires outside the browser.
    pub fn on_pop() -> &'static UIEvent<String> {
        &ON_POP
    }
}

#[cfg(wasm)]
fn set_url(path: &str, replace: bool) {
    use web_sys::wasm_bindgen::JsValue;

    let url = join_base(base_path(), path);
    let history = web_sys::window()
        .expect("Failed to get browser window")
        .history()
        .expect("Failed to get browser history");

    if replace {
        history
            .replace_state_with_url(&JsValue::NULL, "", Some(&url))
            .expect("Failed to replace history state");
    } else {
        history
            .push_state_with_url(&JsValue::NULL, "", Some(&url))
            .expect("Failed to push history state");
    }
}

/// Path of the document base href. Fetches already resolve against it, so
/// history entries must too, or a dist mounted at `/te/` writes URLs the
/// server routes to the Vue app.
///
/// Captured once at startup. Without a `<base>` tag the browser derives
/// the base from the current document URL, which every push moves, so a
/// live read after the first navigation returns the pushed path instead
/// of the mount point.
#[cfg(wasm)]
fn base_path() -> &'static str {
    BASE_PATH.get_or_init(|| {
        let base_uri = web_sys::window()
            .expect("Failed to get browser window")
            .document()
            .expect("Failed to get browser document")
            .base_uri()
            .expect("Failed to get document base uri");

        let Some(base_uri) = base_uri else {
            return "/".to_string();
        };

        web_sys::Url::new(&base_uri)
            .expect("Failed to parse document base uri")
            .pathname()
    })
}

#[cfg(wasm)]
static BASE_PATH: MainLock<String> = MainLock::new();

/// Installed once at startup, after the main thread is set. The closure must
/// stay alive for the page's lifetime, dropping it detaches the listener.
#[cfg(wasm)]
pub(crate) fn install_popstate_listener() {
    use web_sys::wasm_bindgen::{JsCast, closure::Closure};

    // The base must be read before the app's first push can move it.
    base_path();

    let listener = Closure::<dyn FnMut()>::new(|| {
        let path = Router::current_path().expect("Popstate fired outside the browser");
        ON_POP.trigger(path);
    });

    web_sys::window()
        .expect("Failed to get browser window")
        .set_onpopstate(Some(listener.as_ref().unchecked_ref()));

    POPSTATE_LISTENER.set(Some(listener));
}

#[cfg(any(wasm, test))]
fn strip_base(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.strip_prefix(base).unwrap_or(path);
    path.trim_start_matches('/').to_string()
}

#[cfg(any(wasm, test))]
fn join_base(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{base}/{path}")
}

#[cfg(test)]
mod test {
    use super::{join_base, strip_base};

    #[test]
    fn strip_base_at_root() {
        assert_eq!(strip_base("/", "/"), "");
        assert_eq!(strip_base("/", "/deployments"), "deployments");
        assert_eq!(strip_base("/", "/deployments/some-app"), "deployments/some-app");
    }

    #[test]
    fn strip_base_mounted() {
        assert_eq!(strip_base("/te/", "/te/"), "");
        assert_eq!(strip_base("/te/", "/te"), "");
        assert_eq!(strip_base("/te/", "/te/vpn/laptop"), "vpn/laptop");
    }

    #[test]
    fn strip_base_foreign_path_degrades_to_relative() {
        assert_eq!(strip_base("/te/", "/other"), "other");
    }

    #[test]
    fn join_base_at_root() {
        assert_eq!(join_base("/", ""), "/");
        assert_eq!(join_base("/", "deployments"), "/deployments");
    }

    #[test]
    fn join_base_mounted() {
        assert_eq!(join_base("/te/", ""), "/te/");
        assert_eq!(
            join_base("/te/", "deployments/some-app"),
            "/te/deployments/some-app"
        );
    }

    #[test]
    fn join_strip_round_trip() {
        for base in ["/", "/te/"] {
            for path in ["", "deployments", "vpn/laptop"] {
                assert_eq!(strip_base(base, &join_base(base, path)), path);
            }
        }
    }
}
