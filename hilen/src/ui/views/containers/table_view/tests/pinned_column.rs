use std::ops::Deref;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::{Color, WHITE},
    ui::{
        CellRegistry, Label, ModifiersState, Setup, TableData, TableView, TextAlignment, UIManager, View,
        ViewData, ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, inject_modifiers, inject_scroll, inject_scroll_x, inject_touches},
};

static SELECTED: Mutex<String> = Mutex::new(String::new());

const NUMBER_WIDTH: f32 = 50.0;
const CODE_WIDTH: f32 = 900.0;
const ROW: f32 = 50.0;
const TABLE_WIDTH: f32 = 400.0;

/// Cuts the moving text at the edge of the pinned number.
#[view]
struct CodeClip {}

impl Setup for CodeClip {
    fn clips_to_bounds(&self) -> bool {
        true
    }

    fn setup(self: Weak<Self>) {}
}

/// A line of code: the line number pinned on the left, the text moving
/// sideways behind it, and a mark pinned to the right edge of the table.
#[view]
struct CodeCell {
    code: Weak<Label>,

    #[init]
    number: Label,
    clip:   CodeClip,
    mark:   Label,
}

impl Setup for CodeCell {
    fn setup(mut self: Weak<Self>) {
        self.number
            .set_text_size(18)
            .set_color(Color::hex("#39424e"))
            .set_text_color(WHITE);
        self.number.place().l(0).t(0).b(0).w(NUMBER_WIDTH);

        self.clip.place().l(NUMBER_WIDTH).r(0).t(0).b(0);

        self.code = self.clip.add_view();
        self.code.set_text_size(18).set_alignment(TextAlignment::Left);
        self.code.set_moves_sideways(true);
        self.code.place().l(0).t(0).b(0).w(CODE_WIDTH);

        self.mark.set_text("pin").set_text_size(14);
        self.mark.set_color(Color::hex("#e8a33d")).set_corner_radius(6);
        self.mark.place().r(12).t(10).b(10).w(44);
        // A later sibling draws behind an earlier sibling's children, so
        // the mark is pushed in front of the text inside the clip.
        self.mark.bump_z_position(UIManager::subview_z_offset() * 2.0);
    }
}

impl CodeCell {
    fn set_row(self: Weak<Self>, index: usize, sticky: bool) {
        self.number.set_text(index);
        if sticky {
            self.code.set_text(format!(
                "section {index}, a sticky row, 200 300 400 500 600 700 800 end"
            ));
            self.code.set_color(Color::hex("#c9d6e3"));
        } else {
            self.code
                .set_text(format!("line {index}, at 100 200 300 400 500 600 700 800 end"));
            self.code.set_color(if index.is_multiple_of(2) {
                Color::hex("#f2f5f8")
            } else {
                Color::hex("#e2e8ee")
            });
        }
    }
}

// A table wider than its viewport with a pinned line number column. The
// sideways scroll moves only the text, clipped at the number. The number,
// the mark on the right edge, the sticky row and `index_at` stay as they
// are at every offset, and rows that scroll in arrive already moved.
#[view]
struct TablePinnedColumn {
    #[init]
    status: Label,
    table:  TableView,
}

impl Setup for TablePinnedColumn {
    fn setup(mut self: Weak<Self>) {
        self.status.set_text_size(20).set_color(WHITE);
        self.status.place().t(0).l(0).size(600, 60);

        self.table.place().t(80).l(0).size(TABLE_WIDTH, 500);
        self.table.set_data_source(self).register_cell::<CodeCell>();
        self.table.set_sticky_rows(true);
        self.table.set_content_width(NUMBER_WIDTH + CODE_WIDTH);
        self.table.reload_data();
    }
}

impl TableData for TablePinnedColumn {
    fn cell_height(&self, _: usize) -> f32 {
        ROW
    }

    fn number_of_cells(&self) -> usize {
        200
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<CodeCell>();
        cell.set_row(index, self.is_sticky(index));
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        *SELECTED.lock() += &format!("|{index}|");
    }

    fn is_sticky(&self, index: usize) -> bool {
        index.is_multiple_of(16)
    }
}

impl TablePinnedColumn {
    fn say(self: Weak<Self>, text: &'static str) {
        from_main(move || {
            self.status.set_text(text);
        });
        wait_for_next_frame();
    }

    /// Where the number, the text and the mark of row `index` sit on
    /// screen, by their left edges.
    fn row(self: Weak<Self>, index: usize) -> (f32, f32, f32) {
        from_main(move || {
            let cell = self
                .table
                .visible_cells()
                .into_iter()
                .find(|(tag, _)| *tag == index)
                .unwrap_or_else(|| panic!("row {index} is not on screen"))
                .1
                .downcast::<CodeCell>()
                .expect("a cell of another type");
            (
                cell.number.absolute_frame().origin.x,
                cell.code.absolute_frame().origin.x,
                cell.mark.absolute_frame().origin.x,
            )
        })
    }

