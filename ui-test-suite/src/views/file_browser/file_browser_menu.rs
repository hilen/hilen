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
             104   16 - #d9d9de
             448   20 - #e8e8eb
             228   56 - #1c1c1e
             516   56 - #6e6e76
              24   88 - #dcdce3
             504  116 - #ffffff
             232  120 - #3a3a3c
              88  144 - #9a9a9c
             444  260 - #85858c
             524  264 - #d3d3d6
             240  268 - #ffffff
             380  284 - #0a6cff
             184  292 - #0a6cff
             232  296 - #aecfff
             540  300 - #0858d0
             540  308 - #0853c3
             444  348 - #282828
             268  356 - #adadae
             556  356 - #d1d1d3
             572  356 - #c7c7ca
               4  380 - #f4f4f6
             432  416 - #ffa6a1
             444  416 - #ffa6a1
             524  436 - #c3c3c3
             592  556 - #ffffff
             496  564 - #597c95
              40  580 - #597c95
             104  580 - #000000
             152  580 - #000000
             188  580 - #2d3e4b
             224  580 - #425c6e
             404  592 - #597c95
";

const CHECK_2: &str = r"
             332    8 - #d9d9de
             104   16 - #d9d9de
             448   20 - #e8e8eb
             424   24 - #c3c3c9
             228   56 - #1c1c1e
             516   56 - #6e6e76
              68   84 - #818186
             236   84 - #404041
              24   88 - #dcdce3
             228  116 - #c4c4c5
              84  144 - #cdcdcf
             504  176 - #ffffff
             256  204 - #a2a2a3
             236  236 - #373738
             228  296 - #ffffff
             440  296 - #818188
             524  296 - #d3d3d6
             268  300 - #686869
             428  352 - #dfdfe0
               4  356 - #f4f4f6
             260  356 - #1c1c1e
             532  356 - #99999e
             184  380 - #ffffff
             276  384 - #1c1c1e
             544  384 - #717179
             592  552 - #ffffff
             304  560 - #597c95
              32  580 - #597c95
             108  580 - #3c5465
             144  580 - #000000
             184  580 - #000000
             424  592 - #597c95
";

const CHECK_3: &str = r"
             124   24 - #4d4d4e
             424   24 - #929297
             228   56 - #151517
             512   80 - #bfbfbf
             236   84 - #303031
              24   88 - #a5a5aa
             232  120 - #2c2c2d
              76  144 - #151517
             592  168 - #bfbfbf
             500  192 - #0851bf
             184  204 - #0851bf
             256  204 - #537ebf
             216  208 - #0851bf
             432  256 - #f9f9f9
             288  280 - #f9f9f9
             352  280 - #b5b5b6
             524  296 - #9e9ea0
             232  328 - #017aff
             256  328 - #007aff
             212  332 - #f9f9f9
             356  332 - #ff3b30
             372  332 - #fd6d65
             380  332 - #fadcda
             388  332 - #fd6d65
             504  352 - #a2a2a3
               4  368 - #b7b7b8
             444  380 - #646469
             540  384 - #6f6f73
              32  580 - #435d70
             144  580 - #000000
             184  580 - #000000
             424  592 - #435d70
";

const CHECK_4: &str = r"
             316    8 - #d9d9de
             124   24 - #676768
             424   24 - #c3c3c9
             516   56 - #6e6e76
               8   76 - #dcdce3
              68   84 - #818186
             240   84 - #1c1c1e
             232  120 - #3a3a3c
              88  144 - #9a9a9c
             504  176 - #ffffff
             252  236 - #ffffff
             444  260 - #85858c
             220  264 - #1c1c1e
             268  264 - #1c1c1e
             524  264 - #d3d3d6
             544  264 - #d8d8da
             416  324 - #c2c2c5
             444  324 - #6e6e76
             532  324 - #99999e
               4  336 - #f4f4f6
             212  356 - #e8e8e8
             268  356 - #adadae
             420  356 - #ffffff
             496  356 - #d5d5d7
             556  356 - #d1d1d3
             424  556 - #ffffff
              32  580 - #597c95
             108  580 - #3c5465
             144  580 - #000000
             184  580 - #000000
             308  592 - #597c95
             592  592 - #597c95
";
