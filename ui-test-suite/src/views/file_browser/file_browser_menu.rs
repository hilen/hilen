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
             308    8 - #d9d9de
             116   20 - #bcbcbd
             448   20 - #e8e8eb
             580   20 - #6e6e76
             192   84 - #2f8bf5
              16   88 - #dcdce3
             504  116 - #ffffff
             232  120 - #3a3a3c
             188  140 - #64a8f7
              24  144 - #8f8f96
              84  144 - #cdcdcf
             192  144 - #2f8bf5
             188  200 - #64a8f7
             444  260 - #85858c
             524  264 - #d3d3d6
             240  268 - #ffffff
             344  280 - #0a6cff
             232  296 - #aecfff
             184  300 - #0a6cff
             540  300 - #0858d0
             444  348 - #282828
             276  356 - #1c1c1e
             544  356 - #616168
             432  416 - #ffa6a1
             516  436 - #c3c3c3
             592  556 - #ffffff
              40  580 - #597c95
              68  580 - #2d3e4b
             152  580 - #000000
             224  580 - #425c6e
             208  584 - #000000
             404  592 - #597c95
";

const CHECK_2: &str = r"
             124    8 - #d9d9de
             292    8 - #d9d9de
             580   20 - #6e6e76
             424   24 - #c3c3c9
             376   56 - #c5c8ce
             516   56 - #6e6e76
             188   80 - #64a8f7
              20   84 - #dcdce3
             236   84 - #404041
             192  116 - #2f8bf5
             504  116 - #ffffff
              88  144 - #9a9a9c
             188  200 - #64a8f7
             264  204 - #1c1c1e
             192  264 - #2f8bf5
             228  296 - #ffffff
             424  296 - #ffffff
             524  296 - #d3d3d6
               4  348 - #f4f4f6
             276  384 - #1c1c1e
             420  384 - #6e6e76
             540  384 - #949499
             184  392 - #dbdbdd
             592  552 - #ffffff
             304  560 - #597c95
              32  580 - #597c95
              88  580 - #597c95
             108  580 - #3c5465
             144  580 - #000000
             168  580 - #415a6d
             184  580 - #000000
             424  592 - #597c95
";

const CHECK_3: &str = r"
             116   20 - #8d8d8e
             424   24 - #929297
             512   80 - #bfbfbf
              20   84 - #a5a5aa
             192   84 - #2368b8
             232  120 - #2c2c2d
             188  140 - #4b7eb9
              16  144 - #6b6b71
              76  144 - #151517
             592  168 - #bfbfbf
             500  192 - #0851bf
             180  204 - #0851bf
             256  204 - #537ebf
             432  256 - #f9f9f9
             228  280 - #f9f9f9
             288  280 - #f9f9f9
             352  280 - #b5b5b6
             536  292 - #56565c
             208  332 - #017aff
             256  332 - #007aff
             356  332 - #ff3b30
             372  332 - #fd6d65
             380  332 - #fadcda
             388  332 - #fd6d65
             276  384 - #151517
             540  384 - #6f6f73
             184  392 - #a4a4a6
              32  580 - #435d70
             108  580 - #2d3f4c
             144  580 - #000000
             184  580 - #000000
             424  592 - #435d70
";

const CHECK_4: &str = r"
             472    8 - #d9d9de
             104   16 - #d9d9de
             580   20 - #6e6e76
             376   56 - #c5c8ce
              36   60 - #f4f4f6
             192   84 - #2f8bf5
             236   84 - #404041
             232  116 - #1c1c1e
             504  116 - #ffffff
             188  140 - #64a8f7
              16  144 - #8f8f96
              88  144 - #9a9a9c
             192  144 - #2f8bf5
             188  200 - #64a8f7
             252  236 - #ffffff
             524  264 - #d3d3d6
             428  324 - #dfdfe0
             180  352 - #6e6e76
             276  356 - #1c1c1e
             504  356 - #d8d8da
             556  356 - #d1d1d3
             572  356 - #c7c7ca
               4  400 - #f4f4f6
             408  556 - #ffffff
              32  580 - #597c95
              88  580 - #597c95
             108  580 - #3c5465
             144  580 - #000000
             168  580 - #415a6d
             184  580 - #000000
             300  592 - #597c95
             592  592 - #597c95
";
