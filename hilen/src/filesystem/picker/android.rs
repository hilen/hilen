//! The photo picker of Android 13 and later for an image,
//! `ACTION_GET_CONTENT` before it and for a file of any kind. Only Java
//! gets an activity result, so the launch goes through the
//! `ActivityResultRegistry` of the activity with a callback that
//! `java.lang.reflect.Proxy` makes over `io.hilen.NativeHandler`, the one
//! Java class of the engine, in `hilen/android`.

use anyhow::{Result, anyhow};
use jni::{
    Env, NativeMethod, jni_sig, jni_str, native_method,
    objects::{Global, JClass, JObject, JObjectArray, JString, JValue},
};
use log::{error, warn};
use parking_lot::Mutex;
use tokio::sync::oneshot::{Sender, channel};

use crate::{
    filesystem::picker::PickedFile,
    system::android_jni::{activity, clear_exception, with_activity},
};

/// Built by `make android-dex` from `hilen/android/NativeHandler.java`.
const DEX: &[u8] = include_bytes!("../../../android/native_handler.dex");

/// The photo picker action, `MediaStore.ACTION_PICK_IMAGES` from API 33.
const PICK_IMAGES: &str = "android.provider.action.PICK_IMAGES";
const PICK_IMAGES_API: i32 = 33;

/// `Activity.RESULT_OK`.
const RESULT_OK: i32 = -1;

const GET_CONTENT: &str = "android.intent.action.GET_CONTENT";
const ANY_TYPE: &str = "*/*";

const REGISTRY_KEY: &str = "hilen.pick";

/// Read in chunks, a content stream has no length to ask for.
const CHUNK: usize = 64 * 1024;

type Uri = Global<JObject<'static>>;

/// What the chooser offers.
#[derive(Clone, Copy)]
pub(super) enum Kind<'a> {
    Image,
    /// The MIME types a file may have, none for every file.
    File(&'a [String]),
}

impl Kind<'_> {
    /// The name of a pick whose provider tells none.
    fn fallback_name(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::File(_) => "file",
        }
    }
}

/// The loaded `NativeHandler` class, its native method registered.
static HANDLER: Mutex<Option<Global<JClass<'static>>>> = Mutex::new(None);
/// The waiting `pick`. A new pick drops the old sender, which answers None.
static ANSWER: Mutex<Option<Sender<Option<Uri>>>> = Mutex::new(None);
/// The intent the next `run` on the UI thread launches.
static INTENT: Mutex<Option<Global<JObject<'static>>>> = Mutex::new(None);
/// Unregistered once the result arrived.
static LAUNCHER: Mutex<Option<Global<JObject<'static>>>> = Mutex::new(None);

// The default check calls a std method this toolchain marks deprecated, which
// fails the warning free android build. `call` is an instance method in
// NativeHandler.java and is registered as one here, the check has nothing to
// find.
const CALL: NativeMethod = native_method! {
    fn call(method: JString, args: JObject[]),
    abi_check = UnsafeNever,
};

pub(super) async fn pick(title: &str, kind: Kind<'_>) -> Option<PickedFile> {
    let (answer, answered) = channel();
    *ANSWER.lock() = Some(answer);

    if let Err(err) =
        with_activity(|env, activity| start(env, activity, title, kind).inspect_err(|_| clear_exception(env)))
    {
        error!("picker failed to open: {err}");
        ANSWER.lock().take();
        return None;
    }

    let uri = answered.await.ok().flatten()?;

    with_activity(|env, activity| {
        read(env, activity, &uri, kind.fallback_name()).inspect_err(|_| clear_exception(env))
    })
    .inspect_err(|err| error!("picked file failed to read: {err}"))
    .ok()
}

/// Builds the intent here and launches it on the UI thread, where the
/// registry and `startActivityForResult` belong.
fn start(env: &mut Env, activity: &JObject, title: &str, kind: Kind<'_>) -> Result<()> {
    let intent = match kind {
        Kind::Image => image_intent(env, title)?,
        Kind::File(types) => file_intent(env, title, types)?,
    };
    *INTENT.lock() = Some(env.new_global_ref(&intent)?);

    let runnable = proxy(env, activity, "java.lang.Runnable")?;
    env.call_method(
        activity,
        jni_str!("runOnUiThread"),
        jni_sig!("(Ljava/lang/Runnable;)V"),
        &[JValue::Object(&runnable)],
    )?;

    Ok(())
}

