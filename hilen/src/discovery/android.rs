//! The Wi-Fi multicast lock of Android. Without it the Wi-Fi of most phones
//! drops every multicast packet, so no mDNS answer reaches the app. It needs
//! the `CHANGE_WIFI_MULTICAST_STATE` permission, which the build puts into
//! the manifest for an app with a `[local_network]` table in `hilen.toml`.

use anyhow::{Context, Result};
use jni::{
    Env, jni_sig, jni_str,
    objects::{Global, JObject, JValue},
};
use log::{debug, warn};

use crate::system::android_jni::{clear_exception, with_activity};

const TAG: &str = "hilen-discovery";

/// Held while it lives.
pub(super) struct MulticastLock(Global<JObject<'static>>);

impl MulticastLock {
    /// `None` on a device with no Wi-Fi, there is nothing to unlock.
    pub(super) fn acquire() -> Result<Option<Self>> {
        with_activity(|env, activity| {
            let lock = acquire(env, activity).inspect_err(|_| clear_exception(env)).context(
                "Android gave no Wi-Fi multicast lock, the manifest needs CHANGE_WIFI_MULTICAST_STATE, add a [local_network] table to hilen.toml",
            )?;
            match lock {
                Some(lock) => Ok(Some(Self(env.new_global_ref(&lock)?))),
                None => {
                    debug!("No Wi-Fi on this device, the browse runs with no multicast lock");
                    Ok(None)
                }
            }
        })
    }
}

fn acquire<'local>(env: &mut Env<'local>, activity: &JObject) -> Result<Option<JObject<'local>>> {
    let context = env
        .call_method(
            activity,
            jni_str!("getApplicationContext"),
            jni_sig!("()Landroid/content/Context;"),
            &[],
        )?
        .l()?;
    let service = env.new_string("wifi")?;
    let wifi = env
        .call_method(
            &context,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[JValue::Object(&service)],
        )?
        .l()?;
    if wifi.is_null() {
        return Ok(None);
    }
    let tag = env.new_string(TAG)?;
    let lock = env
        .call_method(
            &wifi,
            jni_str!("createMulticastLock"),
            jni_sig!("(Ljava/lang/String;)Landroid/net/wifi/WifiManager$MulticastLock;"),
            &[JValue::Object(&tag)],
        )?
        .l()?;
    env.call_method(&lock, jni_str!("acquire"), jni_sig!("()V"), &[])?;
    Ok(Some(lock))
}

impl Drop for MulticastLock {
    fn drop(&mut self) {
        let released = with_activity(|env, _| {
            env.call_method(&*self.0, jni_str!("release"), jni_sig!("()V"), &[])
                .inspect_err(|_| clear_exception(env))?;
            Ok(())
        });
        if let Err(error) = released {
            warn!("The Wi-Fi multicast lock was not released: {error}");
        }
    }
}
