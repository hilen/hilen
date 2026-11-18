use std::sync::Arc;

use anyhow::{Result, anyhow};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        ContextMenu, FileBrowser, Label, MenuItem, Point, Setup, TextAlignment, ViewData, ViewFrame,
        ViewTest, view,
    },
    ui_test::{
        check_colors, checkpoint, inject_right_click, inject_touches,
        system_input::{press_return, type_text, wait_until},
    },
};

use super::{names, picked, row_center, sample_tree, tap_at};

/// The right click menu. On an entry: Open, the entries of the app, and
/// Rename and Delete because this source can write. On the empty area:
/// New Folder. Each change shows in the list once the source did it.
#[view]
struct FileBrowserMenu {
    #[init]
    browser: FileBrowser,
    status:  Label,
}

impl Setup for FileBrowserMenu {
    fn setup(self: Weak<Self>) {
        self.browser.place().t(0).lr(0).b(40);
        self.browser
            .set_open_on_tap(false)
            .set_source(Arc::new(sample_tree().writable()))
            .set_menu_items(move |paths| {
                let Some(first) = paths.first().cloned() else {
                    return vec![];
                };
                vec![MenuItem::new("Copy Path", move || {
                    self.status.set_text(format!("copy path: {}", first.slash_text()));
                })]
            });

        self.status.set_text("no menu entry was used yet").set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().b(0).lr(12).h(40);
    }
}

fn menu_titles() -> Vec<String> {
    from_main(|| {
        let menu = ContextMenu::open();
        if menu.is_null() {
            return vec![];
        }
        menu.items().iter().map(|item| item.title().to_string()).collect()
    })
}

fn tap_menu_item(title: &'static str) -> Result<()> {
    let center = from_main(move || {
        let menu = ContextMenu::open();
        if menu.is_null() {
            return None;
        }
        let item = menu.items().iter().find(|item| item.title() == title)?;
        Some(item.absolute_frame().center())
    });
    tap_at(center.ok_or_else(|| anyhow!("no menu entry {title}"))?);
    Ok(())
}

fn right_click(point: Point) {
    inject_right_click(point.x, point.y);
}

impl ViewTest for FileBrowserMenu {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let browser = view.browser;
        let has = move |name: &'static str| names(browser).iter().any(|entry| entry == name);

        // A right click picks the row it lands on.
        right_click(row_center(browser, "main.rs")?);
        assert_eq!(picked(browser), ["main.rs"]);
        assert_eq!(menu_titles(), ["Open", "Copy Path", "Rename", "Delete"]);
        check_colors(CHECK_1)?;
        tap_menu_item("Copy Path")?;
        assert_eq!(view.status.text(), "copy path: Disk/main.rs");
        assert_eq!(menu_titles(), Vec::<String>::new());

        // The empty area offers a new folder. The dialog starts with a
        // name, the typed text goes after it.
        let empty = row_center(browser, "readme.md")?;
        right_click(Point::new(empty.x, empty.y + 100.0));
        assert_eq!(picked(browser), Vec::<String>::new());
        assert_eq!(menu_titles(), ["New Folder"]);
        tap_menu_item("New Folder")?;
        checkpoint("the dialog asks for the name of the new folder")?;
        type_text("2")?;
        press_return()?;
        wait_until("the new folder is listed", move || {
            browser.entries().iter().any(|entry| entry.name == "New Folder2")
        })?;
        check_colors(CHECK_2)?;

        right_click(row_center(browser, "New Folder2")?);
        tap_menu_item("Rename")?;
        checkpoint("the dialog holds the old name")?;
        type_text("b")?;
        press_return()?;
        wait_until("the folder has its new name", move || {
            browser.entries().iter().any(|entry| entry.name == "New Folder2b")
        })?;
        assert!(!has("New Folder2"));