fn image_intent<'local>(env: &mut Env<'local>, title: &str) -> Result<JObject<'local>> {
    let api = env
        .get_static_field(
            jni_str!("android/os/Build$VERSION"),
            jni_str!("SDK_INT"),
            jni_sig!("I"),
        )?
        .i()?;

    if api >= PICK_IMAGES_API {
        return typed_intent(env, PICK_IMAGES, "image/*");
    }

    let intent = typed_intent(env, GET_CONTENT, "image/*")?;
    chooser(env, &intent, title)
}

/// `ACTION_GET_CONTENT` filters by 1 type. More types go into
/// `EXTRA_MIME_TYPES` under the type of every file.
fn file_intent<'local>(env: &mut Env<'local>, title: &str, types: &[String]) -> Result<JObject<'local>> {
    let intent = match types {
        [one] => typed_intent(env, GET_CONTENT, one)?,
        _ => {
            let intent = typed_intent(env, GET_CONTENT, ANY_TYPE)?;
            if !types.is_empty() {
                put_mime_types(env, &intent, types)?;
            }
            intent
        }
    };

    chooser(env, &intent, title)
}

fn put_mime_types(env: &mut Env, intent: &JObject, types: &[String]) -> Result<()> {
    let null = JObject::null();
    let array = env.new_object_array(i32::try_from(types.len())?, jni_str!("java/lang/String"), &null)?;
    for (index, mime) in types.iter().enumerate() {
        let mime = env.new_string(mime)?;
        array.set_element(env, index, &mime)?;
    }

    let key = env.new_string("android.intent.extra.MIME_TYPES")?;
    env.call_method(
        intent,
        jni_str!("putExtra"),
        jni_sig!("(Ljava/lang/String;[Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(&key), JValue::Object(&array)],
    )?;

    Ok(())
}

fn typed_intent<'local>(env: &mut Env<'local>, action: &str, mime: &str) -> Result<JObject<'local>> {
    let action = env.new_string(action)?;
    let intent = env.new_object(
        jni_str!("android/content/Intent"),
        jni_sig!("(Ljava/lang/String;)V"),
        &[JValue::Object(&action)],
    )?;

    let mime = env.new_string(mime)?;
    env.call_method(
        &intent,
        jni_str!("setType"),
        jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(&mime)],
    )?;

    Ok(intent)
}

/// The chooser of the apps that can open a file of the intent.
fn chooser<'local>(env: &mut Env<'local>, intent: &JObject, title: &str) -> Result<JObject<'local>> {
    let openable = env.new_string("android.intent.category.OPENABLE")?;
    env.call_method(
        intent,
        jni_str!("addCategory"),
        jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(&openable)],
    )?;

    let title = env.new_string(title)?;
    Ok(env
        .call_static_method(
            jni_str!("android/content/Intent"),
            jni_str!("createChooser"),
            jni_sig!("(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;"),
            &[JValue::Object(intent), JValue::Object(&title)],
        )?
        .l()?)
}

/// The Java side of `NativeHandler`. It runs on the UI thread, for the
/// `Runnable` of `start` and then for the result callback. An error
/// here is logged and never thrown, a throw would end the app.
fn call<'local>(
    env: &mut Env<'local>,
    _: JObject<'local>,
    method: JString<'local>,
    args: JObjectArray<'local>,
) -> jni::errors::Result<()> {
    let method = method.try_to_string(env)?;

    let done = match method.as_str() {
        "run" => launch(env),
        "onActivityResult" => answer(env, &args),
        other => Err(anyhow!("unexpected call {other}")),
    };

    if let Err(err) = done {
        clear_exception(env);
        error!("picker {method} failed: {err}");
        // Nothing comes after a failed step, the waiting pick ends with None.
        ANSWER.lock().take();
    }

    Ok(())
}