    /// The bottom bar: hidden, its x and its length.
    fn bar(self: Weak<Self>) -> (bool, f32, f32) {
        from_main(move || {
            let bar = &self.table.scroll.subviews()[2];
            (bar.is_hidden(), bar.frame().origin.x, bar.frame().size.width)
        })
    }

    fn index_at(self: Weak<Self>, x: f32, y: f32) -> Option<usize> {
        from_main(move || self.table.index_at((x, y).into()))
    }
}

const MARK_X: f32 = TABLE_WIDTH - 12.0 - 44.0;

impl ViewTest for TablePinnedColumn {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        inject_touches("200 300 m");

        view.say("at the left edge, numbers pinned, bar at the left");
        assert_eq!(view.row(1), (0.0, NUMBER_WIDTH, MARK_X));
        assert_eq!(view.index_at(200.0, 75.0), Some(1));
        // 950 of content in 400: the bar is 400 / 950 of a 390 track,
        // which stops short of the vertical bar.
        let (hidden, bar_x, bar_length) = view.bar();
        assert!(!hidden, "no bottom bar over a table wider than its viewport");
        assert!((bar_x - 2.0).abs() < 0.001);
        assert!((bar_length - 390.0 * 400.0 / 950.0).abs() < 0.01);
        check_colors(CHECK_1)?;

        inject_scroll_x(-200);
        view.say("wheel sideways by 200, only the text moved");
        assert_eq!(view.row(1), (0.0, NUMBER_WIDTH - 200.0, MARK_X));
        assert_eq!(view.row(0), (0.0, NUMBER_WIDTH - 200.0, MARK_X));
        assert_eq!(view.index_at(200.0, 75.0), Some(1));
        check_colors(CHECK_2)?;

        // A tap picks the same row as before the scroll: row 3 is at
        // 150 inside the table, which starts 80 below the canvas top.
        inject_touches(
            "
            200 255 b
            200 255 e
        ",
        );
        assert_eq!(SELECTED.lock().deref(), "|3|");
        SELECTED.lock().clear();

        inject_modifiers(ModifiersState::SHIFT);
        inject_scroll(-100);
        inject_modifiers(ModifiersState::empty());
        view.say("Shift plus wheel, 100 more to the left, rows did not move");
        assert_eq!(view.row(1), (0.0, NUMBER_WIDTH - 300.0, MARK_X));
        assert!(from_main(move || view.table.content_offset()).abs() < 0.001);
        check_colors(CHECK_3)?;

        // 20 rows down. Row 16 is sticky and pins to the top, rows 20 to
        // 29 are new on screen. All of them carry the sideways offset.
        inject_scroll(-1000);
        view.say("scrolled down 20 rows, section 16 pinned on top, text still moved");
        assert_eq!(view.row(16), (0.0, NUMBER_WIDTH - 300.0, MARK_X));
        assert_eq!(view.row(25), (0.0, NUMBER_WIDTH - 300.0, MARK_X));
        assert_eq!(view.index_at(200.0, 75.0), Some(21));
        check_colors(CHECK_4)?;

        from_main(move || {
            view.table.set_content_offset_x(-10_000);
        });
        view.say("set far past the end, clamped, the text ends at the right edge");
        assert!((from_main(move || view.table.content_offset_x()) + 550.0).abs() < 0.001);
        assert_eq!(view.row(25), (0.0, NUMBER_WIDTH - 550.0, MARK_X));
        check_colors(CHECK_5)?;

