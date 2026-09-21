use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    plat::platforms();
    stamp_build_time();

    #[cfg(feature = "login")]
    session_key::embed();
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

#[cfg(feature = "login")]
mod session_key {
    use std::{
        env::{var, var_os},
        fmt::Write,
        fs::write,
        path::PathBuf,
    };

    const KEY_ENV: &str = "HILEN_SESSION_KEY";
    const RELEASE_ENV: &str = "HILEN_RELEASE";

    /// Stands in when the build has no key, so a plain `cargo run` still gets
    /// a working `SessionStore`. It is public, a file sealed with it is only
    /// hidden from a casual look.
    const DEV_KEY: &str = "hilen development session key, a release never ships with it";

    /// Writes the built in half of the `SessionStore` key into `OUT_DIR`,
    /// masked.
    ///
    /// The key never reaches the binary as plain bytes. The file holds the key
    /// xored with a random mask, the mask itself, and the shift between the
    /// two, so a strings search or a hex editor finds only noise.
    /// `store/session_key.rs` puts it back together at run time.
    pub fn embed() {
        // One rerun line turns off the default rerun on any package file, which
        // the build time stamp depends on. The file lines bring that back.
        println!("cargo:rerun-if-env-changed={KEY_ENV}");
        println!("cargo:rerun-if-env-changed={RELEASE_ENV}");
        println!("cargo:rerun-if-changed=src");
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed=Cargo.toml");

        let from_env = var(KEY_ENV).ok().filter(|key| !key.is_empty());
        let release = var_os(RELEASE_ENV).is_some_and(|mark| !mark.is_empty());

        assert!(
            from_env.is_some() || !release,
            "{KEY_ENV} is not set. A release of an app with the hilen `login` feature needs it, add it to the \
             Infisical project of the app."
        );

        let is_dev = from_env.is_none();
        if is_dev {
            println!("cargo:warning={KEY_ENV} is not set, SessionStore uses the public development key");
        }

        let key = from_env.unwrap_or_else(|| DEV_KEY.to_owned()).into_bytes();

        let mut mask = vec![0; key.len()];
        getrandom::fill(&mut mask).expect("no random bytes from the system");
        let mut shift = [0; 1];
        getrandom::fill(&mut shift).expect("no random bytes from the system");
        let shift = usize::from(shift[0]) % key.len();

        let masked: Vec<u8> = key
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ mask[(index + shift) % mask.len()])
            .collect();

        let mut code = String::new();
        writeln!(code, "const MASKED: [u8; {}] = {masked:?};", masked.len()).expect("write to a String");
        writeln!(code, "const MASK: [u8; {}] = {mask:?};", mask.len()).expect("write to a String");
        writeln!(code, "const SHIFT: usize = {shift};").expect("write to a String");
        writeln!(code, "const IS_DEV: bool = {is_dev};").expect("write to a String");

        let out = PathBuf::from(var("OUT_DIR").expect("cargo sets OUT_DIR")).join("session_key.rs");
        write(&out, code).expect("Failed to write session_key.rs");
    }
}
