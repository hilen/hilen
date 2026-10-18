use std::{
    fmt::Write,
    sync::atomic::{AtomicU32, Ordering},
};

use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    ui::{
        Button, Color, Label, Setup, SwipeBack, UIManager, ViewData, ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint, inject_touches, system_input::wait_until},
};

static TAPS: AtomicU32 = AtomicU32::new(0);
static SECOND_BUILT: AtomicU32 = AtomicU32::new(0);
static THIRD_BUILT: AtomicU32 = AtomicU32::new(0);
static DONE: AtomicU32 = AtomicU32::new(0);

fn count(counter: &AtomicU32) -> u32 {
    counter.load(Ordering::Relaxed)
}

/// A finger at `y` from `from` to `to` in steps of `step` points, then a
/// release where it stopped.
fn swipe(y: u32, from: u32, to: u32, step: u32) -> String {
    let mut lines = format!("{from} {y} b\n");
    let mut x = from;
    while x < to {
        x = (x + step).min(to);
        writeln!(lines, "{x} {y} m").unwrap();
    }
    writeln!(lines, "{x} {y} e").unwrap();
    lines
}

fn screen_layer_x() -> f32 {
    from_main(|| UIManager::root_view().screen_layer().x())
}

/// The screen that is the app views now, in place and with no swipe
/// running.
fn settled_on(name: &'static str) -> bool {
    let layer = UIManager::root_view().screen_layer();
    layer.x().abs() < 0.5
        && !UIManager::touch_disabled()
        && layer.subviews().len() == 1
        && layer.subviews()[0].label().ends_with(name)
}

fn titled(label: Weak<Label>, text: &str, color: Color) {
    label.set_text(text).set_text_size(24).set_color(color);
    label.place().lrt(0).h(80);
}

/// The first screen, a button at its left edge. A swipe from the edge goes
/// back to the second screen, which swipes back to the third.
#[view]
struct SwipeBackTest {
    #[init]
    title:  Label,
    button: Button,
}

impl Setup for SwipeBackTest {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.85, 0.3, 0.3));
        titled(self.title, "first screen", Color::rgb(0.95, 0.75, 0.75));

        self.button.set_text("edge button").set_color(Color::rgb(1.0, 0.9, 0.5));
        self.button.place().l(0).center_y().size(200, 100);
        self.button.on_tap(|| {
            TAPS.fetch_add(1, Ordering::Relaxed);
        });

        SwipeBack::set_with_done(
            self,
            || SecondScreen::new(),
            || {
                DONE.fetch_add(1, Ordering::Relaxed);
            },
        );
    }
}

#[view]
struct SecondScreen {
    #[init]
    title: Label,
}

impl Setup for SecondScreen {
    fn setup(self: Weak<Self>) {
        SECOND_BUILT.fetch_add(1, Ordering::Relaxed);
        self.set_color(Color::rgb(0.3, 0.45, 0.85));
        titled(self.title, "second screen", Color::rgb(0.75, 0.82, 0.95));
        SwipeBack::set(self, || ThirdScreen::new());
    }
}

#[view]
struct ThirdScreen {
    #[init]
    title: Label,
}

impl Setup for ThirdScreen {
    fn setup(self: Weak<Self>) {
        THIRD_BUILT.fetch_add(1, Ordering::Relaxed);
        self.set_color(Color::rgb(0.3, 0.7, 0.35));
        titled(self.title, "third screen", Color::rgb(0.75, 0.92, 0.78));
    }
}

impl ViewTest for SwipeBackTest {
    // A finger gesture, it swipes only where drag scrolling is on, the
    // touch platform default.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
        for counter in [&TAPS, &SECOND_BUILT, &THIRD_BUILT, &DONE] {
            counter.store(0, Ordering::Relaxed);
        }
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches(swipe(450, 300, 500, 20));
        ensure!(
            count(&SECOND_BUILT) == 0,
            "a drag from the middle started a swipe"
        );

