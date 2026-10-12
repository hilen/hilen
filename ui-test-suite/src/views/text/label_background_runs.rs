use anyhow::Result;
use hilen::{
    AppRunner,
    dispatch::from_main,
    gm::LossyConvert,
    refs::Weak,
    ui::{
        BLACK, Font, GREEN, Label, ORANGE, RED, Setup, TextAlignment, U8Color, UIColor, ViewData, ViewFrame,
        ViewTest, WHITE, YELLOW, view,
    },
    ui_test::check_colors,
};

const LINE: &str = "plain RED GREEN plain";
const WRAPPED: &str = "a color behind text that goes on over the end of a line";
const ROWS: [&str; 3] = ["first row ", "second row", "third row "];

/// The frame of the 3 rows that tile, in the pixels of the canvas.
const ROWS_X: f32 = 20.0;
const ROWS_Y: f32 = 330.0;

const RENDERED: &str = r"
             288   28 - #3c5465
             160   64 - #00ff00
              52   68 - #959595
             104   68 - #6b0000
             116   72 - #590000
             188   72 - #005900
             220   72 - #009b00
              40   80 - #dcdcdc
             576   92 - #ffffff
              68  124 - #476377
             300  164 - #ffff00
              44  176 - #ffffff
             224  180 - #cfcf00
             172  184 - #4a4a00
             124  212 - #222200
              40  244 - #a8a8a8
             296  308 - #000000
             100  332 - #ff0000
              44  344 - #000000
             168  356 - #ff0000
              64  392 - #005300
             212  408 - #00ff00
             120  424 - #b38e00
              48  436 - #ffcb00
             120  484 - #283843
             228  484 - #597c95
              48  524 - #c3c3c3
             592  592 - #597c95
";

/// A label paints a color behind byte ranges of its text, as high as
/// the line, under the glyphs. A range that wraps is painted on every
/// line it is on. Rows on a cell grid that are 1 cell high tile: their
/// colors meet with no line of the label color between them.
#[view]
struct LabelBackgroundRuns {
    #[init]
    line_title:    Label,
    line:          Label,
    wrapped_title: Label,
    wrapped:       Label,
    rows_title:    Label,
    row_one:       Label,
    row_two:       Label,
    row_three:     Label,
    cleared_title: Label,
    cleared:       Label,
}

impl LabelBackgroundRuns {
    fn rows(self: Weak<Self>) -> [Weak<Label>; 3] {
        [self.row_one, self.row_two, self.row_three]
    }
}

impl Setup for LabelBackgroundRuns {
    fn setup(self: Weak<Self>) {
        for (title, role, y) in [
            (self.line_title, "red behind RED, green behind GREEN", 16.0),
            (self.wrapped_title, "a range over 2 lines, yellow on both", 110.0),
            (self.rows_title, "3 rows on a cell grid, no gap between", 296.0),
            (self.cleared_title, "a new text has no colors behind it", 470.0),
        ] {
            title.set_frame((20.0, y, 560.0, 24.0));
            title.set_text(role).set_text_size(16).set_alignment(TextAlignment::Left);
        }

        for label in [self.line, self.wrapped, self.cleared] {
            label.set_text_size(26).set_alignment(TextAlignment::Left);
            label.set_color(WHITE);
            label.set_text_color(BLACK);
        }

        self.line.set_frame((20.0, 44.0, 560.0, 50.0));
        self.line.set_text(LINE);
        self.line
            .set_background_runs([(6..9, UIColor::from(RED)), (10..15, UIColor::from(GREEN))]);

        self.wrapped.set_frame((20.0, 138.0, 330.0, 140.0));
        self.wrapped.set_multiline(true);
        self.wrapped.set_text(WRAPPED);
        self.wrapped.set_background_runs([(8..41, UIColor::from(YELLOW))]);

        for ((label, text), color) in self.rows().into_iter().zip(ROWS).zip([RED, GREEN, ORANGE]) {
            label
                .set_text(text)
                .set_text_size(30)
                .set_font(Font::mono())
                .set_alignment(TextAlignment::Left)
                .set_cell_grid(true);
            label.set_color(WHITE);
            label.set_text_color(BLACK);
            label.set_background_runs([(0..text.len(), UIColor::from(color))]);
        }
        let cell = self.row_one.cell_size();
        let mut y = ROWS_Y;
        for label in self.rows() {
            label.set_frame((ROWS_X, y, 560.0, cell.height));
            y += cell.height;
        }

        self.cleared.set_frame((20.0, 498.0, 560.0, 50.0));
        self.cleared.set_text(LINE);
        self.cleared.set_background_runs([(0..LINE.len(), UIColor::from(RED))]);
    }
}

impl ViewTest for LabelBackgroundRuns {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let (column, top, bottom) = from_main(move || {
            assert_eq!(view.line.background_runs_len(), 2);

            view.cleared.set_text("a new text");
            assert_eq!(view.cleared.background_runs_len(), 0);

            // A column of pixels through the first cell of the 3 rows,
            // left of its glyph.
            let cell = view.row_one.cell_size();
            let column = ROWS_X + view.row_one.text_inset() + 1.0;
            (column, ROWS_Y, ROWS_Y + cell.height * 3.0)
        });

        let shot = AppRunner::take_screenshot()?;
        let mut seen: Vec<(u32, U8Color)> = Vec::new();
        let (column, top, bottom): (u32, u32, u32) = (
            column.lossy_convert(),
            top.lossy_convert(),
            bottom.lossy_convert(),
        );
        for y in top..bottom {
            let pixel = shot.get_pixel((column, y));
            if seen.last().is_none_or(|(_, last)| *last != pixel) {
                seen.push((y, pixel));
            }
        }
        assert!(
            seen.len() == 3,
            "the 3 rows show {} colors from top to bottom and not 3, a gap or a seam is between them: {seen:?}",
            seen.len()
        );

        check_colors(RENDERED)?;

        Ok(())
    }
}
