//! The platform's own color emoji. Apple Color Emoji is an sbix font
//! that every Mac, iPhone and Apple TV carries, licensed for that
//! hardware alone, so it is mapped from disk at runtime and never
//! bundled. Other platforms have no such font and answer `None`.

use crate::{deps::refs::Weak, window::Font};

#[cfg(apple)]
pub(crate) fn system_emoji() -> Option<Weak<Font>> {
    use std::{
        fs::{File, read_dir},
        path::{Path, PathBuf},
    };

    use log::error;
    use memmap2::Mmap;

    use crate::deps::refs::manage::DataManager;

    /// Where the OS keeps its fonts. macOS has the file at the top,
    /// iOS a level down in `Core`.
    const FONTS_DIR: &str = "/System/Library/Fonts";
    const NAME: &str = "Apple Color Emoji";

    /// The first font file whose name is the emoji font's, directories
    /// in name order so `Core` wins over `CoreAddition` and its `160px`
    /// variant. Two levels are enough on every Apple OS so far.
    fn find(dir: &Path, depth: u8) -> Option<PathBuf> {
        let mut entries: Vec<PathBuf> = read_dir(dir).ok()?.flatten().map(|entry| entry.path()).collect();
        entries.sort();
        let is_emoji = |path: &Path| {
            let name = path.file_name()?.to_str()?;
            let stem = name.strip_suffix(".ttc").or_else(|| name.strip_suffix(".ttf"))?;
            Some(stem.replace(' ', "").starts_with("AppleColorEmoji"))
        };
        if let Some(file) = entries.iter().find(|path| path.is_file() && is_emoji(path) == Some(true)) {
            return Some(file.clone());
        }
        if depth == 0 {
            return None;
        }
        entries.iter().filter(|path| path.is_dir()).find_map(|dir| find(dir, depth - 1))
    }

    Font::store_with_name(NAME, || {
        let path = find(Path::new(FONTS_DIR), 2).ok_or_else(|| {
            error!("No {NAME} under {FONTS_DIR}");
            anyhow::anyhow!("no system emoji font")
        })?;
        // The file is around 190 MB on a Mac, mapping it costs nothing
        // until a glyph is read. The map lives until process exit like
        // every managed font's data.
        let file = File::open(&path)?;
        // SAFETY: a system font is read only and never changes while the
        // OS runs, so the mapped bytes stay valid.
        let map = unsafe { Mmap::map(&file)? };
        let data: &'static [u8] = Box::leak(Box::new(map));
        Font::from_static(NAME, data, &[], 0.0)
    })
    .ok()
}

#[cfg(not(apple))]
pub(crate) fn system_emoji() -> Option<Weak<Font>> {
    None
}