        inject_touches("10 300 b\n10 300 e");
        ensure!(count(&TAPS) == 1, "a tap at the edge did not reach the button");

        inject_touches("10 300 b\n12 330 m\n20 420 m\n20 420 e");
        ensure!(
            count(&SECOND_BUILT) == 0,
            "a drag down from the edge started a swipe"
        );
        check_colors(
            r"
               4    4 - #f2bfbf
             592    4 - #f2bfbf
             244   36 - #000000
             276   40 - #987878
             296   40 - #f2bfbf
             328   40 - #aa8686
             340   40 - #aa8686
             268   44 - #f2bfbf
             316   44 - #2c2222
             344   44 - #f2bfbf
             476   76 - #f2bfbf
               4  252 - #ffe580
             108  252 - #ffe580
             592  260 - #d94d4d
              44  288 - #c2ae61
              96  288 - #000000
             140  296 - #c9b565
              44  300 - #c2ae61
              60  300 - #ffe580
              76  300 - #e6cf73
             108  300 - #000000
             160  300 - #ffe580
             180  300 - #010101
              24  304 - #ffe580
              96  304 - #000000
             152  304 - #000000
             172  304 - #827541
             124  308 - #000000
             172  308 - #827541
              84  348 - #ffe580
             284  592 - #d94d4d
             592  592 - #d94d4d
            ",
        )?;

        held_swipe(view)?;
        full_swipe(view)?;
        fling()
    }
}

fn held_swipe(view: Weak<SwipeBackTest>) -> Result<()> {
    // A short swipe that starts on the button and is let go before half.
    let mut held = swipe(300, 10, 160, 5);
    held.truncate(held.trim_end().rfind('\n').unwrap_or(0) + 1);
    inject_touches(held);
    ensure!(count(&SECOND_BUILT) == 1, "the screen below was not built");
    ensure!(
        (screen_layer_x() - 150.0).abs() < 1.0,
        "the screen does not follow the finger: {}",
        screen_layer_x()
    );
    check_colors(
        r"
              28    4 - #bfd1f2
             244    4 - #f2bfbf
             592    4 - #f2bfbf
              96   40 - #bfd1f2
             428   40 - #322828
             464   40 - #9e7d7d
             476   40 - #aa8686
             120   44 - #bfd1f2
             148   44 - #000000
             392   44 - #bd9595
             444   44 - #f2bfbf
             464   44 - #9e7d7d
             492   44 - #f2bfbf
             500   44 - #d6a9a9
               4   76 - #bfd1f2
             152  252 - #ffe580
             224  252 - #ffe580
             196  288 - #000000
             284  292 - #605630
             276  296 - #bfac60
             228  300 - #e6cf73
             276  300 - #bfac60
             284  300 - #605630
             320  300 - #000000
             172  304 - #ffe580
             276  304 - #bfac60
             292  304 - #231f12
             196  308 - #000000
               4  400 - #4d73d9
               4  592 - #4d73d9
             300  592 - #d94d4d
             592  592 - #d94d4d
            ",
    )?;
    checkpoint("the first screen follows the finger, the second shows behind")?;

    let_go_early(view)
}

fn let_go_early(view: Weak<SwipeBackTest>) -> Result<()> {
    inject_touches("160 300 e");
    wait_until("the first screen slid back", || settled_on("SwipeBackTest"))?;
    ensure!(count(&TAPS) == 1, "the release of a swipe tapped the button");
    ensure!(count(&DONE) == 0, "a swipe that went back to its start ran done");
    ensure!(
        from_main(move || view.is_ok()),
        "a short swipe left the first screen"
    );
    check_colors(
        r"
               4    4 - #f2bfbf
             592    4 - #f2bfbf
             244   36 - #000000
             276   40 - #987878
             296   40 - #f2bfbf
             328   40 - #aa8686
             340   40 - #aa8686
             268   44 - #f2bfbf
             316   44 - #2c2222
             344   44 - #f2bfbf
             476   76 - #f2bfbf
               4  252 - #ffe580
             108  252 - #ffe580
             592  260 - #d94d4d
              44  288 - #c2ae61
              96  288 - #000000
             140  296 - #c9b565
              44  300 - #c2ae61
              60  300 - #ffe580
              76  300 - #e6cf73
             108  300 - #000000
             160  300 - #ffe580
             180  300 - #010101
              24  304 - #ffe580
              96  304 - #000000
             152  304 - #000000
             172  304 - #827541
             124  308 - #000000
             172  308 - #827541
              84  348 - #ffe580
             284  592 - #d94d4d
             592  592 - #d94d4d
            ",
    )?;
    checkpoint("let go before half, the first screen slid back")?;

    Ok(())
}

