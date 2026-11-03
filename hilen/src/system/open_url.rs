//! Opens a link in whatever the platform uses for it: the default browser
//! on desktop, a new tab in the browser build, the system handler on
//! phones.

use anyhow::{Result, anyhow};

/// The schemes a link may open. Any other, like `javascript:`, `data:` or a
/// handler an app registered on the desktop, can run code, so text a stranger
/// wrote must never open one.
const SAFE_SCHEMES: [&str; 3] = ["http", "https", "mailto"];

/// Whether `url` is an absolute `http`, `https` or `mailto` address.
///
/// The scheme is read the way a browser reads it: spaces and control
/// characters before it are dropped, and a tab or a line break inside it is
/// skipped, so `java\tscript:` is still `javascript:`.
pub fn is_safe_link(url: &str) -> bool {
    scheme_of(url).is_some_and(|scheme| SAFE_SCHEMES.contains(&scheme.as_str()))
}

/// The scheme of `url` in lower case, or none for a path or a relative address.
fn scheme_of(url: &str) -> Option<String> {
    let mut scheme = String::new();
    for c in url.trim_start_matches(|c: char| c <= ' ').chars() {
        match c {
            '\t' | '\n' | '\r' => {}
            ':' => return (!scheme.is_empty()).then_some(scheme),
            c if c.is_ascii_alphabetic() => scheme.push(c.to_ascii_lowercase()),
            c if !scheme.is_empty() && (c.is_ascii_digit() || matches!(c, '+' | '-' | '.')) => scheme.push(c),
            _ => return None,
        }
    }
    None
}

/// Whether `open_url` refuses `url`: it has a scheme that is not safe. A path
/// has no scheme and opens, also `C:\\Users` on Windows, whose drive letter
/// reads like a scheme of 1 letter.
fn is_refused(url: &str) -> bool {
    match scheme_of(url) {
        None => false,
        Some(scheme) => !SAFE_SCHEMES.contains(&scheme.as_str()) && scheme.len() > 1,
    }
}

pub fn open_url(url: impl ToString) -> Result<()> {
    let url = url.to_string();

    if is_refused(&url) {
        log::warn!("refused to open {url:?}, only http, https and mailto links open");
        return Err(anyhow!("Only http, https and mailto links open"));
    }

    #[cfg(desktop)]
    {
        Ok(open::that_detached(&url)?)
    }

    #[cfg(wasm)]
    {
        let window = web_sys::window().expect("Failed to get browser window");

        match window.open_with_url_and_target(&url, "_blank") {
            Ok(Some(opened)) => {
                // The opened page must not reach this one through
                // `window.opener`, it could send this tab to a fake page.
                // The feature `noopener` does the same, but then `open`
                // gives no window back and a blocked popup looks like an
                // opened one.
                opened
                    .set_opener(&web_sys::wasm_bindgen::JsValue::NULL)
                    .map_err(|err| anyhow!("Failed to drop the opener of {url}: {err:?}"))?;
                Ok(())
            }
            Ok(None) => Err(anyhow!("Failed to open {url}, the browser blocked the popup")),
            Err(err) => Err(anyhow!("Failed to open {url}: {err:?}")),
        }
    }

    #[cfg(ios)]
    {
        use objc2_foundation::{MainThreadMarker, NSDictionary, NSString, NSURL};
        use objc2_ui_kit::UIApplication;

        let mtm =
            MainThreadMarker::new().ok_or_else(|| anyhow!("Failed to open {url}: not on the main thread"))?;

        let ns_url = NSURL::URLWithString(&NSString::from_str(&url))
            .ok_or_else(|| anyhow!("Failed to open {url}: not a valid URL"))?;

        unsafe {
            UIApplication::sharedApplication(mtm).openURL_options_completionHandler(
                &ns_url,
                &NSDictionary::new(),
                None,
            );
        }

        Ok(())
    }

    #[cfg(android)]
    {
        android_open(&url)
    }
}

#[cfg(android)]
fn android_open(url: &str) -> anyhow::Result<()> {
    use jni::{jni_sig, jni_str, objects::JValue};

    use crate::system::android_jni::with_activity;

    with_activity(|env, activity| {
        let uri_string = env.new_string(url)?;
        let uri = env
            .call_static_method(
                jni_str!("android/net/Uri"),
                jni_str!("parse"),
                jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
                &[JValue::Object(&uri_string)],
            )?
            .l()?;

        let action = env.new_string("android.intent.action.VIEW")?;
        let intent = env.new_object(
            jni_str!("android/content/Intent"),
            jni_sig!("(Ljava/lang/String;Landroid/net/Uri;)V"),
            &[JValue::Object(&action), JValue::Object(&uri)],
        )?;

        env.call_method(
            activity,
            jni_str!("startActivity"),
            jni_sig!("(Landroid/content/Intent;)V"),
            &[JValue::Object(&intent)],
        )?;

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::{is_refused, is_safe_link};

    #[test]
    fn only_http_https_and_mailto_are_safe() {
        for url in [
            "https://a.b/c",
            "http://a.b",
            "HTTPS://A.B",
            "mailto:a@b.c",
            " \u{1}https://a.b",
        ] {
            assert!(is_safe_link(url), "{url:?} is a safe link");
        }
        for url in [
            "javascript:alert(1)",
            "JavaScript:alert(1)",
            " javascript:alert(1)",
            "java\tscript:alert(1)",
            "java\nscript:alert(1)",
            "javascript://x%0Aalert(1)",
            "data:text/html,<script>alert(1)</script>",
            "vbscript:x",
            "file:///etc/passwd",
            "ms-msdt:/id",
            "/thread/1",
            "thread/1",
            "//evil.example",
            "",
        ] {
            assert!(!is_safe_link(url), "{url:?} is not a safe link");
        }
    }

    #[test]
    fn open_url_refuses_other_schemes_and_takes_paths() {
        for url in [
            "javascript:alert(1)",
            "java\tscript:x",
            "data:text/html,x",
            "file:///tmp",
            "ms-msdt:/id",
        ] {
            assert!(is_refused(url), "{url:?} is refused");
        }
        for url in [
            "https://a.b",
            "mailto:a@b.c",
            "/Users/me/game",
            "C:\\Users\\me",
            "folder/sub",
            "/thread/1",
        ] {
            assert!(!is_refused(url), "{url:?} opens");
        }
    }
}
