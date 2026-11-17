//! `Diff panel menu`: a long press on a row of the diff panel opens a menu
//! with the discard of that row, the way a phone discards, since it has
//! no pointer to show the discard buttons.

use anyhow::{Result, ensure};
use filediff::DiffTarget;

use super::{
    look_test::{Calls, MemoryHost},
    menu::Discard,
};
use crate::{
    self as hilen,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        ContextMenu, DiffPanel, DiffShown, Label, NamedKey, Point, Setup, TextAlignment, ViewData, ViewFrame,
        ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_long_press, inject_named_key, inject_touches, set_record_probe_count},
};

/// The panel stands this far from the top left of the window.
const PANEL_AT: f32 = 10.0;
/// The middle of the rows in the points of the window, in 1 column: the
/// hunk title, the line that stayed, the removed line and the first
/// added line.
const HUNK_Y: f32 = 52.0;
const KEPT_Y: f32 = 71.0;
const REMOVED_Y: f32 = 89.0;
const ADDED_Y: f32 = 107.0;
/// Inside the code of the left half and of the right half of a row.
const LEFT_X: f32 = 200.0;
const RIGHT_X: f32 = 450.0;

#[view]
struct DiffPanelMenu {
    calls: Calls,

    #[init]
    panel:  DiffPanel,
    status: Label,
}

impl Setup for DiffPanelMenu {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.panel.place().t(PANEL_AT).lr(PANEL_AT).h(400);
        self.status.set_text_size(16).set_multiline(true);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().t(420).lr(10).b(10);

        self.panel.set_host(MemoryHost {
            calls: self.calls.clone(),
        });
        self.panel.show(
            Some(DiffShown {
                file:     "src/main.rs".to_string(),
                old_path: None,
                target:   DiffTarget::WorkingTree,
            }),
            1,
            true,
        );
    }
}

/// What the panel would discard at a point of the window.
fn discard_at(view: Weak<DiffPanelMenu>, x: f32, y: f32) -> Option<Discard> {
    from_main(move || view.panel.discard_at(Point::new(x - PANEL_AT, y - PANEL_AT)))
}

/// The titles of the open menu, empty when no menu is open.
fn menu_titles() -> Vec<String> {
    from_main(|| {
        let menu = ContextMenu::open();
        if menu.is_null() {
            return vec![];
        }
        menu.items().iter().map(|item| item.title().to_string()).collect()
    })
}

fn status(view: Weak<DiffPanelMenu>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
    wait_for_next_frame();
}

impl ViewTest for DiffPanelMenu {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        wait_for_next_frame();

        // The rows and halves map to what their discard buttons do.
        ensure!(discard_at(view, LEFT_X, HUNK_Y) == Some(Discard::Hunk(0)));
        ensure!(discard_at(view, LEFT_X, KEPT_Y).is_none());
        ensure!(discard_at(view, LEFT_X, REMOVED_Y) == Some(Discard::Line { hunk: 0, line: 1 }));
        ensure!(discard_at(view, LEFT_X, ADDED_Y) == Some(Discard::Line { hunk: 0, line: 2 }));

        status(view, "a long press on the removed line: a menu with Discard line");
        inject_long_press(LEFT_X, REMOVED_Y);
        let titles = menu_titles();
        ensure!(titles == ["Discard line"], "the menu shows {titles:?}");
        check_colors(UNIFIED_MENU)?;

        // The item asks first, like the button, then discards the line.
        let item = from_main(|| {
            let menu = ContextMenu::open();
            let frame = menu.frame();
            Point::new(frame.origin.x + 50.0, frame.origin.y + 18.0)
        });
        inject_touches(format!("{} {} b\n{} {} e", item.x, item.y, item.x, item.y));
        wait_for_next_frame();
        let calls = from_main(move || view.calls.lock().clone());
        ensure!(menu_titles().is_empty(), "the menu stayed open after a pick");
        ensure!(
            calls
                .iter()
                .any(|call| call.starts_with("confirm Discard this line") && call.ends_with("Discard line")),
            "no question before the discard: {calls:?}"
        );
        ensure!(
            calls.iter().any(|call| call == "report Line discarded Ok(())"),
            "the line was not discarded: {calls:?}"
        );

