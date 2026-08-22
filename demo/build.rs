use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    plat::platforms();

    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        generate_asset_manifest();
    }
}

/// Writes `assets/assets.json` for the browser build. Every image,
/// font and sound gets a content hash and a load group, and the
/// engine downloads whole groups from it, see
/// `hilen/src/assets.rs`. The group is the first folder under
/// the kind folder, files at the kind root are `boot`. Native builds
/// read assets from disk and skip this.
fn generate_asset_manifest() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set"));
    let assets = crate_dir.parent().expect("Crate dir has no parent").join("assets");

    let mut entries = Vec::new();

    for kind in ["images", "fonts", "sounds"] {
        let root = assets.join(kind);
        let mut files = Vec::new();
        collect(&root, &mut files);

        for file in files {
            println!("cargo:rerun-if-changed={}", file.display());

            let name = file
                .strip_prefix(&root)
                .expect("Walked file is under its root")
                .to_string_lossy()
                .replace('\\', "/");

            let group = name.find('/').map_or_else(|| "boot".to_string(), |i| name[..i].to_string());

            let data =
                fs::read(&file).unwrap_or_else(|err| panic!("Failed to read {}: {err}", file.display()));

            entries.push((kind.to_string(), name, group, fnv1a(&data)));
        }

        println!("cargo:rerun-if-changed={}", root.display());
    }

    entries.sort();

    let mut json = String::from("{\n  \"files\": [\n");
    for (i, (kind, name, group, hash)) in entries.iter().enumerate() {
        let comma = if i + 1 == entries.len() { "" } else { "," };
        json.push_str(&format!(
            "    {{ \"kind\": \"{kind}\", \"name\": \"{name}\", \"group\": \"{group}\", \"hash\": \"{hash}\" }}{comma}\n"
        ));
    }
    json.push_str("  ]\n}\n");

    let path = assets.join("assets.json");

    // A rewrite with identical content would still retrigger the trunk
    // watcher and loop `make serve` forever.
    if fs::read(&path).ok().as_deref() != Some(json.as_bytes()) {
        fs::write(&path, json).expect("Failed to write assets.json");
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("Failed to read {}: {err}", dir.display()));

    for entry in entries {
        let path = entry.expect("Failed to read dir entry").path();

        let hidden = path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'));
        if hidden {
            continue;
        }

        if path.is_dir() {
            collect(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn fnv1a(data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}
