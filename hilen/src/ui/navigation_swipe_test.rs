use std::{
    fmt::Write,
    sync::atomic::{AtomicU32, Ordering},
};

use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::from_main,
        refs::{Own, Weak},
    },
    ui::{
        Button, Color, Label, NavigationView, Setup, TextAlignment, UIManager, View, ViewController,
        ViewData, ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint, inject_touches, system_input::wait_until},
};

static TAPS: AtomicU32 = AtomicU32::new(0);
static THIRD_BUILT: AtomicU32 = AtomicU32::new(0);

/// The second page with its edge button and back button.
const SECOND_PAGE: &str = r"
   4    4 - #bfd1f2
 592    4 - #bfd1f2
 264   40 - #bfd1f2
 324   40 - #bfd1f2
 340   40 - #616a7a
 292   44 - #434954
 352   48 - #000000
 448  124 - #5f5f5f
 496  124 - #acacac
 520  128 - #f2f2f2
 448  132 - #5f5f5f
 452  132 - #333333
 476  132 - #333333
 576  156 - #333333
  44  288 - #c2ae61
 140  296 - #c9b565
  76  300 - #e6cf73
 180  300 - #000000
  24  304 - #ffe580
 172  304 - #827541
 172  308 - #827541
  84  348 - #ffe580
 592  424 - #4d73d9
 192  496 - #f2f2f2
 312  520 - #434343
 244  528 - #010101
 344  528 - #ababab
 272  532 - #f2f2f2
 296  536 - #797979
 332  536 - #626262
 344  536 - #ababab
   4  592 - #4d73d9
";

/// The third page 150 points right, the second behind it.
const THIRD_FOLLOWS: &str = r"
   4    4 - #bfd1f2
 408   32 - #a9cfb0
 436   36 - #000000
 116   40 - #8693aa
 140   40 - #bfd1f2
 496   40 - #86a58c
 128   44 - #bfd1f2
 408   44 - #a9cfb0
 436   44 - #bfebc7
 452   44 - #556858
 484   44 - #bfebc7
 452   48 - #556858
  44   76 - #bfd1f2
 276   76 - #bfebc7
 592   76 - #bfebc7
   4  148 - #4d73d9
  64  252 - #ffe580
  24  300 - #ffe580
  36  300 - #000000
  20  304 - #ffe580
  64  348 - #ffe580
 404  352 - #597c95
 148  380 - #4d73d9
 148  516 - #313131
  96  524 - #161616
 124  524 - #6c6c6c
 104  532 - #f2f2f2
 136  532 - #f2f2f2
 116  536 - #010101
  96  540 - #161616
  56  556 - #f2f2f2
 592  592 - #597c95
";

/// The third page back in its place.
const THIRD_BACK: &str = r"
 120    4 - #bfebc7
 480    4 - #bfebc7
 256   32 - #000000
 292   32 - #000000
 248   36 - #000000
 320   36 - #000000
 332   36 - #000000
 252   40 - #bfebc7
 288   40 - #bfebc7
 308   40 - #bfebc7
 324   40 - #000000
 336   40 - #b9e4c1
 348   40 - #86a58c
 256   44 - #010101
 264   44 - #010101
 276   44 - #010101
 288   44 - #bfebc7
 292   44 - #000000
 308   44 - #bfebc7
 316   44 - #000000
 332   44 - #bfebc7
   4   76 - #bfebc7
 172   76 - #bfebc7
 428   76 - #bfebc7
 592   76 - #bfebc7
 536  324 - #597c95
  40  332 - #597c95
 292  396 - #597c95
 188  552 - #597c95
 592  576 - #597c95
   4  592 - #597c95
 376  592 - #597c95
";

/// The second page on top after the swipe.
const SECOND_ON_TOP: &str = r"
   4    4 - #bfd1f2
 592    4 - #bfd1f2
 264   40 - #bfd1f2
 324   40 - #bfd1f2
 340   40 - #616a7a
 292   44 - #434954
 352   48 - #000000
 448  124 - #5f5f5f
 496  124 - #acacac
 520  128 - #f2f2f2
 448  132 - #5f5f5f
 452  132 - #333333
 476  132 - #333333
 576  156 - #333333
  44  288 - #c2ae61
 140  296 - #c9b565
  76  300 - #e6cf73
 180  300 - #000000
  24  304 - #ffe580
 172  304 - #827541
 172  308 - #827541
  84  348 - #ffe580
 592  424 - #4d73d9
 192  496 - #f2f2f2
 312  520 - #434343
 244  528 - #010101
 344  528 - #ababab
 272  532 - #f2f2f2
 296  536 - #797979
 332  536 - #626262
 344  536 - #ababab
   4  592 - #4d73d9
";

