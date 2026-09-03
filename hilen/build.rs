use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};

fn main() {
    plat::platforms();
    stamp_build_time();
    fetch_ffmpeg();
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

/// The static ffmpeg the `video` feature links, see docs/video.md. The headers
/// are in git under `ffmpeg/include`, this fetches the libraries listed in
/// `ffmpeg/prebuilt.txt` for the target once into `ffmpeg/lib`, so a build
/// never compiles ffmpeg. The bindings crate reads both through `FFMPEG_DIR`,
/// set in `.cargo/config.toml`.
///
/// A miss is only a warning. `cargo check` and clippy link nothing, so they
/// pass offline, and a real build then fails on the missing archives.
fn fetch_ffmpeg() {
    if env::var("CARGO_FEATURE_VIDEO").is_err() {
        return;
    }

    let target = env::var("TARGET").expect("TARGET is set for build scripts");
    let root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set")).join("ffmpeg");
    let list = root.join("prebuilt.txt");
    println!("cargo:rerun-if-changed={}", list.display());

    let text = fs::read_to_string(&list).expect("ffmpeg/prebuilt.txt is readable");
    let Some((url, sha)) = prebuilt_for(&text, &target) else {
        println!("cargo:warning=no prebuilt ffmpeg for {target}, see docs/video.md");
        return;
    };

    let marker = root.join("lib").join("prebuilt.sha256");
    if fs::read_to_string(&marker).is_ok_and(|have| have.trim() == sha) {
        return;
    }

    match download(&url, &sha, &root) {
        Ok(()) => fs::write(&marker, &sha).expect("the ffmpeg marker is writable"),
        Err(err) => println!("cargo:warning=ffmpeg download failed, the link will miss its archives: {err}"),
    }
}

/// The url and sha256 of the archive for `target`, from the `<target> <url>
/// <sha256>` lines of the list.
fn prebuilt_for(list: &str, target: &str) -> Option<(String, String)> {
    list.lines().filter(|line| !line.starts_with('#')).find_map(|line| {
        let mut parts = line.split_whitespace();
        (parts.next()? == target).then(|| Some((parts.next()?.to_string(), parts.next()?.to_string())))?
    })
}

fn download(url: &str, sha: &str, root: &Path) -> Result<(), String> {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set")).join("ffmpeg.tar.gz");

    let status = Command::new("curl")
        .args(["-sSfL", "-o"])
        .arg(&out)
        .arg(url)
        .status()
        .map_err(|err| format!("curl did not start: {err}"))?;
    if !status.success() {
        return Err(format!("curl {url} exited with {status}"));
    }

    let bytes = fs::read(&out).map_err(|err| format!("reading the download: {err}"))?;
    let got = hex::encode(Sha256::digest(&bytes));
    if got != sha {
        return Err(format!("{url} has sha256 {got}, prebuilt.txt says {sha}"));
    }

    let lib = root.join("lib");
    if lib.exists() {
        fs::remove_dir_all(&lib).map_err(|err| format!("clearing {}: {err}", lib.display()))?;
    }
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&out)
        .arg("-C")
        .arg(root)
        .arg("lib")
        .status()
        .map_err(|err| format!("tar did not start: {err}"))?;
    if !status.success() {
        return Err(format!("tar exited with {status}"));
    }
    Ok(())
}
