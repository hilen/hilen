use std::fmt::Write;

use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    ui::{
        Button, Color, Container, Label, ScrollView, Setup, SideSwipe, SideSwipeEnd, SideSwipeEvent,
        UIManager, ViewData, ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint, inject_touches},
};

/// A finger at `y` from `from` to `to` in steps of `step` points, then a
/// release where it stopped.
fn swipe(y: u32, from: u32, to: u32, step: u32) -> String {
    let mut lines = format!("{from} {y} b\n");
    let mut x = from;
    while x != to {
        x = if to > x {
            (x + step).min(to)
        } else {
            x.saturating_sub(step).max(to)
        };
        writeln!(lines, "{x} {y} m").unwrap();
    }
    writeln!(lines, "{x} {y} e").unwrap();
    lines
}

/// The same swipe with no release, the finger is still down.
fn held(y: u32, from: u32, to: u32, step: u32) -> String {
    let mut lines = swipe(y, from, to, step);
    lines.truncate(lines.trim_end().rfind('\n').unwrap_or(0) + 1);
    lines
}

/// A scroll view that moves up and down, with a page in it that follows a
/// sideways swipe, and a strip under it that scrolls sideways by itself.
#[view]
struct SideSwipeTest {
    taps:   u32,
    moves:  u32,
    offset: f32,
    lefts:  u32,
    rights: u32,
    backs:  u32,
    page:   Weak<Container>,
    strip:  Weak<ScrollView>,

    #[init]
    status: Label,
    scroll: ScrollView,
}

impl SideSwipeTest {
    fn swiped(mut self: Weak<Self>, event: SideSwipeEvent) {
        let mut page = self.page;
        match event {
            SideSwipeEvent::Moved(offset) => {
                self.moves += 1;
                self.offset = offset;
                page.set_x(offset);
                self.status.set_text(format!("page moved {offset}"));
            }
            SideSwipeEvent::Ended(end) => {
                match end {
                    SideSwipeEnd::Left => self.lefts += 1,
                    SideSwipeEnd::Right => self.rights += 1,
                    SideSwipeEnd::Back => self.backs += 1,
                }
                page.set_x(0);
                self.status.set_text(format!("went {end:?}"));
            }
        }
    }
}

impl Setup for SideSwipeTest {
    fn setup(mut self: Weak<Self>) {
        self.set_color(Color::rgb(0.2, 0.2, 0.25));
        self.status
            .set_text("no swipe yet")
            .set_text_size(24)
            .set_color(Color::rgb(0.95, 0.9, 0.7));
        self.status.place().lrt(0).h(80);

        self.scroll.set_color(Color::rgb(0.3, 0.3, 0.35));
        self.scroll.place().lr(0).t(80).b(120);
        self.scroll.set_content_height(1200);

        self.page = self.scroll.add_view::<Container>();
        let page = self.page;
        page.set_color(Color::rgb(0.35, 0.55, 0.85));
        page.set_frame((0, 0, 600, 1200));

        let button = page.add_view::<Button>();
        button.set_text("button").set_color(Color::rgb(1.0, 0.9, 0.5));
        button.set_frame((200, 100, 200, 100));
        button.on_tap(move || {
            self.taps += 1;
            self.status.set_text(format!("taps {}", self.taps));
        });

        self.strip = self.add_view::<ScrollView>();
        let mut strip = self.strip;
        strip.set_color(Color::rgb(0.3, 0.7, 0.4));
        strip.place().lrb(0).h(120);
        strip.set_content_width(1500);
        let label = strip.add_view::<Label>();
        label
            .set_text("sideways scroll")
            .set_text_size(24)
            .set_color(Color::rgb(0.75, 0.92, 0.78));
        label.set_frame((20, 20, 300, 60));

        SideSwipe::set(self, move |event| self.swiped(event));
    }
}

impl ViewTest for SideSwipeTest {
    // A finger gesture, it swipes only where drag scrolling is on, the
    // touch platform default.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
               4    4 - #f2e5b2
             592    4 - #f2e5b2
             236   40 - #000000
             308   40 - #f2e5b2
             352   40 - #aaa17d
             252   44 - #f2e5b2
             272   44 - #f2e5b2
             304   48 - #3b382b
             396  180 - #ffe580
             256  220 - #17150c
             284  228 - #726639
             292  228 - #bca95e
             320  228 - #ffe580
             260  232 - #ffe580
             292  232 - #bca95e
             308  232 - #ffe580
             324  236 - #000000
             396  276 - #ffe580
               4  288 - #598cd9
             592  476 - #598cd9
             264  500 - #bfebc7
              20  504 - #bfebc7
             116  524 - #2f3a31
             104  528 - #38453b
             156  528 - #bfebc7
             180  528 - #bfebc7
             220  528 - #0a0d0b
             112  532 - #bfebc7
             128  532 - #667d6a
             248  536 - #000000
             372  592 - #4db266
             512  592 - #4db266
            ",
        )?;

        inject_touches("300 230 b\n300 230 e");
        ensure!(view.taps == 1, "a tap did not reach the button");

        held_then_back(view)?;
        left_and_right(view)?;
        sideways_strip(view)?;
        scrolls_up_and_down(view)
    }
}

