/// The mark of a shipped build, the same one `hilen` reads. With it the build
/// fails when `HILEN_SESSION_KEY` is missing.
const RELEASE_ENV: &str = "HILEN_RELEASE";

fn main() {
    plat::platforms();
    println!("cargo:rerun-if-env-changed={RELEASE_ENV}");
    session_key::embed();
}

mod session_key {
    use std::env::{var, var_os};

    use super::RELEASE_ENV;

    const KEY_ENV: &str = "HILEN_SESSION_KEY";

    /// Stands in when the build has no key, so a plain `cargo run` still gets
    /// a working `SessionStore`. It is public, a file sealed with it is only
    /// hidden from a casual look.
    const DEV_KEY: &str = "hilen development session key, a release never ships with it";

    /// Hands the built in half of the `SessionStore` key to the compiler.
    /// `session_key.rs` reads it through `obfstr`, so the binary holds only
    /// its obfuscated bytes, a strings search or a hex editor finds noise.
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

        let key = from_env.unwrap_or_else(|| DEV_KEY.to_owned());
        assert!(
            !key.contains(['\n', '\r']),
            "{KEY_ENV} holds a line break, cargo cannot pass it to the compiler"
        );
        println!("cargo:rustc-env=HILEN_SESSION_KEY_BUILT={key}");
        println!("cargo:rustc-env=HILEN_SESSION_KEY_IS_DEV={is_dev}");
    }
}
