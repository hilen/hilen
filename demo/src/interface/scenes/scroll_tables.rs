use fake::{
    Fake,
    faker::lorem::en::Sentence,
    rand::{SeedableRng, rngs::StdRng},
};
use hilen::{
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    ui::{
        Anchor, CellRegistry, Container, Font, Label, ScrollView, Setup, Shadow, TableData, TableView,
        TextAlignment, View, ViewData, ViewFrame, ViewSubviews, WHITE, view,
    },
};

use crate::interface::{
    palette::{ACCENT, ACCENT_END, ACCENT_START, BORDER, SURFACE, SURFACE_ALT, TEXT, TEXT_DIM},
    scenes::{HEADER_HEIGHT, add_title},
};

const SCROLL_ROWS: usize = 200;
const TABLE_ROWS: usize = 10_000_000;
const PANEL_HEADER_HEIGHT: f32 = 92.0;
const ROW_HEIGHT: f32 = 56.0;
const ROW_SPACING: f32 = 8.0;
const MANUAL_TEXT_SEED: u64 = 0xA11C_E5EED;
const TABLE_TEXT_SEED: u64 = 0x7AB1_E5EED;

fn row_text(index: usize, seed: u64) -> (String, String) {
    let row = u64::try_from(index).expect("row index fits into u64");
    let mut rng = StdRng::seed_from_u64(row ^ seed);
    (
        Sentence(3..6).fake_with_rng(&mut rng),
        Sentence(4..7).fake_with_rng(&mut rng),
    )
}

#[view]
struct PanelHeader {
    #[init]
    accent:  Container,
    eyebrow: Label,
    title:   Label,
}

impl PanelHeader {
    fn set_content(self: Weak<Self>, eyebrow: &str, title: &str) -> Weak<Self> {
        self.eyebrow.set_text(eyebrow);
        self.title.set_text(title);
        self
    }
}

impl Setup for PanelHeader {
    fn setup(self: Weak<Self>) {
        self.set_color(SURFACE);

        self.accent.set_gradient(ACCENT_START, ACCENT_END);
        self.accent.place().t(0).lr(0).h(4);

        self.eyebrow
            .set_text_color(ACCENT)
            .set_text_size(10)
            .set_alignment(TextAlignment::Left);
        self.eyebrow.place().l(16).t(14).r(16).h(16);

        self.title
            .set_text_color(TEXT)
            .set_text_size(20)
            .set_font(Font::get("RussoOne-Regular.ttf"))
            .set_alignment(TextAlignment::Left);
        self.title.place().l(16).t(31).r(16).h(28);
    }
}

#[view]
struct RecycledRow {
    #[init]
    accent: Container,
    title:  Label,
    detail: Label,
    index:  Label,
}

impl RecycledRow {
    fn set_index(self: Weak<Self>, index: usize) -> Weak<Self> {
        let (title, detail) = row_text(index, TABLE_TEXT_SEED);
        self.set_color(if index.is_multiple_of(2) {
            SURFACE
        } else {
            SURFACE_ALT
        });
        self.title.set_text(title);
        self.detail.set_text(detail);
        self.index.set_text(format!("#{:07}", index + 1));
        self
    }
}

impl Setup for RecycledRow {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(12).set_border_width(1).set_border_color(BORDER);

        self.accent.set_gradient(ACCENT_START, ACCENT_END).set_corner_radius(2);
        self.accent.place().l(0).tb(8).w(4);

        self.title
            .set_text_color(TEXT)
            .set_text_size(14)
            .set_alignment(TextAlignment::Left);
        self.title.place().l(16).t(6).r(116).h(22);

        self.detail
            .set_text_color(TEXT_DIM)
            .set_text_size(11)
            .set_alignment(TextAlignment::Left);
        self.detail.place().l(16).b(6).r(116).h(18);

        self.index
            .set_color(ACCENT)
            .set_text_color(WHITE)
            .set_text_size(11)
            .set_corner_radius(9);
        self.index.place().r(12).center_y().size(92, 30);
    }
}

