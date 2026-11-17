//! The boilerplate every JNI call needs: attach to the VM and get the
//! activity object the app runs in.

use anyhow::Result;
use jni::{Env, JavaVM, objects::JObject};

pub(crate) fn with_activity<T>(action: impl FnOnce(&mut Env, &JObject) -> Result<T>) -> Result<T> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) };

    vm.attach_current_thread(|env| {
        let activity = activity(env);

        action(env, &activity)
    })
}

/// The activity on an env the thread already has, like the one a native
/// method is called with.
pub(crate) fn activity<'local>(env: &Env<'local>) -> JObject<'local> {
    unsafe { JObject::from_raw(env, ndk_context::android_context().context().cast()) }
}

/// A failed call leaves its Java exception pending. A pending exception
/// breaks every later call and is thrown at the Java caller of a native
/// method, so it goes to logcat and is cleared.
pub(crate) fn clear_exception(env: &Env) {
    if env.exception_check() {
        env.exception_describe();
        env.exception_clear();
    }
}
