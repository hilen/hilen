use std::{
    env::var,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

include!("src/inspect/release_guard.rs");

fn main() {
    plat::platforms();
    stamp_build_time();
    rerun_rules();
    refuse_inspect_in_release();
    compile_ios_text_field();
}

/// The system text field of an iPhone is Objective-C, see
/// `native/ios/hilen_text.m`. It is compiled into this library, so an app
/// needs nothing for it in its Xcode project. The Apple TV shell has no
/// system text field.
fn compile_ios_text_field() {
    if var("CARGO_CFG_TARGET_OS").as_deref() != Ok("ios") {
        return;
    }

    // The compiler of the selected Xcode, asked from `xcrun`. The first
    // `clang` on the PATH can be the one of the Command Line Tools, older
    // than the iOS SDK, and it then fails on the modules of that SDK.
    let found = Command::new("xcrun")
        .args(["--find", "clang"])
        .output()
        .expect("xcrun did not start, an iOS build needs Xcode");
    assert!(found.status.success(), "xcrun found no clang");
    let clang = String::from_utf8(found.stdout).expect("the path of clang is not UTF-8");

    cc::Build::new()
        .compiler(clang.trim())
        .file("native/ios/hilen_text.m")
        // The file hands objects between C and Objective-C with ARC casts.
        .flag("-fobjc-arc")
        // With modules the object file names the frameworks it needs, and
        // the link of the app picks them up from there.
        .flag("-fmodules")
        .compile("hilen_text");
}

/// One rerun line turns off the default rerun on any package file, which the
/// build time stamp depends on. The file lines bring that back.
fn rerun_rules() {
    println!("cargo:rerun-if-env-changed={RELEASE_ENV}");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
}

/// A shipped build with the inspect server stops here. The runtime check in
/// `InspectService::start_listening` and the file scan in the release scripts
/// catch the builds that never set the mark.
fn refuse_inspect_in_release() {
    let mark = var(RELEASE_ENV).ok();

    if let Some(error) = inspect_release_error(cfg!(feature = "inspect"), mark.as_deref()) {
        panic!("{error}");
    }
}

/// Stamps when this crate was last compiled, which `hilen-inspect build-time`
/// reads back off a running app.
///
/// The link time of the app bundle is not the same thing and cannot replace
/// this. An iOS build relinks the bundle every time while happily reusing a
/// stale `libdemo.a`, so the binary looks freshly built, runs old code,
/// and every test against it is a lie. This stamp lives inside the Rust code,
/// so it only moves when the Rust code is really rebuilt.
fn stamp_build_time() {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock is before the unix epoch")
        .as_secs();

    println!("cargo:rustc-env=HILEN_BUILD_TIME={seconds}");
}
