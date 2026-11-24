use hilen::{
    dispatch::from_main,
    refs::{Own, Weak},
    ui::{
        Color, Label, PageSource, PageTurn, PageView, Setup, TextAlignment, View, ViewData, ViewFrame, WHITE,
        WeakView, view,
    },
    ui_test::{inject_touches, step_frames},
};

/// The width of the pages, close to the 402 points of the iPhone the moves
/// were measured on.
pub(super) const WIDTH: f32 = 400.0;
pub(super) const HEIGHT: f32 = 300.0;
pub(super) const PAGES: usize = 5;

/// The colors of the pages of `uikit-reference`, so a test picture and the
/// real `UIPageViewController` show the same.
const COLORS: [(f32, f32, f32); PAGES] = [
    (0.20, 0.47, 0.96),
    (0.95, 0.45, 0.20),
    (0.18, 0.70, 0.42),
    (0.62, 0.33, 0.85),
    (0.90, 0.25, 0.40),
];

/// 1 page, its color and its number.
#[view]
pub(super) struct NumberPage {
    pub(super) number: usize,

    #[init]
    label: Label,
}

impl Setup for NumberPage {
    fn setup(self: Weak<Self>) {
        let (r, g, b) = COLORS[(self.number.max(1) - 1) % PAGES];
        self.set_color(Color::rgb(r, g, b));
        self.label.set_alignment(TextAlignment::Center);
        self.label
            .set_text(format!("Page {}", self.number))
            .set_text_size(44)
            .set_text_color(WHITE);
        self.label.place().back();
    }
}

/// 5 pages keyed "1" to "5" with what the page view told.
#[view]
pub(super) struct Pages {
    pub(super) wills:   Vec<String>,
    pub(super) turns:   Vec<PageTurn>,
    pub(super) made:    Vec<String>,
    pub(super) dropped: Vec<String>,

    #[init]
    pub(super) pager: PageView,
}

impl Setup for Pages {
    fn setup(mut self: Weak<Self>) {
        self.set_color(Color::rgb(0.1, 0.1, 0.12));
        self.pager.set_frame((0, 0, WIDTH, HEIGHT));
        self.pager.set_source(self);
        self.pager.will_turn.val(move |key| self.wills.push(key));
        self.pager.turned.val(move |turn| self.turns.push(turn));
        self.pager.set_page("1");
    }
}

impl PageSource for Pages {
    fn page_beside(&self, key: &str, forward: bool) -> Option<String> {
        let number: usize = key.parse().ok()?;
        if forward {
            (number < PAGES).then(|| (number + 1).to_string())
        } else {
            (number > 1).then(|| (number - 1).to_string())
        }
    }

    fn make_page(&mut self, key: &str) -> Own<dyn View> {
        let mut page = NumberPage::new();
        page.number = key.parse().unwrap_or(1);
        page
    }

    fn page_made(&mut self, key: &str, view: WeakView) {
        if view.is_ok() {
            self.made.push(key.to_string());
        }
    }

    fn page_dropped(&mut self, key: &str, view: WeakView) {
        if view.is_ok() {
            self.dropped.push(key.to_string());
        }
    }
}

/// How far the pages moved from page 1 toward the last page, in points. It
/// counts across the open page, which changes at half a page.
pub(super) fn progress(pages: Weak<Pages>) -> f32 {
    from_main(move || {
        let number: f32 = pages.pager.page().and_then(|key| key.parse::<u8>().ok()).map_or(1.0, f32::from);
        (number - 1.0) * WIDTH - pages.pager.offset()
    })
}

/// A finger lands, 1 frame of 1/60 s passes.
pub(super) fn press(x: f32, y: f32) {
    touch(x, y, "b");
}

/// The finger moves from `from` to `to` on the line at `y`, `step` points a
/// frame, so it goes `step` times 60 points a second.
pub(super) fn slide(y: f32, from: f32, to: f32, step: f32) {
    let mut x = from;
    while (x - to).abs() > 0.01 {
        x = if to > x {
            (x + step).min(to)
        } else {
            (x - step).max(to)
        };
        touch(x, y, "m");
    }
}

pub(super) fn lift(x: f32, y: f32) {
    touch(x, y, "e");
}

/// The frames a glide takes at most, about a second.
pub(super) fn settle() {
    step_frames(70);
}

fn touch(x: f32, y: f32, kind: &str) {
    inject_touches(format!("{x} {y} {kind}"));
    step_frames(1);
}
