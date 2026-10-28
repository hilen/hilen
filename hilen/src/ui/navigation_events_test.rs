use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::from_main,
        refs::{Own, Weak},
    },
    ui::{
        Button, Color, Label, NamedKey, NavigationView, Setup, UIManager, View, ViewController, ViewData,
        ViewFrame, ViewSubviews, ViewTest,
        navigation_swipe_test::{held, swipe},
        view,
    },
    ui_test::{checkpoint, inject_named_key, inject_touches, system_input::wait_until},
};

static POPPED: AtomicU32 = AtomicU32::new(0);
static POPPED_ALIVE: AtomicU32 = AtomicU32::new(0);
static RETURNED: AtomicU32 = AtomicU32::new(0);

fn count(counter: &AtomicU32) -> u32 {
    counter.load(Ordering::Relaxed)
}

fn add(counter: &AtomicU32) {
    counter.fetch_add(1, Ordering::Relaxed);
}

/// `depth` pages on the stack, the top one in its place, no slide running.
fn settled(nav: Weak<NavigationView>, depth: usize) -> bool {
    from_main(move || {
        nav.subviews().len() == depth
            && !UIManager::touch_disabled()
            && nav.subviews().last().is_some_and(|top| top.x().abs() < f32::EPSILON)
    })
}

/// The first page. It counts each time it is on top again.
#[view]
struct NavigationEvents {
    #[init]
    title: Label,
    push:  Button,
}

impl Setup for NavigationEvents {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.85, 0.3, 0.3));
        self.title.set_text("first page").set_text_size(24);
        self.title.place().lrt(0).h(80);

        self.push.set_text("push").set_color(Color::rgb(0.95, 0.95, 0.95));
        self.push.place().center().size(220, 64);
        self.push.on_tap(move || self.navigation().push(EventsPage::new()));

        self.on_return(|| add(&RETURNED));
    }
}

/// The pushed page. It counts each time it leaves and whether it was still
/// alive then.
#[view]
struct EventsPage {
    #[init]
    title: Label,
}

impl Setup for EventsPage {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.3, 0.45, 0.85));
        self.title.set_text("pushed page").set_text_size(24);
        self.title.place().lrt(0).h(80);

        let back = self.back_button();
        back.set_color(Color::rgb(0.2, 0.2, 0.2)).set_corner_radius(12);
        back.place().l(420).t(100).size(160, 60);

        self.on_pop(move || {
            add(&POPPED);
            if self.is_ok() && !self.title.text().is_empty() {
                add(&POPPED_ALIVE);
            }
        });
    }
}

impl ViewTest for NavigationEvents {
    fn make_root(view: Own<Self>) -> Own<dyn View> {
        NavigationView::with_view(view)
    }

    fn before_start() {
        UIManager::set_drag_scrolling(true);
        for counter in [&POPPED, &POPPED_ALIVE, &RETURNED] {
            counter.store(0, Ordering::Relaxed);
        }
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let nav = from_main(move || view.navigation());

        push(nav)?;
        ensure!(
            count(&POPPED) == 0 && count(&RETURNED) == 0,
            "a push ran an event"
        );

        inject_named_key(NamedKey::Escape);
        wait_until("Escape popped the page", move || settled(nav, 1))?;
        expect(1)?;
        checkpoint("Escape went back, 1 pop and 1 return")?;

        push(nav)?;
        inject_touches("500 130 b\n500 130 e");
        wait_until("the back button popped the page", move || settled(nav, 1))?;
        expect(2)?;
        checkpoint("the back button went back, 2 pops and 2 returns")?;

        push(nav)?;
        inject_touches(swipe(450, 10, 400, 5));
        wait_until("the swipe popped the page", move || settled(nav, 1))?;
        expect(3)?;
        checkpoint("the swipe went back, 3 pops and 3 returns")?;

        push(nav)?;
        inject_touches(held(450, 10, 160, 5));
        inject_touches("160 450 e");
        wait_until("the page slid back", move || settled(nav, 2))?;
        expect(3)?;
        checkpoint("a swipe let go before half, no event")?;

        inject_named_key(NamedKey::Escape);
        wait_until("Escape popped the page again", move || settled(nav, 1))?;
        expect(4)?;
        checkpoint("Escape after the slide back, 4 pops and 4 returns")?;
        Ok(())
    }
}

fn push(nav: Weak<NavigationView>) -> Result<()> {
    inject_touches("300 300 b\n300 300 e");
    wait_until("the page was pushed", move || settled(nav, 2))
}

fn expect(times: u32) -> Result<()> {
    ensure!(
        count(&POPPED) == times,
        "{} pops, {times} expected",
        count(&POPPED)
    );
    ensure!(
        count(&POPPED_ALIVE) == times,
        "{} pops saw a live page, {times} expected",
        count(&POPPED_ALIVE)
    );
    ensure!(
        count(&RETURNED) == times,
        "{} returns, {times} expected",
        count(&RETURNED)
    );
    Ok(())
}
