//! What btleplug needs on Android: its Java half, which the engine carries
//! as a dex, and the permissions the user gives at run time.

use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use jni::{
    Env, jni_sig, jni_str,
    objects::{JObject, JValue},
};
use log::info;
use parking_lot::Mutex;
use tokio::time::sleep;

use crate::{
    bluetooth::permissions::android_permissions,
    system::android_jni::{clear_exception, with_activity},
};

/// Built by `make android-btleplug-dex` from the Java sources of the
/// btleplug crate.
const DEX: &[u8] = include_bytes!("../../../android/btleplug.dex");

/// `PackageManager.PERMISSION_GRANTED`.
const GRANTED: i32 = 0;
/// The answer to the request goes to the activity, which does not pass it
/// on, so the code only has to be its own.
const REQUEST_CODE: i32 = 0x0B1E;
/// How long the user has to answer the system dialog.
const ANSWER_WAIT: Duration = Duration::from_secs(60);
const ANSWER_STEP: Duration = Duration::from_millis(250);

static LOADED: Mutex<bool> = Mutex::new(false);

/// Loads the Java classes of btleplug and starts it, once.
pub(super) fn load() -> Result<()> {
    let mut loaded = LOADED.lock();
    if *loaded {
        return Ok(());
    }
    with_activity(|env, activity| start(env, activity).inspect_err(|_| clear_exception(env)))?;
    *loaded = true;
    info!("BLE Java classes of btleplug are loaded");
    Ok(())
}

/// btleplug finds its classes through the class loader of the thread, so
/// the loader of the dex is that loader while it starts.
fn start(env: &mut Env, activity: &JObject) -> Result<()> {
    let bytes = env.byte_array_from_slice(DEX)?;
    let buffer = env
        .call_static_method(
            jni_str!("java/nio/ByteBuffer"),
            jni_str!("wrap"),
            jni_sig!("([B)Ljava/nio/ByteBuffer;"),
            &[JValue::Object(&bytes)],
        )?
        .l()?;
    let parent = env
        .call_method(
            activity,
            jni_str!("getClassLoader"),
            jni_sig!("()Ljava/lang/ClassLoader;"),
            &[],
        )?
        .l()?;
    let loader = env.new_object(
        jni_str!("dalvik/system/InMemoryDexClassLoader"),
        jni_sig!("(Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V"),
        &[JValue::Object(&buffer), JValue::Object(&parent)],
    )?;

    let thread = env
        .call_static_method(
            jni_str!("java/lang/Thread"),
            jni_str!("currentThread"),
            jni_sig!("()Ljava/lang/Thread;"),
            &[],
        )?
        .l()?;
    let previous = env
        .call_method(
            &thread,
            jni_str!("getContextClassLoader"),
            jni_sig!("()Ljava/lang/ClassLoader;"),
            &[],
        )?
        .l()?;
    set_loader(env, &thread, &loader)?;
    let started = btleplug::platform::init(env).map_err(|err| anyhow!("btleplug failed to start: {err}"));
    set_loader(env, &thread, &previous)?;
    started
}

fn set_loader(env: &mut Env, thread: &JObject, loader: &JObject) -> Result<()> {
    env.call_method(
        thread,
        jni_str!("setContextClassLoader"),
        jni_sig!("(Ljava/lang/ClassLoader;)V"),
        &[JValue::Object(loader)],
    )?;
    Ok(())
}

/// Asks the user for the permissions that are not given yet and waits for
/// the answer. The answer goes only to the activity, so the permissions
/// are checked again until they are given or the wait is over.
pub(super) async fn ask_permissions() -> Result<()> {
    let missing =
        with_activity(|env, activity| missing(env, activity).inspect_err(|_| clear_exception(env)))?;
    if missing.is_empty() {
        return Ok(());
    }
    info!("BLE asks for the permissions {missing:?}");
    with_activity(|env, activity| request(env, activity, &missing).inspect_err(|_| clear_exception(env)))?;

    let mut waited = Duration::ZERO;
    while waited < ANSWER_WAIT {
        sleep(ANSWER_STEP).await;
        waited += ANSWER_STEP;
        let still =
            with_activity(|env, activity| missing_now(env, activity).inspect_err(|_| clear_exception(env)))?;
        if !still {
            info!("BLE permissions are given");
            return Ok(());
        }
    }
    bail!("The Bluetooth permissions {missing:?} were not given")
}

fn missing(env: &mut Env, activity: &JObject) -> Result<Vec<&'static str>> {
    let api = env
        .get_static_field(
            jni_str!("android/os/Build$VERSION"),
            jni_str!("SDK_INT"),
            jni_sig!("I"),
        )?
        .i()?;
    let mut missing = Vec::new();
    for permission in android_permissions(api) {
        let name = env.new_string(permission)?;
        let state = env
            .call_method(
                activity,
                jni_str!("checkSelfPermission"),
                jni_sig!("(Ljava/lang/String;)I"),
                &[JValue::Object(&name)],
            )?
            .i()?;
        if state != GRANTED {
            missing.push(*permission);
        }
    }
    Ok(missing)
}

fn missing_now(env: &mut Env, activity: &JObject) -> Result<bool> {
    Ok(!missing(env, activity)?.is_empty())
}

fn request(env: &mut Env, activity: &JObject, permissions: &[&str]) -> Result<()> {
    let null = JObject::null();
    let array = env.new_object_array(
        i32::try_from(permissions.len())?,
        jni_str!("java/lang/String"),
        &null,
    )?;
    for (index, permission) in permissions.iter().enumerate() {
        let name = env.new_string(permission)?;
        array.set_element(env, index, &name)?;
    }
    env.call_method(
        activity,
        jni_str!("requestPermissions"),
        jni_sig!("([Ljava/lang/String;I)V"),
        &[JValue::Object(&array), JValue::Int(REQUEST_CODE)],
    )?;
    Ok(())
}
