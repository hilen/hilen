//! The last fallback for a char no label font and no registered fallback
//! covers. The OS font folders are walked once, through fontdb, and every
//! face found there is asked for the char in a fixed order, so text from
//! the network in any script draws with some font instead of notdef.
//! A browser page cannot read the system fonts, so there it stays off.

use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Instant,
};

use fontdb::{Database, FaceInfo, Source, Style};
use log::{error, info};
use memmap2::Mmap;
use rustybuzz::ttf_parser::Face;

use crate::{
    deps::refs::{Weak, main_lock::MainLock, manage::DataManager},
    window::Font,
};

/// Asked first, in this order, so a common script lands on a font that
/// looks like UI text. Every other face follows by family name.
#[cfg(macos)]
const PREFERRED: &[&str] = &[
    "Helvetica Neue",
    "Helvetica",
    "Lucida Grande",
    "Geneva",
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Arial Unicode MS",
];
#[cfg(ios)]
const PREFERRED: &[&str] = &[
    "Helvetica Neue",
    "Helvetica",
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
];
#[cfg(win)]
const PREFERRED: &[&str] = &[
    "Segoe UI",
    "Arial",
    "Segoe UI Symbol",
    "Microsoft YaHei",
    "Yu Gothic",
    "Malgun Gothic",
    "Nirmala UI",
];
#[cfg(linux)]
const PREFERRED: &[&str] = &["Noto Sans", "DejaVu Sans", "Liberation Sans", "Noto Sans CJK SC"];
#[cfg(android)]
const PREFERRED: &[&str] = &["Roboto", "Noto Sans", "Noto Sans CJK"];

/// macOS draws a placeholder for every code point with it, it covers all
/// of Unicode and would hide that nothing real has the char.
const SKIPPED: &[&str] = &["LastResort"];

struct Candidate {
    name:  String,
    data:  &'static [u8],
    index: u32,
    face:  Face<'static>,
}

static ENABLED: AtomicBool = AtomicBool::new(true);
static FACES: OnceLock<Vec<Candidate>> = OnceLock::new();
static STARTED: AtomicBool = AtomicBool::new(false);
static FOUND: MainLock<HashMap<char, Option<Weak<Font>>>> = MainLock::new();
/// The fonts `FOUND` points at, each once.
static LOADED: MainLock<Vec<Weak<Font>>> = MainLock::new();

pub(crate) fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
    if enabled {
        start_index();
    }
}

/// The walk takes a noticeable moment on a machine with many fonts, so it
/// starts off the main thread at launch and the first lookup waits for it.
pub(crate) fn start_index() {
    if !STARTED.swap(true, Ordering::AcqRel) {
        thread::spawn(|| {
            FACES.get_or_init(index);
        });
    }
}

pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// The first system face that has a glyph for `char`, loaded as a managed
/// font on first use. The answer per char is kept, a miss included.
pub(crate) fn font_for(char: char) -> Option<Weak<Font>> {
    if !enabled() {
        return None;
    }

    if let Some(found) = FOUND.get_mut().get(&char) {
        return *found;
    }

    start_index();

    let found = FACES
        .wait()
        .iter()
        .find(|candidate| candidate.face.glyph_index(char).is_some())
        .and_then(|candidate| {
            Font::store_with_name(&candidate.name, || {
                Font::from_static(&candidate.name, candidate.data, candidate.index, &[], 0.0)
            })
            .inspect_err(|err| error!("System font {} failed to load: {err}", candidate.name))
            .ok()
        });

    if let Some(font) = found
        && !LOADED.iter().any(|loaded| loaded.name == font.name)
    {
        LOADED.get_mut().push(font);
    }

    FOUND.get_mut().insert(char, found);
    found
}

/// A system font with color glyphs was picked for some char, so labels
/// need the color glyph pass. Apple Color Emoji is one.
pub(crate) fn any_color() -> bool {
    enabled() && LOADED.iter().any(|font| font.is_ok() && font.has_color())
}

fn load(db: &mut Database) {
    #[cfg(desktop)]
    db.load_system_fonts();
    // fontdb takes iOS for a Linux and looks for fontconfig, and knows no
    // folder on Android.
    #[cfg(ios)]
    db.load_fonts_dir("/System/Library/Fonts");
    #[cfg(android)]
    db.load_fonts_dir("/system/fonts");
}

fn index() -> Vec<Candidate> {
    let start = Instant::now();

    let mut db = Database::new();
    load(&mut db);

    let mut faces: Vec<&FaceInfo> = db
        .faces()
        .filter(|face| !SKIPPED.contains(&face.post_script_name.as_str()))
        .collect();
    faces.sort_by_cached_key(|face| order(face));

    let mut files: HashMap<PathBuf, Option<&'static [u8]>> = HashMap::new();
    let candidates: Vec<Candidate> = faces
        .into_iter()
        .filter_map(|face| {
            let path = match &face.source {
                Source::File(path) | Source::SharedFile(path, _) => path,
                Source::Binary(_) => return None,
            };
            let data = (*files.entry(path.clone()).or_insert_with(|| map(path)))?;
            Some(Candidate {
                name: format!("system {}", face.post_script_name),
                data,
                index: face.index,
                face: Face::parse(data, face.index).ok()?,
            })
        })
        .collect();

    info!(
        "system fonts: {} faces in {} files, {} ms",
        candidates.len(),
        files.len(),
        start.elapsed().as_millis()
    );

    candidates
}

/// Preferred families first, then the upright regular faces, then the
/// nearest weight, then the family name so the order is the same on
/// every run.
fn order(face: &FaceInfo) -> (usize, bool, u16, String) {
    let family = face.families.first().map(|(name, _)| name.clone()).unwrap_or_default();
    let preferred = PREFERRED.iter().position(|name| *name == family).unwrap_or(PREFERRED.len());
    (
        preferred,
        face.style != Style::Normal,
        face.weight.0.abs_diff(400),
        family,
    )
}

/// A font file is mapped once for the life of the process, like every
/// managed font's data, and costs nothing until a glyph is read.
fn map(path: &Path) -> Option<&'static [u8]> {
    let file = File::open(path)
        .inspect_err(|err| error!("System font {} failed to open: {err}", path.display()))
        .ok()?;
    // SAFETY: a system font folder is read only for apps and a font file
    // does not change while it is installed, so the mapped bytes stay valid.
    let map = unsafe { Mmap::map(&file) }
        .inspect_err(|err| error!("System font {} failed to map: {err}", path.display()))
        .ok()?;
    let data: &'static Mmap = Box::leak(Box::new(map));
    Some(&data[..])
}