fn launch(env: &mut Env) -> Result<()> {
    let intent = INTENT.lock().take().ok_or_else(|| anyhow!("no intent to launch"))?;
    let activity = activity(env);

    let registry = env
        .call_method(
            &activity,
            jni_str!("getActivityResultRegistry"),
            jni_sig!("()Landroidx/activity/result/ActivityResultRegistry;"),
            &[],
        )?
        .l()?;

    let loader = class_loader(env, &activity)?;
    let contract = load_class(
        env,
        &loader,
        "androidx.activity.result.contract.ActivityResultContracts$StartActivityForResult",
    )?;
    let contract = env.new_object(&contract, jni_sig!("()V"), &[])?;
    let callback = proxy(env, &activity, "androidx.activity.result.ActivityResultCallback")?;
    let key = env.new_string(REGISTRY_KEY)?;

    let launcher = env
        .call_method(
            &registry,
            jni_str!("register"),
            jni_sig!(
                "(Ljava/lang/String;Landroidx/activity/result/contract/ActivityResultContract;Landroidx/activity/result/ActivityResultCallback;)Landroidx/activity/result/ActivityResultLauncher;"
            ),
            &[
                JValue::Object(&key),
                JValue::Object(&contract),
                JValue::Object(&callback),
            ],
        )?
        .l()?;

    env.call_method(
        &launcher,
        jni_str!("launch"),
        jni_sig!("(Ljava/lang/Object;)V"),
        &[JValue::Object(&intent)],
    )?;

    *LAUNCHER.lock() = Some(env.new_global_ref(&launcher)?);
    Ok(())
}

/// The `ActivityResult` of the launch. A cancel has no data.
fn answer(env: &mut Env, args: &JObjectArray) -> Result<()> {
    let result = args.get_element(env, 0)?;
    let code = env.call_method(&result, jni_str!("getResultCode"), jni_sig!("()I"), &[])?.i()?;
    let data = env
        .call_method(
            &result,
            jni_str!("getData"),
            jni_sig!("()Landroid/content/Intent;"),
            &[],
        )?
        .l()?;

    let uri = if code == RESULT_OK && !data.is_null() {
        let uri = env
            .call_method(&data, jni_str!("getData"), jni_sig!("()Landroid/net/Uri;"), &[])?
            .l()?;
        (!uri.is_null()).then(|| env.new_global_ref(&uri)).transpose()?
    } else {
        None
    };

    if let Some(launcher) = LAUNCHER.lock().take() {
        env.call_method(&*launcher, jni_str!("unregister"), jni_sig!("()V"), &[])?;
    }

    if let Some(answer) = ANSWER.lock().take()
        && answer.send(uri).is_err()
    {
        warn!("file picked after the caller stopped waiting");
    }

    Ok(())
}

fn read(env: &mut Env, activity: &JObject, uri: &Uri, fallback_name: &str) -> Result<PickedFile> {
    let resolver = env
        .call_method(
            activity,
            jni_str!("getContentResolver"),
            jni_sig!("()Landroid/content/ContentResolver;"),
            &[],
        )?
        .l()?;

    let content_type = env
        .call_method(
            &resolver,
            jni_str!("getType"),
            jni_sig!("(Landroid/net/Uri;)Ljava/lang/String;"),
            &[JValue::Object(uri)],
        )?
        .l()?;
    let content_type = optional_string(env, content_type)?;

    let name = display_name(env, &resolver, uri)?.unwrap_or_else(|| fallback_name.to_string());

    let stream = env
        .call_method(
            &resolver,
            jni_str!("openInputStream"),
            jni_sig!("(Landroid/net/Uri;)Ljava/io/InputStream;"),
            &[JValue::Object(uri)],
        )?
        .l()?;
    if stream.is_null() {
        return Err(anyhow!("the provider gave no stream for {name}"));
    }

    let bytes = read_stream(env, &stream);
    env.call_method(&stream, jni_str!("close"), jni_sig!("()V"), &[])?;

    Ok(PickedFile::new(name, content_type, bytes?))
}

fn read_stream(env: &mut Env, stream: &JObject) -> Result<Vec<u8>> {
    let buffer = env.new_byte_array(CHUNK)?;
    let mut bytes = Vec::new();

    loop {
        let count = env
            .call_method(
                stream,
                jni_str!("read"),
                jni_sig!("([B)I"),
                &[JValue::Object(&buffer)],
            )?
            .i()?;
        // A negative count is the end of the stream.
        let Ok(count) = usize::try_from(count) else {
            return Ok(bytes);
        };
        bytes.extend_from_slice(&env.convert_byte_array(&buffer)?[..count]);
    }
}