        // Delete asks first, the right button of the question is the yes.
        right_click(row_center(browser, "New Folder2b")?);
        tap_menu_item("Delete")?;
        check_colors(CHECK_3)?;
        inject_touches("367 345 b\n367 345 e");
        wait_until("the folder is gone", move || {
            !browser.entries().iter().any(|entry| entry.name == "New Folder2b")
        })?;
        assert_eq!(names(browser).len(), 10);
        check_colors(CHECK_4)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
              376   16 - #d9d9de
              124   24 - #676768
              420   24 - #c3c3c9
              228   56 - #1c1c1e
              516   56 - #6e6e76
              236   84 - #404041
               24   88 - #dcdce3
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              504  176 - #ffffff
              184  204 - #ffffff
              444  260 - #85858c
              240  268 - #ffffff
              356  280 - #0a6cff
              184  292 - #0a6cff
              232  296 - #aecfff
              540  300 - #0858d0
              540  304 - #0853c4
              296  308 - #0a6cff
              540  312 - #c3c3c3
              444  352 - #282828
              248  356 - #e4e4e5
              540  368 - #c3c3c3
                4  380 - #f4f4f6
              540  428 - #c3c3c3
              376  432 - #e0e0e0
              592  556 - #ffffff
              400  568 - #597c95
               40  580 - #597c95
              104  580 - #000000
              208  584 - #000000
              496  592 - #597c95
";

const CHECK_2: &str = r"
              116    8 - #d9d9de
              320    8 - #d9d9de
              420   24 - #c3c3c9
              228   56 - #1c1c1e
              516   56 - #6e6e76
               52   84 - #dcdce3
              236   84 - #404041
               24   88 - #dcdce3
              148   92 - #dcdce3
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              372  164 - #ffffff
              236  236 - #373738
              504  236 - #ffffff
              228  296 - #ffffff
              432  296 - #7e7e85
              540  296 - #717179
                4  348 - #f4f4f6
              532  356 - #99999e
              184  380 - #ffffff
              276  384 - #1c1c1e
              420  384 - #6e6e76
              540  384 - #949499
              592  552 - #ffffff
              304  560 - #597c95
               32  580 - #597c95
               88  580 - #597c95
              144  580 - #000000
              176  580 - #000000
              184  580 - #000000
              424  592 - #597c95
              512  592 - #597c95
";

const CHECK_3: &str = r"
              388   20 - #bfbfbf
              420   24 - #929297
              228   56 - #151517
              516   56 - #535359
              236   84 - #303031
               24   88 - #a5a5aa
              232  120 - #2c2c2d
               76  144 - #151517
              324  192 - #0851bf
              184  204 - #0851bf
              500  208 - #0851bf
              588  216 - #0851bf
              428  252 - #f9f9f9
              260  256 - #f9f9f9
              216  280 - #f9f9f9
              348  280 - #f9f9f9
              532  296 - #bfbfbf
              256  328 - #007aff
              208  332 - #017aff
              356  332 - #ff3b30
              372  332 - #fd6d65
              388  332 - #fd6d65
              268  356 - #bfbfbf
                4  372 - #b7b7b8
              444  380 - #646469
              540  384 - #6f6f73
              372  556 - #bfbfbf
              484  560 - #435d70
               32  580 - #435d70
              144  580 - #000000
              184  580 - #000000
              592  592 - #435d70
";

const CHECK_4: &str = r"
              116    8 - #d9d9de
              312    8 - #d9d9de
              420   24 - #c3c3c9
              516   56 - #6e6e76
               52   84 - #dcdce3
              236   84 - #404041
               24   88 - #dcdce3
              148   92 - #dcdce3
              232  120 - #3a3a3c
               76  144 - #1c1c1e
              504  176 - #ffffff
              232  236 - #d8d8d8
              252  236 - #ffffff
              496  260 - #85858c
              220  264 - #1c1c1e
              268  264 - #1c1c1e
              424  264 - #ffffff
              536  264 - #6e6e76
              444  324 - #6e6e76
              532  324 - #99999e
                4  356 - #f4f4f6
              248  356 - #e4e4e5
              128  436 - #f4f4f6
              436  556 - #ffffff
               32  580 - #597c95
               88  580 - #597c95
              144  580 - #000000
              176  580 - #000000
              184  580 - #000000
              312  592 - #597c95
              508  592 - #597c95
              592  592 - #597c95
";
