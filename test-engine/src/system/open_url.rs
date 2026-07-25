//! Opens a link in whatever the platform uses for it: the default browser
//! on desktop, a new tab in the browser build, the system handler on
//! phones.

use anyhow::Result;
#[cfg(any(wasm, ios))]
use anyhow::anyhow;

pub fn open_url(url: impl ToString) -> Result<()> {
    let url = url.to_string();

    #[cfg(desktop)]
    {
        Ok(open::that_detached(&url)?)
    }

    #[cfg(wasm)]
    {
        let window = web_sys::window().expect("Failed to get browser window");

        match window.open_with_url_and_target(&url, "_blank") {
            Ok(Some(_)) => Ok(()),
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
    use jni::objects::JValue;

    use crate::system::android_jni::with_activity;

    with_activity(|env, activity| {
        let uri_string = env.new_string(url)?;
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&uri_string)],
            )?
            .l()?;

        let action = env.new_string("android.intent.action.VIEW")?;
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[JValue::Object(&action), JValue::Object(&uri)],
        )?;

        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;

        Ok(())
    })
}
