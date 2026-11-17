//! The clipboard of Android, the `ClipboardManager` service over JNI.

use anyhow::{Result, anyhow};
use jni::{
    Env, jni_sig, jni_str,
    objects::{JObject, JString, JValue},
};
use zeroize::Zeroizing;

use crate::system::android_jni::with_activity;

/// `ClipDescription.EXTRA_IS_SENSITIVE`. The constant exists from API 33,
/// the key itself is read by the system from API 24.
const IS_SENSITIVE: &str = "android.content.extra.IS_SENSITIVE";

fn manager<'local>(env: &mut Env<'local>, activity: &JObject) -> Result<JObject<'local>> {
    let service = env.new_string("clipboard")?;
    Ok(env
        .call_method(
            activity,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[JValue::Object(&service)],
        )?
        .l()?)
}

/// Makes a plain text clip and puts it on the clipboard. A `sensitive`
/// clip tells the system to hide its text in the clipboard preview.
fn put(env: &mut Env, manager: &JObject, text: &str, sensitive: bool) -> Result<()> {
    let label = env.new_string("hilen")?;
    let value = env.new_string(text)?;
    let clip = env
        .call_static_method(
            jni_str!("android/content/ClipData"),
            jni_str!("newPlainText"),
            jni_sig!("(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"),
            &[JValue::Object(&label), JValue::Object(&value)],
        )?
        .l()?;

    if sensitive {
        let key = env.new_string(IS_SENSITIVE)?;
        let extras = env.new_object(jni_str!("android/os/PersistableBundle"), jni_sig!("()V"), &[])?;
        env.call_method(
            &extras,
            jni_str!("putBoolean"),
            jni_sig!("(Ljava/lang/String;Z)V"),
            &[JValue::Object(&key), JValue::Bool(true)],
        )?;

        let description = env
            .call_method(
                &clip,
                jni_str!("getDescription"),
                jni_sig!("()Landroid/content/ClipDescription;"),
                &[],
            )?
            .l()?;
        env.call_method(
            &description,
            jni_str!("setExtras"),
            jni_sig!("(Landroid/os/PersistableBundle;)V"),
            &[JValue::Object(&extras)],
        )?;
    }

    env.call_method(
        manager,
        jni_str!("setPrimaryClip"),
        jni_sig!("(Landroid/content/ClipData;)V"),
        &[JValue::Object(&clip)],
    )?;

    Ok(())
}

/// The text on the clipboard, `None` when it holds none.
fn read(env: &mut Env, manager: &JObject) -> Result<Option<String>> {
    let clip = env
        .call_method(
            manager,
            jni_str!("getPrimaryClip"),
            jni_sig!("()Landroid/content/ClipData;"),
            &[],
        )?
        .l()?;

    if clip.is_null() {
        return Ok(None);
    }

    let item = env
        .call_method(
            &clip,
            jni_str!("getItemAt"),
            jni_sig!("(I)Landroid/content/ClipData$Item;"),
            &[JValue::Int(0)],
        )?
        .l()?;

    let text = env
        .call_method(
            &item,
            jni_str!("getText"),
            jni_sig!("()Ljava/lang/CharSequence;"),
            &[],
        )?
        .l()?;

    if text.is_null() {
        return Ok(None);
    }

    let string = env
        .call_method(&text, jni_str!("toString"), jni_sig!("()Ljava/lang/String;"), &[])?
        .l()?;

    let string = env.cast_local::<JString>(string)?;

    Ok(Some(string.try_to_string(env)?))
}

pub(super) fn set_text(text: &str, sensitive: bool) -> Result<()> {
    with_activity(|env, activity| {
        let manager = manager(env, activity)?;
        put(env, &manager, text, sensitive)
    })
}

pub(super) fn get_text() -> Result<String> {
    with_activity(|env, activity| {
        let manager = manager(env, activity)?;
        read(env, &manager)?.ok_or_else(|| anyhow!("The clipboard holds no text"))
    })
}

/// Android gives the clipboard only to the app in front. A read from the
/// background comes back empty, so nothing is cleared then.
pub(super) fn clear_if_holds(text: &str) -> Result<bool> {
    with_activity(|env, activity| {
        let manager = manager(env, activity)?;

        let Some(held) = read(env, &manager)?.map(Zeroizing::new) else {
            return Ok(false);
        };
        if held.as_str() != text {
            return Ok(false);
        }

        // `clearPrimaryClip` exists only from API 28 and the engine runs
        // from API 26, an empty clip takes the secret off on both.
        put(env, &manager, "", false)?;
        Ok(true)
    })
}