/// `OpenableColumns.DISPLAY_NAME` of the picked file.
fn display_name(env: &mut Env, resolver: &JObject, uri: &Uri) -> Result<Option<String>> {
    let column = env.new_string("_display_name")?;
    let columns = env.new_object_array(1, jni_str!("java/lang/String"), &column)?;
    let null = JObject::null();

    let cursor = env
        .call_method(
            resolver,
            jni_str!("query"),
            jni_sig!(
                "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;"
            ),
            &[
                JValue::Object(uri),
                JValue::Object(&columns),
                JValue::Object(&null),
                JValue::Object(&null),
                JValue::Object(&null),
            ],
        )?
        .l()?;
    if cursor.is_null() {
        return Ok(None);
    }

    let name = if env.call_method(&cursor, jni_str!("moveToFirst"), jni_sig!("()Z"), &[])?.z()? {
        let name = env
            .call_method(
                &cursor,
                jni_str!("getString"),
                jni_sig!("(I)Ljava/lang/String;"),
                &[JValue::Int(0)],
            )?
            .l()?;
        optional_string(env, name)?
    } else {
        None
    };
    env.call_method(&cursor, jni_str!("close"), jni_sig!("()V"), &[])?;

    Ok(name)
}

fn optional_string(env: &mut Env, object: JObject) -> Result<Option<String>> {
    if object.is_null() {
        return Ok(None);
    }
    let string = env.cast_local::<JString>(object)?;
    Ok(Some(string.try_to_string(env)?))
}

/// An object of the interface, its calls go to `call`.
fn proxy<'local>(env: &mut Env<'local>, activity: &JObject, interface: &str) -> Result<JObject<'local>> {
    let loader = class_loader(env, activity)?;
    let interface = load_class(env, &loader, interface)?;
    let interfaces = env.new_object_array(1, jni_str!("java/lang/Class"), &interface)?;
    let handler = new_handler(env, &loader)?;

    Ok(env
        .call_static_method(
            jni_str!("java/lang/reflect/Proxy"),
            jni_str!("newProxyInstance"),
            jni_sig!(
                "(Ljava/lang/ClassLoader;[Ljava/lang/Class;Ljava/lang/reflect/InvocationHandler;)Ljava/lang/Object;"
            ),
            &[
                JValue::Object(&loader),
                JValue::Object(&interfaces),
                JValue::Object(&handler),
            ],
        )?
        .l()?)
}

/// A `NativeHandler`, its class loaded from the dex on first use.
fn new_handler<'local>(env: &mut Env<'local>, parent: &JObject) -> Result<JObject<'local>> {
    let mut handler = HANDLER.lock();

    if handler.is_none() {
        let bytes = env.byte_array_from_slice(DEX)?;
        let buffer = env
            .call_static_method(
                jni_str!("java/nio/ByteBuffer"),
                jni_str!("wrap"),
                jni_sig!("([B)Ljava/nio/ByteBuffer;"),
                &[JValue::Object(&bytes)],
            )?
            .l()?;
        let loader = env.new_object(
            jni_str!("dalvik/system/InMemoryDexClassLoader"),
            jni_sig!("(Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V"),
            &[JValue::Object(&buffer), JValue::Object(parent)],
        )?;
        let class = load_class(env, &loader, "io.hilen.NativeHandler")?;
        unsafe { env.register_native_methods(&class, &[CALL])? };
        *handler = Some(env.new_global_ref(&class)?);
    }

    let class = handler.as_ref().ok_or_else(|| anyhow!("NativeHandler is not loaded"))?;
    Ok(env.new_object(class, jni_sig!("()V"), &[])?)
}

fn class_loader<'local>(env: &mut Env<'local>, activity: &JObject) -> Result<JObject<'local>> {
    Ok(env
        .call_method(
            activity,
            jni_str!("getClassLoader"),
            jni_sig!("()Ljava/lang/ClassLoader;"),
            &[],
        )?
        .l()?)
}

/// By the given loader, the system loader of a native thread does not
/// see the classes of the app.
fn load_class<'local>(env: &mut Env<'local>, loader: &JObject, name: &str) -> Result<JClass<'local>> {
    let name = env.new_string(name)?;
    let class = env
        .call_method(
            loader,
            jni_str!("loadClass"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"),
            &[JValue::Object(&name)],
        )?
        .l()?;
    Ok(env.cast_local::<JClass>(class)?)
}
