use std::{
    env::var,
    time::{SystemTime, UNIX_EPOCH},
};

include!("src/inspect/release_guard.rs");

fn main() {
    plat::platforms();
    stamp_build_time();
    rerun_rules();
    refuse_inspect_in_release();
}

/// One rerun line turns off the default rerun on any package file, which the
/// build time stamp depends on. The file lines bring that back.
fn rerun_rules() {
    println!("cargo:rerun-if-env-changed={RELEASE_ENV}");
    println!("cargo:rerun-if-changed=src");
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