        from_main(move || {
            view.table.set_content_offset_x(0);
        });
        view.say("set back to 0, the text starts at the number again");
        assert_eq!(view.row(25), (0.0, NUMBER_WIDTH, MARK_X));
        check_colors(CHECK_6)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
         592    4 - #ffffff
         320   28 - #787878
         120   32 - #ffffff
         300   32 - #b7b7b7
         448   32 - #dfdfdf
         320   36 - #787878
          24  104 - #39424e
         136  104 - #858e96
         368  104 - #9aa4ae
         200  108 - #c9d6e3
         280  148 - #5b5e60
         360  156 - #e8a33d
         224  204 - #282829
          76  208 - #0a0b0b
         384  252 - #e8a33d
         224  304 - #282829
         304  304 - #010101
         368  304 - #b17d2f
          24  308 - #666d76
         152  336 - #e2e8ee
         264  404 - #b7b9bc
         368  404 - #b17d2f
           4  448 - #39424e
         160  456 - #e2e8ee
         232  504 - #f2f5f8
         328  504 - #a0a2a4
          24  508 - #39424e
         104  552 - #e2e8ee
         360  556 - #e8a33d
           4  576 - #252b33
         160  576 - #93979b
         592  592 - #597c95
";

const CHECK_2: &str = r"
         592    4 - #ffffff
         188   24 - #262626
         140   32 - #f9f9f9
         440   32 - #dcdcdc
         284   36 - #343434
          24  104 - #39424e
         108  104 - #c9d6e3
         176  104 - #484c51
         244  104 - #7b838b
         340  112 - #c9d6e3
          24  156 - #787e86
         128  204 - #a0a2a4
         368  204 - #b17d2f
         240  208 - #f2f5f8
         368  256 - #b17d2f
         344  300 - #e8a33d
         128  304 - #a0a2a4
          24  308 - #666d76
         232  308 - #151515
         136  360 - #030303
         396  380 - #f2f5f8
         592  384 - #597c95
         208  404 - #58595a
           4  416 - #39424e
         136  452 - #e2e8ee
         360  456 - #e8a33d
         224  468 - #e2e8ee
          64  504 - #b7b9bc
         360  556 - #e8a33d
         164  576 - #93979b
         264  576 - #e2e8ee
         568  592 - #597c95
";

const CHECK_3: &str = r"
         156   24 - #adadad
         348   24 - #6c6c6c
         500   28 - #c2c2c2
          88   32 - #525252
         208   32 - #959595
         272   32 - #eaeaea
         328   32 - #2c2c2c
         500   32 - #c2c2c2
         524   32 - #ffffff
          64  104 - #a3aeb8
         116  104 - #35383c
         384  116 - #e8a33d
          24  152 - #787e86
         264  180 - #f2f5f8
         140  204 - #f2f5f8
         592  216 - #597c95
          60  252 - #e2e8ee
         368  256 - #b17d2f
         232  276 - #e2e8ee
         132  308 - #151515
          24  348 - #afb3b8
         368  356 - #b17d2f
          68  404 - #3e3e3f
         276  404 - #f2f5f8
         592  412 - #597c95
           4  452 - #39424e
         136  456 - #010101
         360  456 - #e8a33d
          52  548 - #5b5e60
         360  556 - #e8a33d
         204  572 - #e2e8ee
         284  576 - #93979b
";

const CHECK_4: &str = r"
          36   28 - #2e2e2e
         128   28 - #7a7a7a
         256   32 - #393939
         344   32 - #5f5f5f
         404   32 - #c5c5c5
         452   32 - #b8b8b8
         580   32 - #ffffff
          56  104 - #9aa4ae
         156  104 - #848c95
         380  140 - #e8a33d
         132  156 - #161717
         260  184 - #f2f5f8
          84  204 - #000000
           4  224 - #39424e
         148  256 - #e2e8ee
         368  256 - #b17d2f
         108  304 - #58595a
         592  324 - #597c95
          28  356 - #39424e
         148  356 - #e2e8ee
         360  356 - #e8a33d
         248  380 - #f2f5f8
         132  456 - #161717
         360  456 - #e8a33d
          20  460 - #ffffff
         228  476 - #e2e8ee
          28  552 - #39424e
         368  556 - #b17d2f
          96  560 - #010101
         252  572 - #e2e8ee
         172  576 - #93979b
         592  592 - #597c95
";

const CHECK_5: &str = r"
           4    4 - #ffffff
         204   24 - #0c0c0c
         404   24 - #bfbfbf
         500   24 - #c7c7c7
         540   24 - #898989
         264   28 - #ffffff
         552   28 - #000000
         132   32 - #5f5f5f
         144   32 - #dfdfdf
         324   32 - #f9f9f9
         384   32 - #c7c7c7
         452   32 - #000000
         512   32 - #6c6c6c
         368  104 - #b17d2f
           4  140 - #39424e
         124  144 - #e2e8ee
         348  168 - #e8a33d
          28  204 - #39424e
         200  232 - #e2e8ee
         572  232 - #597c95
          32  304 - #f0f1f2
         368  304 - #b17d2f
           4  376 - #39424e
         372  408 - #e8a33d
         200  424 - #f2f5f8
         592  436 - #597c95
          32  448 - #afb3b8
         384  492 - #e8a33d
          28  552 - #39424e
         360  556 - #e8a33d
         232  576 - #93979b
         296  576 - #93979b
";

const CHECK_6: &str = r"
           4    4 - #ffffff
         404   28 - #d6d6d6
         504   28 - #e0e0e0
         108   32 - #6c6c6c
         260   32 - #ffffff
          24   80 - #39424e
          96  104 - #181a1c
         184  104 - #70777e
         340  104 - #17181a
         332  108 - #9ba5af
         396  140 - #93979b
           4  152 - #39424e
         228  204 - #000000
         384  216 - #e8a33d
         128  232 - #e2e8ee
          28  256 - #39424e
         592  272 - #597c95
         304  304 - #010101
         372  308 - #e8a33d
         156  336 - #e2e8ee
          28  356 - #39424e
         368  404 - #b17d2f
          32  448 - #afb3b8
         292  448 - #5b5e60
         204  480 - #f2f5f8
         264  504 - #b7b9bc
         368  504 - #b17d2f
         360  556 - #e8a33d
          20  560 - #ffffff
          48  576 - #252b33
         136  576 - #93979b
         592  592 - #597c95
";
