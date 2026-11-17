//! The 2 files that prove a backend owns its domain to the phones, so a
//! link of that domain opens the app and not the browser:
//!
//! - `/.well-known/apple-app-site-association` for iOS, from the team id and
//!   the bundle id of the app.
//! - `/.well-known/assetlinks.json` for Android, from the package name and the
//!   SHA-256 hash of the key the app is signed with.
//!
//! ```ignore
//! let links = AppLinks::new()
//!     .apple("ABCDE12345", "vladas.shopping")
//!     .android("com.example.shopping", ["14:6D:E9:...:5C"])?;
//! let app = base_routes("shopping").merge(app_links_mount(links));
//! ```
//!
//! Every path of the domain opens the app, the same on both phones. The
//! domains themselves are in the `[links]` table of the app's `hilen.toml`.

use anyhow::{Result, ensure};
use axum::{
    Json, Router,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::{Value, json};

pub const APPLE_PATH: &str = "/.well-known/apple-app-site-association";
pub const ANDROID_PATH: &str = "/.well-known/assetlinks.json";

/// The apps a domain opens. A phone with no app here gets a 404 for its file.
#[derive(Debug, Clone, Default)]
pub struct AppLinks {
    apple:   Vec<String>,
    android: Vec<AndroidApp>,
}

#[derive(Debug, Clone)]
struct AndroidApp {
    package:      String,
    fingerprints: Vec<String>,
}

impl AppLinks {
    pub fn new() -> Self {
        Self::default()
    }

    /// An iOS app, by the team id of the Apple developer account, the 10
    /// letters in the top right corner of the developer portal, and the
    /// `bundle_id` of `hilen.toml`.
    #[must_use]
    pub fn apple(mut self, team_id: &str, bundle_id: &str) -> Self {
        self.apple.push(format!("{team_id}.{bundle_id}"));
        self
    }

    /// An Android app, by its package name and the SHA-256 hash of each key
    /// it is signed with, as `keytool -list` or the Play Console print it,
    /// with or without the colons. A wrong hash is an error.
    pub fn android<S: AsRef<str>>(
        mut self,
        package: &str,
        fingerprints: impl IntoIterator<Item = S>,
    ) -> Result<Self> {
        let fingerprints = fingerprints
            .into_iter()
            .map(|hash| fingerprint(hash.as_ref()))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            !fingerprints.is_empty(),
            "the Android app {package} has no key hash"
        );
        self.android.push(AndroidApp {
            package: package.to_string(),
            fingerprints,
        });
        Ok(self)
    }

    /// The text of the file of iOS. It has both forms, `paths` for iOS 12
    /// and `components` for iOS 13 and later.
    pub fn apple_file(&self) -> Option<Value> {
        if self.apple.is_empty() {
            return None;
        }
        let details: Vec<Value> = self
            .apple
            .iter()
            .map(|app_id| {
                json!({
                    "appID": app_id,
                    "appIDs": [app_id],
                    "paths": ["*"],
                    "components": [{ "/": "*" }],
                })
            })
            .collect();
        Some(json!({ "applinks": { "apps": [], "details": details } }))
    }

    /// The text of the file of Android.
    pub fn android_file(&self) -> Option<Value> {
        if self.android.is_empty() {
            return None;
        }
        let statements: Vec<Value> = self
            .android
            .iter()
            .map(|app| {
                json!({
                    "relation": ["delegate_permission/common.handle_all_urls"],
                    "target": {
                        "namespace": "android_app",
                        "package_name": app.package,
                        "sha256_cert_fingerprints": app.fingerprints,
                    },
                })
            })
            .collect();
        Some(Value::Array(statements))
    }
}

/// `AB:CD:...` in capitals, the form Android compares, from a hash with or
/// without colons in any case.
fn fingerprint(hash: &str) -> Result<String> {
    let hex: String = hash.chars().filter(|c| *c != ':').collect::<String>().to_uppercase();
    ensure!(
        hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()),
        "{hash:?} is no SHA-256 hash of a signing key, it needs 32 bytes in hex"
    );
    let pairs: Vec<&str> = (0..hex.len()).step_by(2).map(|at| &hex[at..at + 2]).collect();
    Ok(pairs.join(":"))
}

/// The 2 routes. Both answer JSON with no redirect, the phones follow none.
pub fn app_links_mount<S>(links: AppLinks) -> Router<S>
where S: Clone + Send + Sync + 'static {
    let apple = links.apple_file();
    let android = links.android_file();
    Router::new()
        .route(APPLE_PATH, get(move || std::future::ready(serve(apple.clone()))))
        .route(
            ANDROID_PATH,
            get(move || std::future::ready(serve(android.clone()))),
        )
}

fn serve(file: Option<Value>) -> Response {
    match file {
        Some(file) => ([(header::CACHE_CONTROL, "max-age=3600")], Json(file)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod test {
    use super::fingerprint;

    const COLONS: &str =
        "14:6d:e9:83:c5:73:06:50:d8:ee:b9:95:2f:34:fc:64:16:a0:83:42:e6:1d:be:a8:8a:04:96:b2:3f:cf:44:e5";

    #[test]
    fn a_hash_is_written_the_way_android_compares_it() {
        let plain = COLONS.replace(':', "");
        let wanted = COLONS.to_uppercase();
        assert_eq!(fingerprint(COLONS).unwrap(), wanted);
        assert_eq!(fingerprint(&plain).unwrap(), wanted);
    }

    #[test]
    fn a_wrong_hash_is_an_error() {
        let not_hex = COLONS.replace("14", "ZZ");
        for hash in ["", "14:6D", &COLONS[3..], not_hex.as_str()] {
            assert!(fingerprint(hash).is_err(), "{hash}");
        }
    }
}
