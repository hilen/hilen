use anyhow::{Result, anyhow};
use hilen::{
    dispatch::from_main,
    filesystem::FilePath,
    refs::Weak,
    ui::{
        ContextMenu, FileBrowser, FileBrowserControl, FileBrowserMode, Label, Modifiers, Point, Setup,
        TextAlignment, ViewData, ViewTest, view,
    },
    ui_test::{checkpoint, inject_modifiers, inject_right_click},
};

use super::{double_tap_row, names, picked, sample, tap_at, tap_control, tap_row};

/// `selection_changed` also fires when the browser drops the picked rows
/// itself, on a folder change and on a mode change, so an app that shows
/// the picked rows never keeps the old ones. A change, a tap or a right
/// click on the empty area with nothing picked fires nothing, and neither
/// does the release of a double tap that opened a folder with no row
/// under it.
#[view]
struct FileBrowserSelectionEvent {
    events: usize,

    #[init]
    browser: FileBrowser,
    status:  Label,
    counter: Label,
}

impl Setup for FileBrowserSelectionEvent {
    fn setup(mut self: Weak<Self>) {
        self.browser.place().t(0).lr(0).b(80);
        self.browser.set_open_on_tap(false).set_source(sample());
        self.browser.selection_changed.val(move |paths| {
            let names: Vec<&str> = paths.iter().map(FilePath::name).collect();
            self.events += 1;
            self.status.set_text(format!("picked: {}", names.join(", ")));
            self.counter.set_text(format!("events: {}", self.events));
        });

        for (label, text, bottom) in [(self.status, "picked:", 40), (self.counter, "events: 0", 0)] {
            label.set_text(text).set_text_size(16);
            label.set_alignment(TextAlignment::Left);
            label.place().b(bottom).lr(12).h(40);
        }
    }
}

impl ViewTest for FileBrowserSelectionEvent {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;
        let seen = move || from_main(move || (view.status.text().to_string(), view.events));

        tap_row(browser, "Music")?;
        inject_modifiers(Modifiers::META);
        tap_row(browser, "Photos")?;
        inject_modifiers(Modifiers::empty());
        assert_eq!(seen(), ("picked: Music, Photos".to_string(), 2));
        checkpoint("Music and Photos picked, 2 events")?;

        // The first tap picks Many, the second opens it and drops the pick.
        double_tap_row(browser, "Many")?;
        assert_eq!(names(browser).len(), 40);
        assert_eq!(picked(browser), Vec::<String>::new());
        assert_eq!(seen(), ("picked: ".to_string(), 4));
        checkpoint("a double tap opened Many, nothing is picked, 4 events")?;

        tap_control(browser, FileBrowserControl::Back);
        assert_eq!(names(browser).len(), 10);
        assert_eq!(seen(), ("picked: ".to_string(), 4));
        checkpoint("back to Disk with nothing picked, still 4 events")?;

        tap_at(empty_point(browser)?);
        assert_eq!(seen(), ("picked: ".to_string(), 4));
        right_click_empty(browser)?;
        assert_eq!(seen(), ("picked: ".to_string(), 4));

        // Photos has 3 rows, so the release lands on the empty area of the
        // new folder.
        double_tap_row(browser, "Photos")?;
        assert_eq!(names(browser), ["Trip", "cat.png", "dog.jpg"]);
        assert_eq!(seen(), ("picked: ".to_string(), 6));
        checkpoint("a double tap opened Photos, only its pick and its drop, 6 events")?;

        tap_row(browser, "cat.png")?;
        tap_at(empty_point(browser)?);
        assert_eq!(seen(), ("picked: ".to_string(), 8));
        tap_row(browser, "cat.png")?;
        assert_eq!(seen(), ("picked: cat.png".to_string(), 9));
        right_click_empty(browser)?;
        assert_eq!(seen(), ("picked: ".to_string(), 10));
        tap_row(browser, "cat.png")?;
        assert_eq!(seen(), ("picked: cat.png".to_string(), 11));

        // The drop comes before the listing, so the folder that up picks
        // is the last word.
        tap_control(browser, FileBrowserControl::Up);
        assert_eq!(picked(browser), ["Photos"]);
        assert_eq!(seen(), ("picked: Photos".to_string(), 13));
        checkpoint("up from Photos with cat.png picked, Photos is picked, 13 events")?;

        from_main(move || {
            browser.set_mode(FileBrowserMode::PickFolder);
        });
        assert_eq!(picked(browser), Vec::<String>::new());
        assert_eq!(seen(), ("picked: ".to_string(), 14));

        from_main(move || {
            browser.set_mode(FileBrowserMode::PickFile);
        });
        assert_eq!(seen(), ("picked: ".to_string(), 14));
        checkpoint("2 mode changes, the first dropped Photos, 14 events")?;

        Ok(())
    }
}

/// A point of the empty area, under the last row.
fn empty_point(browser: Weak<FileBrowser>) -> Result<Point> {
    from_main(move || {
        let last = browser.entries().last()?.name.clone();
        browser.row_center(&last).map(|center| Point::new(center.x, center.y + 100.0))
    })
    .ok_or_else(|| anyhow!("no last row on screen"))
}

/// A right click on the empty area. The menu it opens is closed again, the
/// test is about the event.
fn right_click_empty(browser: Weak<FileBrowser>) -> Result<()> {
    let point = empty_point(browser)?;
    inject_right_click(point.x, point.y);
    from_main(ContextMenu::dismiss_open);
    Ok(())
}