fn held_then_back(view: Weak<SideSwipeTest>) -> Result<()> {
    inject_touches(held(230, 300, 150, 5));
    ensure!(
        (view.offset + 150.0).abs() < 0.5,
        "the page does not follow the finger: {}",
        view.offset
    );
    ensure!(
        from_main(move || view.scroll.get_scroll_content_offset()).abs() < 0.5,
        "a swipe scrolled the page up or down"
    );
    check_colors(
        r"
               4    4 - #f2e5b2
             592    4 - #f2e5b2
             360   32 - #d0c599
             360   36 - #d0c599
             384   36 - #f2e5b2
             268   40 - #504b3b
             296   40 - #f2e5b2
             320   40 - #aaa17d
             360   40 - #d0c599
             232   44 - #312f24
             268   44 - #504b3b
             276   44 - #211f18
             248  180 - #ffe580
             592  216 - #4d4d59
             112  228 - #ffe580
             156  228 - #e0c970
             156  232 - #e0c970
             180  232 - #9a8a4d
             168  236 - #ffe580
             180  236 - #9a8a4d
             192  236 - #010101
             592  376 - #4d4d59
               4  380 - #598cd9
             448  448 - #598cd9
             248  524 - #000000
             196  528 - #010101
             112  532 - #bfebc7
             128  532 - #667d6a
             164  536 - #bfebc7
             304  556 - #bfebc7
             444  592 - #4db266
             592  592 - #4db266
            ",
    )?;
    checkpoint("the page follows the finger 150 points to the left")?;

    inject_touches("150 230 e");
    ensure!(view.backs == 1, "a short slow swipe did not go back");
    ensure!(view.taps == 1, "the release of a swipe tapped the button");
    check_colors(
        r"
             112    4 - #f2e5b2
             592    4 - #f2e5b2
             344   32 - #b6ac86
             308   36 - #f2e5b2
             268   40 - #aaa17d
             336   40 - #f2e5b2
             304   44 - #756e56
             328   44 - #312f24
             344   44 - #b6ac86
               4  124 - #598cd9
             396  180 - #ffe580
             256  220 - #17150c
             284  228 - #726639
             320  228 - #ffe580
             260  232 - #ffe580
             292  232 - #bca95e
             308  232 - #ffe580
             324  236 - #000000
             396  276 - #ffe580
             592  476 - #598cd9
             252  500 - #bfebc7
              20  504 - #bfebc7
             116  524 - #2f3a31
             148  528 - #010101
             172  528 - #000000
             212  528 - #bfebc7
             232  528 - #bfebc7
             112  532 - #bfebc7
             128  532 - #667d6a
             248  536 - #000000
             364  592 - #4db266
             508  592 - #4db266
            ",
    )?;
    checkpoint("let go before half, the page went back")?;
    Ok(())
}

fn left_and_right(view: Weak<SideSwipeTest>) -> Result<()> {
    inject_touches(swipe(400, 500, 100, 5));
    ensure!(view.lefts == 1, "a swipe past half did not go left");

    let mut fling = held(400, 100, 150, 5);
    fling.push_str("180 400 m\n180 400 e\n");
    inject_touches(fling);
    ensure!(view.rights == 1, "a fling did not go right");
    Ok(())
}

fn sideways_strip(view: Weak<SideSwipeTest>) -> Result<()> {
    let moves = view.moves;
    inject_touches(swipe(540, 500, 200, 10));
    ensure!(
        view.moves == moves,
        "a swipe on the sideways strip started a swipe"
    );
    ensure!(
        from_main(move || view.strip.content_offset_x()) < 0.0,
        "the sideways strip did not scroll"
    );
    check_colors(
        r"
             128    4 - #f2e5b2
             592    4 - #f2e5b2
             308   36 - #f2e5b2
             268   40 - #aaa17d
             320   40 - #d8cd9f
             328   40 - #f2e5b2
             272   44 - #f2e5b2
             284   44 - #010101
             320   44 - #d8cd9f
             344   44 - #cdc297
             328   52 - #010101
               4   76 - #f2e5b2
             396  180 - #ffe580
             256  220 - #17150c
             284  228 - #726639
             292  228 - #bca95e
             332  228 - #000000
             592  228 - #598cd9
             260  232 - #ffe580
             292  232 - #bca95e
             304  232 - #000000
             316  232 - #ffe580
             276  236 - #000000
             296  236 - #ffe580
             324  236 - #000000
             200  276 - #ffe580
             396  276 - #ffe580
              24  336 - #598cd9
             592  476 - #598cd9
             156  480 - #4db266
             308  588 - #4db266
               4  592 - #4db266
            ",
    )?;
    checkpoint("the sideways strip scrolled, the page did not move")?;
    Ok(())
}

/// The last steps scroll the page up, a fling under a later tap would only
/// stop it.
fn scrolls_up_and_down(mut view: Weak<SideSwipeTest>) -> Result<()> {
    let moves = view.moves;
    inject_touches("300 440 b\n302 425 m\n400 420 m\n450 420 e");
    ensure!(view.moves == moves, "a drag that went up first started a swipe");
    from_main(move || {
        view.scroll.set_content_offset(0);
    });
    inject_touches("300 440 b\n300 420 m\n300 380 m\n300 300 m\n300 300 e");
    ensure!(view.moves == moves, "a drag up started a swipe");
    ensure!(
        from_main(move || view.scroll.get_scroll_content_offset()) < 0.0,
        "a drag up did not scroll"
    );
    Ok(())
}