        status(view, "a long press on the hunk title: a menu with Discard hunk");
        inject_long_press(LEFT_X, HUNK_Y);
        let titles = menu_titles();
        ensure!(titles == ["Discard hunk"], "the menu shows {titles:?}");
        inject_named_key(NamedKey::Escape);

        // A line that did not change has nothing to discard.
        inject_long_press(LEFT_X, KEPT_Y);
        let titles = menu_titles();
        ensure!(titles.is_empty(), "a kept line opened {titles:?}");

        from_main(move || view.panel.set_split(true));
        wait_for_next_frame();
        // In 2 columns the added line stands right of the removed one.
        ensure!(discard_at(view, LEFT_X, REMOVED_Y) == Some(Discard::Line { hunk: 0, line: 1 }));
        ensure!(discard_at(view, RIGHT_X, REMOVED_Y) == Some(Discard::Line { hunk: 0, line: 2 }));

        status(view, "split: a long press on the added line on the right");
        inject_long_press(RIGHT_X, REMOVED_Y);
        let titles = menu_titles();
        ensure!(titles == ["Discard line"], "the menu shows {titles:?}");
        check_colors(SPLIT_MENU)?;
        inject_named_key(NamedKey::Escape);

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const UNIFIED_MENU: &str = r"
              60   28 - #9f9f9f
              96   28 - #b4b4b4
             548   28 - #ededed
             156   52 - #a6a6a6
             188   52 - #b6b6b6
             128   72 - #ccbaee
             148   72 - #2b7dde
             160   72 - #2378dd
             476   80 - #f5e4e4
              12   88 - #f5e4e4
             200   88 - #8b63cd
             336   88 - #d2c3c3
             360  100 - #abb8b3
             232  104 - #ffffff
             280  104 - #ff3b30
             172  108 - #9673e1
             264  108 - #ff3b30
             272  108 - #ffffff
             280  108 - #ff3b30
             284  108 - #ffaaa5
             292  108 - #ffb4b0
             200  116 - #d2d2d7
             364  124 - #cad9d4
             588  124 - #dff0ea
             228  128 - #abb8b3
             324  128 - #abb8b3
             156  144 - #b6d0ee
             184  144 - #a3a3a3
             216  144 - #858585
              32  160 - #898989
             592  228 - #ffffff
             308  312 - #f6f6f6
             592  416 - #ffffff
             224  500 - #888888
             232  500 - #525252
              52  504 - #cdcdcd
              84  504 - #000000
             404  504 - #ffffff
              32  508 - #000000
             132  508 - #191919
             180  508 - #000000
             236  508 - #1f1f1f
             284  508 - #919191
             288  508 - #323232
             300  508 - #4d4d4d
             348  508 - #4c4c4c
             392  508 - #000000
             428  508 - #c0c0c0
";

/// Recorded with `--record-colors`.
const SPLIT_MENU: &str = r"
             548   24 - #ededed
              64   28 - #787878
             156   52 - #a6a6a6
             188   52 - #b6b6b6
              88   72 - #ccbaee
             108   72 - #2b7dde
             120   72 - #2378dd
             140   72 - #f6f6f6
             384   72 - #9b74e4
             400   72 - #adcaed
             404   72 - #b7d0ef
             416   72 - #91baea
             420   72 - #3d88e0
             228   80 - #f5e4e4
             176   88 - #b85d18
             408   88 - #9068e1
             452   88 - #7f5ec1
             572   88 - #bfcec8
             320   92 - #b2bcb8
              12   96 - #f5e4e4
             280   96 - #f5e4e4
             520  104 - #ff3b30
             408  108 - #9068e1
             472  108 - #ffffff
             504  108 - #ff3b30
             520  108 - #ff3b30
             524  108 - #ffaaa5
             532  108 - #ffb4b0
             560  124 - #d1d1d6
             444  128 - #c2c2c2
             592  128 - #c3c3c3
               4  280 - #ffffff
             212  292 - #f6f6f6
             592  404 - #ffffff
             184  500 - #747474
             228  500 - #1e1e1e
             308  500 - #cdcdcd
              36  504 - #000000
              68  504 - #ffffff
             336  504 - #000000
              48  508 - #525252
              92  508 - #a7a7a7
              96  508 - #191919
             188  508 - #4c4c4c
             256  508 - #9e9e9e
             260  508 - #4d4d4d
             296  508 - #000000
             348  508 - #676767
";
