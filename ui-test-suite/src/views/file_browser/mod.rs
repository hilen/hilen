//! The tests of `FileBrowser`. They all browse the same tree, held in
//! memory, so they run the same on every platform.

mod file_browser_dark;
mod file_browser_grid;
mod file_browser_keys;
mod file_browser_list;
mod file_browser_loading;
mod file_browser_menu;
mod file_browser_modal;
mod file_browser_narrow;
mod file_browser_navigate;
mod file_browser_path;
mod file_browser_pick;
mod file_browser_search;
mod file_browser_select;
mod file_browser_sort;

use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{NaiveDate, NaiveDateTime};
use hilen::{
    dispatch::from_main,
    filesystem::{MemoryFiles, PlaceKind},
    refs::Weak,
    ui::{FileBrowser, FileBrowserControl, Point},
    ui_test::inject_touches,
};

fn at(day: u32, hour: u32, minute: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 10, day)
        .and_then(|date| date.and_hms_opt(hour, minute, 0))
        .unwrap_or_default()
}

/// A small disk: folders, files of every kind, hidden entries, a deep
/// path for the crumbs and a long folder for scrolling.
fn sample_tree() -> MemoryFiles {
    let mut tree = MemoryFiles::new("Disk")
        .file("Documents/report.pdf", 1_240_000)
        .file("Documents/notes.txt", 2_100)
        .file("Documents/budget.xlsx", 48_000)
        .file("Music/song.mp3", 5_300_000)
        .file("Photos/cat.png", 820_000)
        .file("Photos/dog.jpg", 1_900_000)
        .file("Photos/Trip/beach.png", 2_400_000)
        .file("Projects/engine/source/views/buttons/round.rs", 900)
        .folder("Empty")
        .file("archive.zip", 12_000_000)
        .file("main.rs", 3_400)
        .file("movie.mkv", 1_400_000_000)
        .file("readme.md", 890)
        .file(".env", 120)
        .hidden(".env")
        .folder(".cache")
        .hidden(".cache")
        .modified("Documents", at(1, 9, 30))
        .modified("Music", at(2, 18, 0))
        .modified("Photos", at(3, 12, 15))
        .modified("Projects", at(6, 14, 5))
        .modified("Empty", at(4, 8, 0))
        .modified("archive.zip", at(5, 23, 59))
        .modified("main.rs", at(6, 10, 1))
        .modified("movie.mkv", at(1, 20, 45))
        .modified("readme.md", at(2, 7, 7))
        .place("Photos", "Photos", PlaceKind::Folder)
        .place("Documents", "Documents", PlaceKind::Home);

    for number in 1..=40 {
        tree = tree.file(&format!("Many/file-{number:02}.txt"), 1000 * number);
    }
    tree
}

fn sample() -> Arc<MemoryFiles> {
    Arc::new(sample_tree())
}

fn tap_at(point: Point) {
    let (x, y) = (point.x.round(), point.y.round());
    inject_touches(format!("{x} {y} b\n{x} {y} e"));
}

/// Both taps go to the main thread in one trip, so they are a double tap
/// however slow the machine is.
fn double_tap_at(point: Point) {
    let (x, y) = (point.x.round(), point.y.round());
    inject_touches(format!("{x} {y} b\n{x} {y} e\n{x} {y} b\n{x} {y} e"));
}

fn row_center(browser: Weak<FileBrowser>, name: &'static str) -> Result<Point> {
    from_main(move || browser.row_center(name)).ok_or_else(|| anyhow!("no row {name} on screen"))
}

fn tap_row(browser: Weak<FileBrowser>, name: &'static str) -> Result<()> {
    tap_at(row_center(browser, name)?);
    Ok(())
}

fn double_tap_row(browser: Weak<FileBrowser>, name: &'static str) -> Result<()> {
    double_tap_at(row_center(browser, name)?);
    Ok(())
}

fn tap_control(browser: Weak<FileBrowser>, control: FileBrowserControl) {
    tap_at(from_main(move || browser.control_center(control)));
}

/// The names on screen, top to bottom.
fn names(browser: Weak<FileBrowser>) -> Vec<String> {
    from_main(move || browser.entries().iter().map(|entry| entry.name.clone()).collect())
}

/// The names of the picked entries.
fn picked(browser: Weak<FileBrowser>) -> Vec<String> {
    from_main(move || browser.selected().iter().map(|path| path.name().to_string()).collect())
}
