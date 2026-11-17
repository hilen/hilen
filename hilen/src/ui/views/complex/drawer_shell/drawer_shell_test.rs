use std::{thread::sleep, time::Duration};

use anyhow::{Result, bail, ensure};

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::Color,
    ui::{Button, DrawerShell, Label, NamedKey, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, checkpoint, inject_named_key, inject_touches},
};

const SIDE: Color = Color::hex("#2060C0");
const PAGE: Color = Color::hex("#F4F1E8");
const BUTTON: Color = Color::hex("#E0A030");

const SIDE_WIDTH: f32 = 200.0;
const GIVE_UP_AFTER: Duration = Duration::from_secs(5);

/// A `DrawerShell` with a sidebar button and a page button. Wide, both take
/// taps side by side. Narrow, the sidebar hides behind the menu button of
/// the bar and slides in over the page, the page under it takes no tap, and
/// a tap on the scrim, Escape and a change to wide close it.
#[view]
struct DrawerShellTest {
    side_taps: u32,
    page_taps: u32,
    opened:    Vec<bool>,

    #[init]
    shell: DrawerShell,
}

impl Setup for DrawerShellTest {
    fn setup(mut self: Weak<Self>) {
        self.shell.place().back();
        self.shell.set_side_width(SIDE_WIDTH);
        self.shell.set_narrow_below(400.0);
        self.shell.set_title("page");
        self.shell.opened.val(move |open| self.opened.push(open));

        let side = self.shell.side();
        side.set_color(SIDE);
        let side_button = side.add_view::<Button>();
        side_button.set_text("side").set_color(BUTTON);
        side_button.place().l(20).t(100).size(160, 50);
        side_button.on_tap(move || {
            self.side_taps += 1;
            self.show_taps();
        });

        let page = self.shell.content();
        page.set_color(PAGE);
        let page_button = page.add_view::<Button>();
        page_button.set_text("page").set_color(BUTTON);
        page_button.place().l(100).t(200).size(160, 50);
        page_button.on_tap(move || {
            self.page_taps += 1;
            self.show_taps();
        });
        let status = page.add_view::<Label>();
        status.set_text_size(16);
        status.place().lrb(0).h(40);
        self.show_taps();
    }
}

impl DrawerShellTest {
    fn show_taps(self: Weak<Self>) {
        let text = format!("page taps {}, sidebar taps {}", self.page_taps, self.side_taps);
        if let Some(status) = self.shell.content().subviews().last()
            && let Some(label) = status.downcast_view::<Label>()
        {
            label.set_text(text);
        }
    }

    fn taps(self: Weak<Self>) -> (u32, u32) {
        from_main(move || (self.page_taps, self.side_taps))
    }
}

/// Waits until the drawer stands still: fully in when open, hidden when
/// closed.
fn wait_still(view: Weak<DrawerShellTest>, open: bool) -> Result<()> {
    let start = web_time::Instant::now();
    loop {
        let still = from_main(move || {
            let shell = view.shell;
            let panel = shell.layer.panel;
            if open {
                shell.is_open() && panel.x().abs() < 0.5 && !shell.layer.is_hidden()
            } else {
                !shell.is_open() && shell.layer.is_hidden()
            }
        });
        if still {
            return Ok(());
        }
        if start.elapsed() > GIVE_UP_AFTER {
            bail!(
                "the drawer did not come to rest {}",
                if open { "open" } else { "closed" }
            );
        }
        sleep(Duration::from_millis(30));
    }
}

fn tap(x: u32, y: u32) {
    inject_touches(format!("{x} {y} b\n{x} {y} e"));
}

