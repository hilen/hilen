/// The mark of a shipped build, the same one `hilen` reads. With it the build
/// fails when `HILEN_SESSION_KEY` is missing.
const RELEASE_ENV: &str = "HILEN_RELEASE";

fn main() {
    plat::platforms();
    println!("cargo:rerun-if-env-changed={RELEASE_ENV}");
    session_key::embed();
}

mod session_key {
    use std::{
        env::{var, var_os},
        fmt::Write,
        fs::write,
        path::PathBuf,
    };

    use super::RELEASE_ENV;

    const KEY_ENV: &str = "HILEN_SESSION_KEY";

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
        println!("cargo:rerun-if-env-changed={KEY_ENV}");

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
