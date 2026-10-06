use anyhow::Result;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{
        ToF32,
        color::{BLACK, Color, WHITE},
    },
    ui::{
        Container, Label, ModifiersState, ScrollView, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest,
        view,
    },
    ui_test::{checkpoint, inject_modifiers, inject_scroll, inject_scroll_x, inject_touches},
};

const COLUMNS: usize = 8;
const TILE_WIDTH: f32 = 150.0;
const ROW_HEIGHT: f32 = 120.0;
const GRID_ROWS: usize = 3;
const GRID_TILE_HEIGHT: f32 = 200.0;

/// A plain mouse wheel sends only a vertical delta. The row on top is as
/// high as its content and wider than its view, so it can move only
/// sideways, and the plain wheel moves it sideways. The grid under it is
/// higher and wider than its view, so the same wheel moves it up and
/// down, and only Shift sends it sideways.
///
/// A pointer move draws nothing on its own, so a black square marks the
/// cursor.
#[view]
struct SidewaysWheel {
    pointer: Weak<Container>,

    #[init]
    status:     Label,
    row_title:  Label,
    row:        ScrollView,
    grid_title: Label,
    grid:       ScrollView,
}

impl Setup for SidewaysWheel {
    fn setup(mut self: Weak<Self>) {
        self.status.set_text_size(20).set_color(WHITE);
        self.status.place().tl(0).size(600, 60);

        self.row_title
            .set_text("row, as high as its content, can move only sideways")
            .set_text_size(16)
            .set_color(WHITE);
        self.row_title.place().t(70).l(50).size(500, 26);

        // The height stays automatic, it follows the tiles, so the row
        // has no vertical range.
        self.row.set_color(WHITE);
        self.row.set_content_width(TILE_WIDTH * COLUMNS.to_f32());
        self.row.place().t(100).l(50).size(500, ROW_HEIGHT);
        Self::add_tiles(self.row, 1, ROW_HEIGHT);

        self.grid_title
            .set_text("grid, higher and wider than its view, moves both ways")
            .set_text_size(16)
            .set_color(WHITE);
        self.grid_title.place().t(240).l(50).size(500, 26);

        self.grid.set_color(WHITE);
        self.grid.set_content_size((
            TILE_WIDTH * COLUMNS.to_f32(),
            GRID_TILE_HEIGHT * GRID_ROWS.to_f32(),
        ));
        self.grid.place().t(270).l(50).size(500, 300);
        Self::add_tiles(self.grid, GRID_ROWS, GRID_TILE_HEIGHT);

        let mut pointer = Container::new();
        pointer.set_z_position(0.1);
        pointer
            .set_color(BLACK)
            .set_border_width(3)
            .set_border_color(WHITE)
            .set_size(20, 20);
        pointer.set_hidden(true);
        self.pointer = self.add_subview(pointer);
    }
}

impl SidewaysWheel {
    fn add_tiles(scroll: Weak<ScrollView>, rows: usize, height: f32) {
        for row in 0..rows {
            for column in 0..COLUMNS {
                let tile = scroll.add_view::<Label>();
                tile.set_text(format!("{row} {column}")).set_text_size(40);
                tile.set_color(if (row + column).is_multiple_of(2) {
                    Color::hex("#dfe8f0")
                } else {
                    Color::hex("#a9bdd0")
                });
                tile.set_frame((
                    column.to_f32() * TILE_WIDTH,
                    row.to_f32() * height,
                    TILE_WIDTH,
                    height,
                ));
            }
        }
    }

    fn point_at(self: Weak<Self>, x: u32, y: u32) {
        inject_touches(format!("{x} {y} m"));
        from_main(move || {
            let mut pointer = self.pointer;
            pointer.set_hidden(false);
            pointer.set_center((x, y));
        });
    }

    /// Shows what just happened and holds there for a human.
    fn step(self: Weak<Self>, text: &'static str) -> Result<()> {
        from_main(move || {
            self.status.set_text(text);
        });
        wait_for_next_frame();
        checkpoint(text)
    }

    /// The sideways and the vertical offset of the row, then of the grid.
    fn offsets(self: Weak<Self>) -> [f32; 4] {
        from_main(move || {
            [
                self.row.content_offset_x(),
                self.row.get_scroll_content_offset(),
                self.grid.content_offset_x(),
                self.grid.get_scroll_content_offset(),
            ]
        })
    }

    /// Where the first tile of the row is drawn.
    fn first_row_tile_x(self: Weak<Self>) -> f32 {
        from_main(move || self.row.content.subviews()[0].absolute_frame().origin.x)
    }
}

impl ViewTest for SidewaysWheel {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        let (content, height) = from_main(move || (view.row.content_height(), view.row.height()));
        assert!(
            (content - height).abs() < f32::EPSILON,
            "the row has a vertical range"
        );

        view.point_at(300, 125);
        view.step("pointer on the row, nothing moved yet")?;
        assert_eq!(view.offsets(), [0.0, 0.0, 0.0, 0.0]);

        inject_scroll(-100);
        view.step("plain wheel by 100 on the row, the row moved left")?;
        assert_eq!(view.offsets(), [-100.0, 0.0, 0.0, 0.0]);
        assert!((view.first_row_tile_x() - (50.0 - 100.0)).abs() < 0.001);

        inject_scroll(-5000);
        view.step("plain wheel far past the end, the row stopped at its right edge")?;
        assert_eq!(view.offsets(), [-700.0, 0.0, 0.0, 0.0]);

        inject_scroll(100);
        view.step("plain wheel the other way by 100, the row moved right")?;
        assert_eq!(view.offsets(), [-600.0, 0.0, 0.0, 0.0]);

        inject_modifiers(ModifiersState::SHIFT);
        inject_scroll(100);
        inject_modifiers(ModifiersState::empty());
        view.step("Shift plus wheel by 100 on the row, right again")?;
        assert_eq!(view.offsets(), [-500.0, 0.0, 0.0, 0.0]);

        inject_scroll_x(200);
        view.step("sideways wheel by 200 on the row, right again")?;
        assert_eq!(view.offsets(), [-300.0, 0.0, 0.0, 0.0]);

        view.point_at(300, 420);
        inject_scroll(-100);
        view.step("plain wheel by 100 on the grid, it moved up only")?;
        assert_eq!(view.offsets(), [-300.0, 0.0, 0.0, -100.0]);

        inject_modifiers(ModifiersState::SHIFT);
        inject_scroll(-100);
        inject_modifiers(ModifiersState::empty());
        view.step("Shift plus wheel by 100 on the grid, the grid moved left")?;
        assert_eq!(view.offsets(), [-300.0, 0.0, -100.0, -100.0]);

        Ok(())
    }
}
