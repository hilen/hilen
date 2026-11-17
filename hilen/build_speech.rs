//! The libraries of sherpa-onnx in an iOS build, see `docs/speech.md`.
//!
//! On iOS `sherpa-onnx-sys` links the C API only as the dynamic
//! `SherpaOnnxC.framework`. The engine is a staticlib there, which carries no
//! dynamic library, and the link of the app then found none of the symbols.
//! The static builds of the same release go into this crate in its place, so
//! the staticlib holds them.

use std::{
    env::var,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct Archive {
    url:         &'static str,
    sha256:      &'static str,
    xcframework: &'static str,
    binary:      &'static str,
    lib:         &'static str,
}

/// The release of `sherpa-onnx` in the root `Cargo.toml`, a newer pin needs
/// these 2 archives of its release. onnxruntime is the version the static
/// sherpa-onnx of that release was built against, in its `build-ios.sh`.
const ARCHIVES: [Archive; 2] = [
    Archive {
        url:         "https://github.com/k2-fsa/sherpa-onnx/releases/download/xcframework/sherpa-onnx-v2.0.1-ios-static.xcframework.zip",
        sha256:      "88aa569f71b7903e0bbdfbe6e2e114b3b95862b1ef0759eea3024f82926d7374",
        xcframework: "sherpa-onnx.xcframework",
        binary:      "SherpaOnnxC.framework/SherpaOnnxC",
        lib:         "hilen_sherpa_onnx",
    },
    Archive {
        url:         "https://github.com/csukuangfj/onnxruntime-libs/releases/download/v1.28.2/onnxruntime-ios-static-xcframework-1.28.2.xcframework.zip",
        sha256:      "2c2299acbb461d26d4bac4bc85985d40e7c7177ed6072703ae0846d88b0b4599",
        xcframework: "onnxruntime.xcframework",
        binary:      "onnxruntime.framework/onnxruntime",
        lib:         "hilen_onnxruntime",
    },
];

pub fn link() {
    println!("cargo:rerun-if-changed=build_speech.rs");
    if var("CARGO_CFG_TARGET_OS").as_deref() != Ok("ios") || var("CARGO_FEATURE_SPEECH").is_err() {
        return;
    }

    let target = var("TARGET").expect("cargo sets TARGET");
    let arch = match var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        other => panic!("sherpa-onnx has no iOS library for the arch {other:?}"),
    };
    let slice = if target.ends_with("-sim") || arch == "x86_64" {
        "ios-arm64_x86_64-simulator"
    } else {
        "ios-arm64"
    };

    let out = PathBuf::from(var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let cache = target_dir(&out).join("hilen-speech-ios");
    for archive in &ARCHIVES {
        let folder = fetch(archive, &cache);
        let binary = folder.join(archive.xcframework).join(slice).join(archive.binary);
        thin(&binary, arch, &out.join(format!("lib{}.a", archive.lib)));
        println!("cargo:rustc-link-lib=static={}", archive.lib);
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    // A staticlib only names these, the app links them with the flags of
    // `build/py/shared/ios.py`. A hot build links them itself.
    println!("cargo:rustc-link-lib=c++");
    println!("cargo:rustc-link-lib=framework=CoreML");
}

/// The cache is shared by every target of the folder, the device and the
/// simulator builds unpack each archive once.
fn target_dir(out: &Path) -> PathBuf {
    if let Ok(dir) = var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir);
    }
    out.ancestors()
        .find(|path| path.file_name().is_some_and(|name| name == "target"))
        .unwrap_or(out)
        .to_path_buf()
}

/// The unpacked archive. Both the download and the unpack land under a
/// temporary name first, so a build that was stopped leaves no half folder.
fn fetch(archive: &Archive, cache: &Path) -> PathBuf {
    let name = archive.url.rsplit('/').next().expect("the url names a file");
    let folder = cache.join(name.trim_end_matches(".zip"));
    if folder.is_dir() {
        return folder;
    }
    fs::create_dir_all(cache).expect("the cache folder of the speech libraries");

    let zip = cache.join(name);
    if !zip.is_file() || sha256(&zip) != archive.sha256 {
        let part = cache.join(format!("{name}.part"));
        run(Command::new("curl")
            .args(["--fail", "--location", "--silent", "--show-error", "--output"])
            .arg(&part)
            .arg(archive.url));
        let sum = sha256(&part);
        assert_eq!(sum, archive.sha256, "{} has a wrong sha256", archive.url);
        fs::rename(&part, &zip).expect("rename of the downloaded archive");
    }

    let unpack = cache.join(format!("{name}.unpack"));
    if unpack.exists() {
        fs::remove_dir_all(&unpack).expect("removal of an old unpack");
    }
    run(Command::new("ditto").args(["-x", "-k"]).arg(&zip).arg(&unpack));
    fs::rename(&unpack, &folder).expect("rename of the unpacked archive");
    fs::remove_file(&zip).expect("removal of the unpacked zip");
    folder
}

fn sha256(file: &Path) -> String {
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(file)
        .output()
        .expect("shasum did not start");
    assert!(output.status.success(), "shasum failed on {}", file.display());
    let text = String::from_utf8(output.stdout).expect("the output of shasum is not UTF-8");
    text.split_whitespace().next().unwrap_or_default().to_string()
}

/// rustc takes an archive of 1 arch only, the simulator slices hold 2 and
/// the device slice of onnxruntime is a fat file with 1.
fn thin(binary: &Path, arch: &str, lib: &Path) {
    let info = Command::new("xcrun")
        .arg("lipo")
        .arg("-info")
        .arg(binary)
        .output()
        .expect("lipo did not start");
    assert!(info.status.success(), "lipo -info failed on {}", binary.display());
    if String::from_utf8_lossy(&info.stdout).starts_with("Non-fat") {
        fs::copy(binary, lib).expect("copy of the speech library");
        return;
    }
    run(Command::new("xcrun")
        .arg("lipo")
        .arg(binary)
        .args(["-thin", arch, "-output"])
        .arg(lib));
}

fn run(command: &mut Command) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("{command:?} did not start: {error}"));
    assert!(status.success(), "{command:?} failed with {status}");
}