/// A manual `ScrollView` with a tall stack of rows and a recycling
/// `TableView` driven by a data source. Side by side on wide screens,
/// stacked vertically on narrow ones.
#[view]
pub struct ScrollTables {
    #[init]
    scroll: ScrollView,
    table:  TableView,
}

impl Setup for ScrollTables {
    fn setup(mut self: Weak<Self>) {
        self.scroll
            .set_color(SURFACE_ALT)
            .set_border_width(1)
            .set_border_color(BORDER)
            .set_shadow(Shadow::default());
        self.fill_scroll();

        self.table
            .set_color(SURFACE_ALT)
            .set_border_width(1)
            .set_border_color(BORDER)
            .set_shadow(Shadow::default());
        self.table.set_data_source(self).register_cell::<RecycledRow>();
        self.table.set_cell_spacing(ROW_SPACING).set_header_height(PANEL_HEADER_HEIGHT);

        let header = self.table.add_header_view::<PanelHeader>();
        header.set_content("TABLEVIEW", "10 million messages");
        header.place().t(0).lr(0).h(PANEL_HEADER_HEIGHT);

        add_title(
            self,
            "Scrolling",
            "A plain ScrollView next to a TableView that recycles its rows.",
        );

        self.size_changed().sub(move || self.arrange());
        self.arrange();
    }
}

impl ScrollTables {
    /// Wide screens show the panels side by side, narrow ones stack them
    /// vertically so both stay usable.
    fn arrange(self: Weak<Self>) {
        let top = HEADER_HEIGHT + 4.0;
        if self.width() < 560.0 {
            self.scroll.place().clear().t(top).lr(12).relative_height(self, 0.42);
            self.table.place().clear().anchor(Anchor::Top, self.scroll, 12).lr(12).b(12);
        } else {
            self.scroll.place().clear().t(top).b(20).l(28).relative_width(self, 0.45);
            self.table.place().clear().t(top).b(20).r(28).relative_width(self, 0.45);
        }
    }

    fn fill_scroll(self: Weak<Self>) {
        let header = self.scroll.add_view::<PanelHeader>();
        header.set_content("SCROLLVIEW", "200 messages");
        header.place().t(0).lr(0).h(PANEL_HEADER_HEIGHT);

        for i in 0..SCROLL_ROWS {
            let (title_text, detail_text) = row_text(i, MANUAL_TEXT_SEED);
            let row = self.scroll.add_view::<Container>();
            row.set_color(SURFACE)
                .set_corner_radius(12)
                .set_border_width(1)
                .set_border_color(BORDER);
            row.place()
                .t(PANEL_HEADER_HEIGHT + 10.0 + (ROW_HEIGHT + ROW_SPACING) * i.lossy_convert())
                .lr(10)
                .h(ROW_HEIGHT);

            let index = row.add_view::<Label>();
            index
                .set_text(format!("{:03}", i + 1))
                .set_color(ACCENT)
                .set_text_color(WHITE)
                .set_text_size(11)
                .set_corner_radius(9);
            index.place().l(12).center_y().size(44, 30);

            let title = row.add_view::<Label>();
            title
                .set_text(title_text)
                .set_text_color(TEXT)
                .set_text_size(14)
                .set_alignment(TextAlignment::Left);
            title.place().l(68).t(6).r(12).h(22);

            let detail = row.add_view::<Label>();
            detail
                .set_text(detail_text)
                .set_text_color(TEXT_DIM)
                .set_text_size(11)
                .set_alignment(TextAlignment::Left);
            detail.place().l(68).b(6).r(12).h(18);
        }
    }
}

impl TableData for ScrollTables {
    fn number_of_cells(&self) -> usize {
        TABLE_ROWS
    }

    fn cell_height(&self, _: usize) -> f32 {
        ROW_HEIGHT
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        registry.cell::<RecycledRow>().set_index(index)
    }

    fn cell_selected(&mut self, _: usize) {}
}
