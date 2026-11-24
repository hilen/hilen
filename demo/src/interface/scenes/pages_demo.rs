use hilen::{
    refs::{Own, Weak},
    ui::{Color, Font, Label, PageSource, PageView, Setup, TextAlignment, View, ViewData, WHITE, view},
};
use log::info;

/// The pages of `uikit-reference` in the engine repo, the same 5 colors and
/// names, so the 2 can be compared side by side in the simulator with
/// `make sim args="reference beside"`.
const COLORS: [(f32, f32, f32); 5] = [
    (0.20, 0.47, 0.96),
    (0.95, 0.45, 0.20),
    (0.18, 0.70, 0.42),
    (0.62, 0.33, 0.85),
    (0.90, 0.25, 0.40),
];

#[view]
struct DemoPage {
    number: usize,

    #[init]
    label: Label,
}

impl Setup for DemoPage {
    fn setup(self: Weak<Self>) {
        let (r, g, b) = COLORS[(self.number.max(1) - 1) % COLORS.len()];
        self.set_color(Color::rgb(r, g, b));
        self.label.set_alignment(TextAlignment::Center);
        self.label
            .set_text(format!("Page {}", self.number))
            .set_font(Font::bold())
            .set_text_size(44)
            .set_text_color(WHITE);
        self.label.place().back();
    }
}

/// A `PageView` of 5 pages that a finger turns.
#[view]
pub struct PagesDemo {
    #[init]
    pager: PageView,
}

impl Setup for PagesDemo {
    fn setup(self: Weak<Self>) {
        self.pager.place().back();
        self.pager.set_source(self).set_page("1");
        self.pager.turned.val(|turn| {
            info!("pages demo shows page {}, turned {}", turn.key, turn.completed);
        });
    }
}

impl PageSource for PagesDemo {
    fn page_beside(&self, key: &str, forward: bool) -> Option<String> {
        let number: usize = key.parse().ok()?;
        if forward {
            (number < COLORS.len()).then(|| (number + 1).to_string())
        } else {
            (number > 1).then(|| (number - 1).to_string())
        }
    }

    fn make_page(&mut self, key: &str) -> Own<dyn View> {
        let mut page = DemoPage::new();
        page.number = key.parse().unwrap_or(1);
        page
    }
}