/// The first page on top after the fling.
const FIRST_AFTER_FLING: &str = r"
   4    4 - #f2bfbf
 252   40 - #2c2222
 264   40 - #b69090
 288   40 - #c39a9a
 332   40 - #f2bfbf
 348   40 - #000000
 252   44 - #2c2222
 264   44 - #b69090
 276   44 - #f2bfbf
 316   44 - #f2bfbf
 300   48 - #010101
 120   76 - #f2bfbf
 592   76 - #f2bfbf
 192  268 - #f2f2f2
 408  268 - #f2f2f2
 264  288 - #000000
 388  292 - #525252
 216  300 - #f2f2f2
 296  300 - #f2f2f2
 312  300 - #dadada
 240  304 - #000000
 276  304 - #494949
 332  304 - #f2f2f2
 360  304 - #acacac
 276  308 - #494949
 308  308 - #010101
 360  308 - #acacac
 388  308 - #525252
 408  328 - #f2f2f2
   4  592 - #d94d4d
 292  592 - #d94d4d
 592  592 - #d94d4d
";

/// The first page on top after the back button.
const FIRST_AFTER_BACK: &str = r"
   4    4 - #f2bfbf
 252   40 - #2c2222
 264   40 - #b69090
 288   40 - #c39a9a
 332   40 - #f2bfbf
 348   40 - #000000
 252   44 - #2c2222
 264   44 - #b69090
 276   44 - #f2bfbf
 316   44 - #f2bfbf
 300   48 - #010101
 120   76 - #f2bfbf
 592   76 - #f2bfbf
 192  268 - #f2f2f2
 408  268 - #f2f2f2
 264  288 - #000000
 388  292 - #525252
 216  300 - #f2f2f2
 296  300 - #f2f2f2
 312  300 - #dadada
 240  304 - #000000
 276  304 - #494949
 332  304 - #f2f2f2
 360  304 - #acacac
 276  308 - #494949
 308  308 - #010101
 360  308 - #acacac
 388  308 - #525252
 408  328 - #f2f2f2
   4  592 - #d94d4d
 292  592 - #d94d4d
 592  592 - #d94d4d
";

fn count(counter: &AtomicU32) -> u32 {
    counter.load(Ordering::Relaxed)
}

/// A finger at `y` from `from` to `to` in steps of `step` points, then a
/// release where it stopped.
pub(crate) fn swipe(y: u32, from: u32, to: u32, step: u32) -> String {
    let mut lines = format!("{from} {y} b\n");
    let mut x = from;
    while x < to {
        x = (x + step).min(to);
        writeln!(lines, "{x} {y} m").unwrap();
    }
    writeln!(lines, "{x} {y} e").unwrap();
    lines
}

/// The same swipe with no release, the finger stays down.
pub(crate) fn held(y: u32, from: u32, to: u32, step: u32) -> String {
    let mut lines = swipe(y, from, to, step);
    lines.truncate(lines.trim_end().rfind('\n').unwrap_or(0) + 1);
    lines
}

/// `depth` pages on the stack, the top one in its place, no slide running.
fn settled(nav: Weak<NavigationView>, depth: usize) -> bool {
    from_main(move || {
        nav.subviews().len() == depth
            && !UIManager::touch_disabled()
            && nav.subviews().last().is_some_and(|top| top.x().abs() < f32::EPSILON)
    })
}

fn top_x(nav: Weak<NavigationView>) -> f32 {
    from_main(move || nav.subviews().last().map_or(0.0, |top| top.x()))
}

fn titled(label: Weak<Label>, text: &str, color: Color) {
    label.set_text(text).set_text_size(24).set_color(color);
    label.place().lrt(0).h(80);
}

/// The first page of a stack. It has no way back, its push button puts the
/// second page on top.
#[view]
struct NavigationSwipe {
    #[init]
    title: Label,
    push:  Button,
}

impl Setup for NavigationSwipe {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.85, 0.3, 0.3));
        titled(self.title, "first page", Color::rgb(0.95, 0.75, 0.75));

        self.push.set_text("push second").set_color(Color::rgb(0.95, 0.95, 0.95));
        self.push.place().center().size(220, 64);
        self.push.on_tap(move || self.navigation().push(SecondPage::new()));
    }
}

/// A button at its left edge, a back button the page styles, and a push
/// button for the third page.
#[view]
struct SecondPage {
    #[init]
    title: Label,
    edge:  Button,
    push:  Button,
}

impl Setup for SecondPage {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.3, 0.45, 0.85));
        titled(self.title, "second page", Color::rgb(0.75, 0.82, 0.95));

        self.edge.set_text("edge button").set_color(Color::rgb(1.0, 0.9, 0.5));
        self.edge.place().l(0).center_y().size(200, 100);
        self.edge.on_tap(|| {
            TAPS.fetch_add(1, Ordering::Relaxed);
        });

        self.push.set_text("push third").set_color(Color::rgb(0.95, 0.95, 0.95));
        self.push.place().center_x().b(40).size(220, 64);
        self.push.on_tap(move || self.navigation().push(ThirdPage::new()));

        let back = self.back_button();
        back.set_color(Color::rgb(0.2, 0.2, 0.2)).set_corner_radius(12);
        back.place().l(420).t(100).size(160, 60);
        let label = back.add_view::<Label>();
        label
            .set_text("back button")
            .set_text_size(20)
            .set_text_color(Color::rgb(0.95, 0.95, 0.95))
            .set_alignment(TextAlignment::Center)
            .set_color(Color::rgb(0.2, 0.2, 0.2));
        label.place().back();
    }
}

