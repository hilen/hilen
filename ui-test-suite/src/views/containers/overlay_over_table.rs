use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, CellRegistry, Color, Container, Label, Setup, TableData, TableView, View, ViewData,
        ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, checkpoint, inject_trackpad_scroll},
};

const ROW_H: f32 = 60.0;
const STICKY_H: f32 = 60.0;

const UNDER: &str = r"
             420    4 - #1d3557
             548    4 - #1d3557
              52   28 - #fffcfc
             168   28 - #ec656f
             176   28 - #e63946
              68   32 - #e63946
              84   32 - #e63946
             136   32 - #f2959c
             168   32 - #ec656f
             176   32 - #f2959c
             168   36 - #ec656f
              12   48 - #e63946
             228   48 - #e63946
             320   84 - #86adb0
             280   92 - #a8dadc
             300  144 - #2d3438
             304  204 - #20262c
             100  240 - #f1faee
             592  240 - #f1faee
             292  268 - #f1faee
             300  324 - #293237
             276  388 - #f1faee
             300  392 - #d4ddd3
               4  420 - #a8dadc
             300  444 - #95c1c4
             300  508 - #d4ddd3
             324  516 - #4a5154
             592  544 - #a8dadc
             300  564 - #95c1c4
             300  568 - #95c1c4
             276  572 - #a8dadc
             324  572 - #a8dadc
            ";

const COVERED: &str = r"
             244    4 - #04080d
             324    4 - #04080d
             424    4 - #04080d
             592    4 - #04080d
              52   28 - #262626
             172   32 - #262626
             508   56 - #04080d
             280   92 - #192021
             280  152 - #242523
             304  208 - #050607
             216  224 - #f4a261
             124  228 - #f4a261
             476  228 - #f4a261
             264  300 - #f4a261
             292  300 - #d08a53
             292  304 - #f4a261
             296  304 - #f4a261
             304  304 - #ce8952
             324  304 - #f4a261
             332  304 - #f4a261
             292  308 - #000000
             148  376 - #f4a261
             464  376 - #f4a261
             324  388 - #242523
               8  392 - #242523
             272  452 - #050607
             592  460 - #192021
             312  508 - #050607
             156  556 - #192021
             312  572 - #050607
               4  592 - #192021
             484  592 - #192021
            ";

/// A screen with an overlay as its last child, like the mark of a file
/// held over the window. The overlay is a sibling of the table, the cells
/// and the stuck header sit 4 levels deeper and the header is raised over
/// the cells. The overlay is still in front of all of it, with no raise of
/// its own.
#[view]
struct OverlayOverTable {
    sticky: Weak<Container>,

    #[init]
    table:   TableView,
    overlay: Container,
    card:    Container,
    text:    Label,
}

impl Setup for OverlayOverTable {
    fn setup(mut self: Weak<Self>) {
        self.table.place().back();
        self.table
            .set_data_source(self)
            .register_cell::<Label>()
            .set_header_height(STICKY_H);

        self.sticky = self.table.add_sticky_header_view::<Container>();
        self.sticky.set_color(Color::hex("#1d3557"));
        self.table.set_sticky_header_place(0.0, STICKY_H);
        let label = self.sticky.add_view::<Label>();
        label
            .set_text("sticky header")
            .set_text_size(24)
            .set_text_color(WHITE)
            .set_color(Color::hex("#e63946"));
        label.place().l(10).center_y().size(220, 40);

        // The overlay holds the card, the card the text, 3 views that
        // come after the table.
        self.overlay.set_color(BLACK.with_alpha(0.85)).set_hidden(true);
        self.overlay.place().back();
        self.card = self.overlay.add_view::<Container>();
        self.card.set_color(Color::hex("#f4a261")).set_corner_radius(16);
        self.card.place().center().size(360, 160);
        self.text = self.card.add_view::<Label>();
        self.text.set_text("overlay").set_text_size(28).set_text_color(BLACK);
        self.text.place().center().size(300, 50);

        self.table.reload_data();
    }
}

impl TableData for OverlayOverTable {
    fn cell_height(&self, _: usize) -> f32 {
        ROW_H
    }

    fn number_of_cells(&self) -> usize {
        40
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_text(format!("cell {index}"));
        cell.set_text_size(20);
        cell.set_text_color(Color::hex("#20262c"));
        cell.set_color(if index.is_multiple_of(2) {
            Color::hex("#f1faee")
        } else {
            Color::hex("#a8dadc")
        });
        cell
    }
}

impl ViewTest for OverlayOverTable {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Scrolled, so the header is stuck at the top over the cells.
        inject_trackpad_scroll(-300);
        let offset = from_main(move || view.table.content_offset());
        ensure!(
            offset < -200.0,
            "the table did not scroll, its offset is {offset}"
        );
        checkpoint("the sticky header stuck over the cells")?;
        check_colors(UNDER)?;

        from_main(move || {
            view.overlay.set_hidden(false);
        });
        checkpoint("the dark overlay covers the header and the cells, the card is on top")?;
        check_colors(COVERED)?;

        Ok(())
    }
}
