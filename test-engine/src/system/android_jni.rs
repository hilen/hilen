//! The boilerplate every JNI call needs: attach to the VM and get the
//! activity object the app runs in.

use anyhow::Result;
use jni::{JNIEnv, objects::JObject};

pub(crate) fn with_activity<T>(action: impl FnOnce(&mut JNIEnv, &JObject) -> Result<T>) -> Result<T> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
    let mut env = vm.attach_current_thread()?;
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    action(&mut env, &activity)
}