/// No background of its own, like most pages of an app, it shows the clear
/// color of the window. The second page must not show through it.
#[view]
struct ThirdPage {
    #[init]
    title: Label,
}

impl Setup for ThirdPage {
    fn setup(self: Weak<Self>) {
        THIRD_BUILT.fetch_add(1, Ordering::Relaxed);
        titled(self.title, "third page", Color::rgb(0.75, 0.92, 0.78));
    }
}

impl ViewTest for NavigationSwipe {
    fn make_root(view: Own<Self>) -> Own<dyn View> {
        NavigationView::with_view(view)
    }

    // A finger gesture, it swipes only where drag scrolling is on, the
    // touch platform default.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
        for counter in [&TAPS, &THIRD_BUILT] {
            counter.store(0, Ordering::Relaxed);
        }
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let nav = from_main(move || view.navigation());

        inject_touches(swipe(450, 10, 400, 5));
        ensure!(settled(nav, 1), "a swipe on the first page changed the stack");

        inject_touches("300 300 b\n300 300 e");
        wait_until("the second page slid in", move || settled(nav, 2))?;

        inject_touches(swipe(450, 300, 500, 20));
        ensure!(settled(nav, 2), "a drag from the middle started a swipe");

        inject_touches("10 300 b\n10 300 e");
        ensure!(count(&TAPS) == 1, "a tap at the edge did not reach the button");

        inject_touches("10 300 b\n12 330 m\n20 420 m\n20 420 e");
        ensure!(settled(nav, 2), "a drag down from the edge started a swipe");
        check_colors(SECOND_PAGE)?;

        inject_touches("300 528 b\n300 528 e");
        wait_until("the third page slid in", move || settled(nav, 3))?;
        ensure!(
            count(&THIRD_BUILT) == 1,
            "the third page was built {} times",
            count(&THIRD_BUILT)
        );

        held_swipe(nav)?;
        full_swipe(nav)?;
        fling(view, nav)?;
        back_button(view, nav)
    }
}

fn held_swipe(nav: Weak<NavigationView>) -> Result<()> {
    // A short swipe, let go before half.
    inject_touches(held(300, 10, 160, 5));
    ensure!(
        (top_x(nav) - 150.0).abs() < 1.0,
        "the top page does not follow the finger: {}",
        top_x(nav)
    );
    check_colors(THIRD_FOLLOWS)?;
    checkpoint("the third page follows the finger, the second shows behind")?;

    inject_touches("160 300 e");
    wait_until("the third page slid back", move || settled(nav, 3))?;
    ensure!(count(&TAPS) == 1, "the release of a swipe tapped a button");
    ensure!(
        from_main(move || nav.subviews()[1].is_hidden()),
        "the second page still shows under the third"
    );
    check_colors(THIRD_BACK)?;
    checkpoint("let go before half, the third page slid back")?;
    Ok(())
}

fn full_swipe(nav: Weak<NavigationView>) -> Result<()> {
    inject_touches(swipe(450, 10, 400, 5));
    wait_until("the second page is on top", move || settled(nav, 2))?;
    ensure!(
        from_main(move || !nav.subviews()[1].is_hidden()),
        "the second page is hidden after the swipe"
    );
    ensure!(count(&THIRD_BUILT) == 1, "the swipe built the third page again");
    check_colors(SECOND_ON_TOP)?;
    checkpoint("let go past half, the second page is on top")?;
    Ok(())
}

fn fling(view: Weak<NavigationSwipe>, nav: Weak<NavigationView>) -> Result<()> {
    // A fast last step well before half.
    let mut fling = held(450, 10, 60, 5);
    fling.push_str("90 450 m\n90 450 e\n");
    inject_touches(fling);
    wait_until("the first page is on top", move || settled(nav, 1))?;
    ensure!(
        from_main(move || !view.is_hidden()),
        "the first page is hidden after the fling"
    );
    check_colors(FIRST_AFTER_FLING)?;
    checkpoint("a fling went back to the first page")?;
    Ok(())
}

fn back_button(view: Weak<NavigationSwipe>, nav: Weak<NavigationView>) -> Result<()> {
    inject_touches("300 300 b\n300 300 e");
    wait_until("the second page slid in again", move || settled(nav, 2))?;
    checkpoint("the second page is on top again")?;

    inject_touches("500 130 b\n500 130 e");
    wait_until("the back button popped the second page", move || settled(nav, 1))?;
    ensure!(
        from_main(move || !view.is_hidden()),
        "the first page is hidden after the back button"
    );
    check_colors(FIRST_AFTER_BACK)?;
    checkpoint("the back button went back to the first page")?;
    Ok(())
}
