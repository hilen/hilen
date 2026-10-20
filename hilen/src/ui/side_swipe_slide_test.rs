use std::{thread::sleep, time::Duration};

use anyhow::{Result, bail, ensure};

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::Animation,
    ui::{
        Button, Color, Container, Label, Setup, SideSwipe, SideSwipeEnd, SideSwipeEvent, UIAnimation,
        UIManager, ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{checkpoint, inject_touches},
};

const PAGES: [(f32, f32, f32); 3] = [(0.35, 0.55, 0.85), (0.85, 0.45, 0.35), (0.4, 0.75, 0.45)];
const SLIDE_SECONDS: f32 = 0.2;
const GIVE_UP_AFTER: Duration = Duration::from_secs(10);

/// A finger at `y` from `from` to `to` in steps of 10 points, then a release.
fn swipe(y: u32, from: u32, to: u32) -> String {
    let mut lines = format!("{from} {y} b\n");
    let mut x = from;
    while x != to {
        x = if to > x {
            (x + 10).min(to)
        } else {
            x.saturating_sub(10).max(to)
        };
        lines.push_str(&format!("{x} {y} m\n"));
    }
    lines.push_str(&format!("{x} {y} e\n"));
    lines
}

/// Pages that a swipe changes the way an app does it: the page slides out
/// with an animation, and the finish of that animation shows the next page
/// and starts the slide in from the other side. banda froze at that second
/// animation, see `UIManager::commit_animations`.
#[view]
struct SideSwipeSlide {
    index:   usize,
    /// Slides in that finished, each one is a page switch done.
    arrived: u32,
    taps:    u32,

    #[init]
    status: Label,
    page:   Container,
    title:  Label,
    button: Button,
}

impl SideSwipeSlide {
    fn swiped(self: Weak<Self>, event: SideSwipeEvent) {
        match event {
            SideSwipeEvent::Moved(offset) => {
                let mut page = self.page;
                page.set_x(offset);
            }
            SideSwipeEvent::Ended(SideSwipeEnd::Left) => self.slide(-self.width(), Some(1)),
            SideSwipeEvent::Ended(SideSwipeEnd::Right) => self.slide(self.width(), Some(-1)),
            SideSwipeEvent::Ended(SideSwipeEnd::Back) => self.slide(0.0, None),
        }
    }

    /// Slides the page to `to`. With `then` the page that many steps away
    /// shows when it is there, and slides in from the other side.
    fn slide(self: Weak<Self>, to: f32, then: Option<isize>) {
        let from = self.page.x();
        let animation = UIAnimation::new(|page, x| {
            page.set_x(x);
        })
        .animation(Animation::new(from, to, SLIDE_SECONDS));
        animation.on_finish.sub(move || self.slid(to, then));
        self.page.add_animation(animation);
    }

    fn slid(mut self: Weak<Self>, to: f32, then: Option<isize>) {
        let mut page = self.page;
        let Some(step) = then else {
            page.set_x(0);
            self.arrived += 1;
            self.status.set_text(format!("page {} is in place", self.index + 1));
            return;
        };
        self.index = self.index.saturating_add_signed(step).min(PAGES.len() - 1);
        self.show_page();
        page.set_x(-to);
        self.slide(0.0, None);
    }

    fn show_page(self: Weak<Self>) {
        let (r, g, b) = PAGES[self.index];
        self.page.set_color(Color::rgb(r, g, b));
        self.title.set_text(format!("page {}", self.index + 1));
    }
}

impl Setup for SideSwipeSlide {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.2, 0.2, 0.25));
        self.status
            .set_text("no swipe yet")
            .set_text_size(24)
            .set_color(Color::rgb(0.95, 0.9, 0.7));
        self.status.place().lrt(0).h(80);
        self.page.place().lrb(0).t(80);
        self.title.set_text_size(32).set_text_color(Color::rgb(0.95, 0.95, 0.95));
        self.title.place().t(100).l(20).size(300, 50);
        self.button.set_text("button").set_color(Color::rgb(1.0, 0.9, 0.5));
        self.button.place().t(200).l(200).size(200, 100);
        self.button.on_tap(move || {
            let mut view = self;
            view.taps += 1;
            view.status.set_text(format!("taps {}", view.taps));
        });
        self.show_page();
        SideSwipe::set(self.page, move |event| self.swiped(event));
    }
}

/// Waits until `arrived` slides in finished, with the app still answering.
fn wait_arrived(view: Weak<SideSwipeSlide>, arrived: u32) -> Result<()> {
    let start = web_time::Instant::now();
    while from_main(move || view.arrived) < arrived {
        if start.elapsed() > GIVE_UP_AFTER {
            bail!("the slide in after a page switch never finished, the app is stuck");
        }
        sleep(Duration::from_millis(30));
    }
    Ok(())
}

impl ViewTest for SideSwipeSlide {
    // A finger gesture, it swipes only where drag scrolling is on, the
    // touch platform default.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        checkpoint("page 1, before any swipe")?;

        inject_touches(swipe(400, 500, 100));
        wait_arrived(view, 1)?;
        let (index, x) = from_main(move || (view.index, view.page.x()));
        ensure!(
            index == 1,
            "a swipe left did not switch to page 2, it is on page {}",
            index + 1
        );
        ensure!(x.abs() < 0.5, "page 2 did not slide all the way in, it is at {x}");
        checkpoint("swiped left, page 2 slid in from the right")?;

        inject_touches("300 250 b\n300 250 e");
        ensure!(
            view.taps == 1,
            "a tap after the page switch did not reach the button"
        );

        inject_touches(swipe(400, 100, 500));
        wait_arrived(view, 2)?;
        let index = from_main(move || view.index);
        ensure!(index == 0, "a swipe right did not switch back to page 1");
        checkpoint("swiped right, page 1 slid back in from the left")?;
        Ok(())
    }
}