impl ViewTest for DrawerShellTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let wide_page = (300 + 80, 200 + 25);
        let bar: u32 = 44;
        let narrow_page = (100 + 80, 200 + 25 + bar);
        let side_button = (100, 125);
        let menu = (24, 22);

        ensure!(
            !from_main(move || view.shell.is_narrow()),
            "a 600 point shell is not narrow below 400"
        );
        tap(side_button.0, side_button.1);
        tap(wide_page.0, wide_page.1);
        ensure!(
            view.taps() == (1, 1),
            "wide, a tap of each button counts, got {:?}",
            view.taps()
        );
        check_colors(WIDE)?;
        checkpoint("wide: the sidebar stands left of the page, each button took a tap")?;

        from_main(move || view.shell.set_narrow_below(700.0));
        ensure!(
            from_main(move || view.shell.is_narrow()),
            "a 600 point shell is narrow below 700"
        );
        ensure!(
            from_main(move || view.shell.layer.is_hidden() && !view.shell.bar.is_hidden()),
            "narrow, the sidebar hides and the bar shows"
        );
        tap(narrow_page.0, narrow_page.1);
        ensure!(
            view.taps() == (2, 1),
            "narrow, the page button moved under the bar, got {:?}",
            view.taps()
        );
        check_colors(NARROW)?;
        checkpoint("narrow: the sidebar is hidden, the bar holds the menu button")?;

        tap(menu.0, menu.1);
        wait_still(view, true)?;
        tap(narrow_page.0, narrow_page.1);
        ensure!(
            view.taps() == (2, 1) && from_main(move || view.shell.is_open()),
            "the page under the open drawer took a tap, got {:?}",
            view.taps()
        );
        tap(side_button.0, side_button.1);
        ensure!(
            view.taps() == (2, 2),
            "the sidebar in the drawer takes a tap, got {:?}",
            view.taps()
        );
        check_colors(OPEN)?;
        checkpoint("open: the sidebar slid in, the scrim dims the page right of it")?;

        tap(400, 300);
        wait_still(view, false)?;
        tap(narrow_page.0, narrow_page.1);
        ensure!(
            view.taps() == (3, 2),
            "after a tap on the scrim the page takes taps, got {:?}",
            view.taps()
        );
        checkpoint("a tap on the scrim closed the drawer, the page took a tap")?;

        tap(menu.0, menu.1);
        wait_still(view, true)?;
        inject_named_key(NamedKey::Escape);
        wait_still(view, false)?;
        checkpoint("Escape closed the drawer")?;

        tap(menu.0, menu.1);
        wait_still(view, true)?;
        from_main(move || view.shell.set_narrow_below(400.0));
        ensure!(
            from_main(move || !view.shell.is_open() && !view.shell.is_narrow()),
            "a shell that turns wide has no open drawer"
        );
        tap(wide_page.0, wide_page.1);
        tap(side_button.0, side_button.1);
        ensure!(
            view.taps() == (4, 3),
            "wide again, both buttons take taps, got {:?}",
            view.taps()
        );
        checkpoint("wide again: the drawer gave the touches back")?;

        let opened = from_main(move || view.opened.clone());
        ensure!(
            opened == [true, false, true, false, true, false],
            "opened fired {opened:?}"
        );
        Ok(())
    }
}

/// Wide: the sidebar left of the page.
const WIDE: &str = r"
               4    4 - #2060c0
             368    4 - #f4f1e8
             592    4 - #f4f1e8
             176  100 - #e0a030
             108  116 - #000000
              80  124 - #e0a030
             100  124 - #e0a030
             120  124 - #e0a030
             108  132 - #000000
              20  148 - #e0a030
             456  200 - #e0a030
             300  204 - #e0a030
             348  220 - #010100
             388  224 - #e0a030
             360  228 - #010100
             376  232 - #000000
             408  232 - #e0a030
             348  240 - #010100
             592  332 - #f4f1e8
             196  344 - #2060c0
               4  404 - #2060c0
             428  408 - #f4f1e8
             592  524 - #f4f1e8
             220  580 - #f4f1e8
             244  580 - #000000
             272  580 - #f4f1e8
             316  580 - #000000
             340  580 - #f4f1e8
             380  580 - #f4f1e8
             384  580 - #000000
             224  588 - #f4f1e8
              32  592 - #2060c0
";

/// Narrow: the bar over the page, the sidebar hidden.
const NARROW: &str = r"
             340    4 - #f9f9f9
             592    4 - #f9f9f9
              88   20 - #1c1c1e
              64   24 - #1c1c1e
              88   24 - #1c1c1e
             256  244 - #e0a030
             100  248 - #e0a030
             148  264 - #010100
             188  264 - #000000
             172  268 - #e0a030
             212  268 - #010100
             152  272 - #e0a030
             192  272 - #e0a030
             148  276 - #000000
             188  276 - #e0a030
             208  276 - #e0a030
             148  280 - #010100
             148  284 - #010100
             104  292 - #e0a030
             256  292 - #e0a030
             552  300 - #f4f1e8
             392  480 - #f4f1e8
              20  580 - #f4f1e8
              44  580 - #000000
              72  580 - #f4f1e8
             116  580 - #000000
             140  580 - #f4f1e8
             180  580 - #f4f1e8
             184  580 - #000000
              96  584 - #4d4c49
              24  588 - #f4f1e8
             592  592 - #f4f1e8
";

/// Open: the sidebar over the dimmed page.
const OPEN: &str = r"
             360    4 - #a2a2a2
             592    4 - #a2a2a2
              36  100 - #e0a030
             168  100 - #e0a030
             108  116 - #000000
             124  120 - #000000
              80  124 - #e0a030
             100  124 - #e0a030
             108  124 - #000000
             120  124 - #e0a030
             104  128 - #e0a030
             108  128 - #000000
             108  132 - #000000
              20  148 - #e0a030
             176  148 - #e0a030
             228  244 - #92681f
             256  244 - #92681f
             516  256 - #9f9d97
             204  264 - #000000
             208  268 - #92681f
             212  268 - #000000
             240  268 - #92681f
             200  272 - #000000
             208  276 - #92681f
              56  288 - #2060c0
             256  292 - #92681f
               4  424 - #2060c0
             164  428 - #2060c0
             592  504 - #9f9d97
             200  580 - #9f9d97
             204  584 - #323130
              28  592 - #2060c0
";
