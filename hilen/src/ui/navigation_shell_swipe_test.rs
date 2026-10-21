use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::from_main,
        refs::{Own, Weak},
    },
    gm::flat::Rect,
    ui::{
        Button, Color, Label, Setup, Tab, TabShell, TextAlignment, UIManager, View, ViewController, ViewData,
        ViewFrame, ViewSubviews, ViewTest,
        navigation_swipe_test::{held, swipe},
        view,
    },
    ui_test::{check_colors, checkpoint, inject_touches, system_input::wait_until},
};

/// The detail page over the tab, the bar under it.
const DETAIL_PAGE: &str = r"
   4    4 - #bfd1f2
 592    4 - #bfd1f2
 252   32 - #000000
 296   32 - #060708
 272   36 - #000000
 280   36 - #000000
 312   36 - #000000
 248   40 - #bfd1f2
 260   40 - #8693aa
 292   40 - #727d90
 344   40 - #6c7689
 352   40 - #8693aa
 252   44 - #000000
 264   44 - #bfd1f2
 280   44 - #bfd1f2
 292   44 - #727d90
 308   44 - #000000
 324   44 - #bfd1f2
 344   44 - #6c7689
 116   76 - #bfd1f2
   4  276 - #4d73d9
 592  276 - #4d73d9
 336  300 - #4d73d9
 164  392 - #4d73d9
   4  552 - #f9f9f9
 296  552 - #f9f9f9
 592  552 - #f9f9f9
 448  576 - #f9f9f9
 140  588 - #1f8afe
 152  592 - #f9f9f9
 436  592 - #f9f9f9
 452  592 - #f9f9f9
";

/// The detail page 190 points right, the tab page behind, the bar in place.
const DETAIL_FOLLOWS: &str = r"
   4    4 - #d94d4d
 328    4 - #bfd1f2
 440   32 - #a2b1cd
 488   32 - #98a6c1
 452   40 - #8693aa
 488   40 - #98a6c1
 504   40 - #bfd1f2
 528   40 - #bfd1f2
 436   44 - #bfd1f2
 476   44 - #4d5462
 480   44 - #2c3038
 520   44 - #272b31
 220   52 - #bfd1f2
 396  244 - #4d73d9
 156  264 - #c3c3c3
 112  272 - #000000
 132  272 - #9d9d9d
 164  272 - #000000
 156  276 - #c3c3c3
 180  276 - #000000
 132  280 - #9d9d9d
 156  284 - #c3c3c3
 164  284 - #010101
 188  284 - #010101
 592  332 - #4d73d9
   4  440 - #d94d4d
 296  552 - #f9f9f9
 592  552 - #f9f9f9
 448  576 - #f9f9f9
 140  588 - #1f8afe
 436  592 - #f9f9f9
 452  592 - #f9f9f9
";

/// The tab page on top again, the first tab marked.
const TAB_PAGE: &str = r"
   4    4 - #d94d4d
 592    4 - #d94d4d
 276  264 - #4a4a4a
 312  264 - #707070
 372  264 - #b3b3b3
 224  272 - #161616
 244  272 - #111111
 276  272 - #4a4a4a
 236  276 - #010101
 244  276 - #111111
 252  276 - #6c6c6c
 328  276 - #616161
 340  276 - #434343
 360  276 - #cbcbcb
 372  276 - #b3b3b3
 276  280 - #4a4a4a
 308  280 - #f2f2f2
 320  280 - #000000
 340  280 - #434343
 360  280 - #cbcbcb
 252  284 - #000000
 288  284 - #010101
 372  284 - #b3b3b3
 224  288 - #161616
 592  548 - #d94d4d
   4  552 - #f9f9f9
 256  552 - #f9f9f9
 448  576 - #f9f9f9
 140  588 - #1f8afe
 368  592 - #f9f9f9
 436  592 - #f9f9f9
 452  592 - #f9f9f9
";

/// A tab page with a push button.
#[view]
struct ShellTabPage {
    #[init]
    push: Button,
}

impl Setup for ShellTabPage {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.85, 0.3, 0.3));
        self.push.set_text("push detail").set_color(Color::rgb(0.95, 0.95, 0.95));
        self.push.place().center().size(220, 64);
        self.push.on_tap(move || self.navigation().push(ShellDetailPage::new()));
    }
}

#[view]
struct ShellDetailPage {
    #[init]
    title: Label,
}

impl Setup for ShellDetailPage {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.3, 0.45, 0.85));
        self.title
            .set_text("detail page")
            .set_text_size(24)
            .set_alignment(TextAlignment::Center);
        self.title.set_color(Color::rgb(0.75, 0.82, 0.95));
        self.title.place().lrt(0).h(80);
    }
}

/// A swipe from the left edge leaves a page pushed inside a tab. Only the
/// page moves, the bar stays where it is and keeps its mark.
#[view]
struct NavigationShellSwipe {
    #[init]
    shell: TabShell,
}

impl Setup for NavigationShellSwipe {
    fn setup(self: Weak<Self>) {
        self.shell.place().back();
        self.shell.set_tabs(
            vec![
                Tab::new("Lists", "tab_lists.svg"),
                Tab::new("Friends", "tab_friends.svg"),
            ],
            |_| -> Own<dyn View> { ShellTabPage::new() },
        );
        self.shell.select(0);
    }
}

fn settled(view: Weak<NavigationShellSwipe>, depth: usize) -> bool {
    from_main(move || {
        let nav = view.shell.navigation();
        nav.subviews().len() == depth
            && !UIManager::touch_disabled()
            && nav.subviews().last().is_some_and(|top| top.x().abs() < f32::EPSILON)
    })
}

fn bar_frame(view: Weak<NavigationShellSwipe>) -> Rect {
    from_main(move || *view.shell.bar().absolute_frame())
}

impl ViewTest for NavigationShellSwipe {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_until("the tab is up", move || settled(view, 1))?;
        let bar = bar_frame(view);

        inject_touches("300 270 b\n300 270 e");
        wait_until("the detail page slid in", move || settled(view, 2))?;
        ensure!(bar_frame(view) == bar, "the push moved the bar");
        check_colors(DETAIL_PAGE)?;

        inject_touches(held(300, 10, 200, 5));
        ensure!(bar_frame(view) == bar, "the swipe moved the bar");
        check_colors(DETAIL_FOLLOWS)?;
        checkpoint("the detail page follows the finger, the bar stays")?;

        inject_touches("200 300 e");
        wait_until("the detail page slid back", move || settled(view, 2))?;

        inject_touches(swipe(300, 10, 400, 5));
        wait_until("the tab page is on top", move || settled(view, 1))?;
        ensure!(bar_frame(view) == bar, "the swipe moved the bar");
        ensure!(
            from_main(move || view.shell.selected()) == 0,
            "the swipe changed the tab"
        );
        check_colors(TAB_PAGE)?;
        checkpoint("the swipe went back to the tab page, the bar kept its place and mark")?;
        Ok(())
    }
}
