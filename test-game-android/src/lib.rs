// The lib gates its `register_app!` behind `cfg(ios)`, and the weak
// `test_engine_create_app` stub in an rlib would not be overridden from
// there anyway. The final crate must register, like the desktop binary.
test_engine::register_app!(test_game::TestGameApp);

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: test_game::test_engine::AndroidApp) {
    test_game::test_engine::test_engine_start_app(app);
}

use jni::{
    EnvUnowned,
    objects::{JClass, JString},
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_example_test_1game_MainActivity_setFilesDir<'local>(
    mut env: EnvUnowned<'local>,
    _: JClass<'local>,
    input: JString<'local>,
) {
    env.with_env(|env| -> Result<(), jni::errors::Error> {
        let input: String = input.try_to_string(env)?;
        test_game::test_engine::filesystem::Paths::set_storage_path(input);
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_example_test_1game_MainActivity_setAssetManager<'local>(
    _env: EnvUnowned<'local>,
    _: JClass<'local>,
    _input: JClass<'local>,
) {
    dbg!("Java_com_example_test_1game_MainActivity_setAssetManager");
}
