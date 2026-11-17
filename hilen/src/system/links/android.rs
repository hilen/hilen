//! Android starts the activity with a `VIEW` intent for an https link of a
//! domain that `[links]` of `hilen.toml` names, and for a link of the url
//! scheme. `hilen-mobile` writes the intent filters. The activity runs as
//! `singleTask`, so a link while the app runs goes to `onNewIntent` of the
//! same activity, and the template makes that the intent of the activity.
//!
//! The engine reads the intent of the activity when the app comes to the
//! front, at the start and after every new intent, and then puts a blank
//! intent in its place, so the same link is never given twice.

use anyhow::Result;
use jni::{
    jni_sig, jni_str,
    objects::{JString, JValue},
};
use log::warn;

use crate::system::android_jni::{clear_exception, with_activity};

const VIEW: &str = "android.intent.action.VIEW";

/// Gives the link of the intent of the activity to the app, if it has one.
pub(crate) fn check_intent() {
    match take_link() {
        Ok(Some(url)) => super::receive(url, "an Android intent"),
        Ok(None) => {}
        Err(err) => warn!("the intent of the activity could not be read: {err:#}"),
    }
}

fn take_link() -> Result<Option<String>> {
    with_activity(|env, activity| {
        let result = (|| -> Result<Option<String>> {
            let intent = env
                .call_method(
                    activity,
                    jni_str!("getIntent"),
                    jni_sig!("()Landroid/content/Intent;"),
                    &[],
                )?
                .l()?;
            if intent.is_null() {
                return Ok(None);
            }

            let action = env
                .call_method(
                    &intent,
                    jni_str!("getAction"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;
            if action.is_null() || env.cast_local::<JString>(action)?.try_to_string(env)? != VIEW {
                return Ok(None);
            }

            let data = env
                .call_method(
                    &intent,
                    jni_str!("getDataString"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;
            if data.is_null() {
                return Ok(None);
            }
            let url = env.cast_local::<JString>(data)?.try_to_string(env)?;

            let blank = env.new_object(jni_str!("android/content/Intent"), jni_sig!("()V"), &[])?;
            env.call_method(
                activity,
                jni_str!("setIntent"),
                jni_sig!("(Landroid/content/Intent;)V"),
                &[JValue::Object(&blank)],
            )?;
            Ok(Some(url))
        })();
        if result.is_err() {
            clear_exception(env);
        }
        result
    })
}
