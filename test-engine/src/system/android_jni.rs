//! The boilerplate every JNI call needs: attach to the VM and get the
//! activity object the app runs in.

use anyhow::Result;
use jni::{Env, JavaVM, objects::JObject};

pub(crate) fn with_activity<T>(action: impl FnOnce(&mut Env, &JObject) -> Result<T>) -> Result<T> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) };

    vm.attach_current_thread(|env| {
        let activity = unsafe { JObject::from_raw(env, ctx.context().cast()) };

        action(env, &activity)
    })
}