fn full_swipe(view: Weak<SwipeBackTest>) -> Result<()> {
    inject_touches(swipe(450, 10, 400, 5));
    wait_until("the second screen is the app views", || {
        settled_on("SecondScreen")
    })?;
    ensure!(count(&DONE) == 1, "done ran {} times", count(&DONE));
    wait_until("the first screen is gone", move || view.is_null())?;
    check_colors(
        r"
             112    4 - #bfd1f2
             492    4 - #bfd1f2
             296   32 - #555d6c
             280   36 - #000000
             312   36 - #000000
             372   36 - #010101
             240   40 - #8693aa
             268   40 - #bfd1f2
             332   40 - #a3b3cf
             344   40 - #8693aa
             228   44 - #bfd1f2
             252   44 - #bfd1f2
             276   44 - #000000
             284   44 - #000000
             292   44 - #bfd1f2
             316   44 - #010101
             324   44 - #bfd1f2
             332   44 - #a3b3cf
             348   44 - #bfd1f2
             360   44 - #bfd1f2
             368   44 - #6f798c
             376   44 - #94a2bc
               4   76 - #bfd1f2
             152   76 - #bfd1f2
             452   76 - #bfd1f2
             592   76 - #bfd1f2
              44  332 - #4d73d9
             548  332 - #4d73d9
             296  396 - #4d73d9
             592  588 - #4d73d9
               4  592 - #4d73d9
             380  592 - #4d73d9
            ",
    )?;
    checkpoint("let go past half, the second screen is the app views")?;

    Ok(())
}

fn fling() -> Result<()> {
    // A fling, a fast last step well before half.
    let mut fling = swipe(450, 10, 60, 5);
    fling.truncate(fling.trim_end().rfind('\n').unwrap_or(0) + 1);
    fling.push_str("90 450 m\n90 450 e\n");
    inject_touches(fling);
    wait_until("the third screen is the app views", || settled_on("ThirdScreen"))?;
    ensure!(
        count(&THIRD_BUILT) == 1,
        "the third screen was built {} times",
        count(&THIRD_BUILT)
    );
    check_colors(
        r"
             116    4 - #bfebc7
             440    4 - #bfebc7
             592    4 - #bfebc7
             248   32 - #364238
             252   36 - #000000
             296   36 - #000000
             356   36 - #000000
             240   40 - #000000
             280   40 - #bfebc7
             300   40 - #bfebc7
             308   40 - #bfebc7
             312   40 - #bfebc7
             344   40 - #86a58c
             352   40 - #000000
             360   40 - #000000
             248   44 - #364238
             256   44 - #5b705f
             260   44 - #010101
             320   44 - #a9cfb0
             332   44 - #bfebc7
             344   44 - #bfebc7
             348   44 - #bfebc7
               4   76 - #bfebc7
             160   76 - #bfebc7
             484   76 - #bfebc7
             564  292 - #4db259
              40  332 - #4db259
             292  392 - #4db259
             192  552 - #4db259
             592  580 - #4db259
               4  592 - #4db259
             376  592 - #4db259
            ",
    )?;
    checkpoint("a fling went on to the third screen")?;

    Ok(())
}
