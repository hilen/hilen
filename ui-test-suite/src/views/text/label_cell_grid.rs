use std::ops::Range;

use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    ui::{
        BLACK, Font, GRAY, LIGHT_GRAY, Label, RunStyle, Setup, TextAlignment, UIColor, UIManager, ViewData,
        ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

const TEXT_SIZE: f32 = 28.0;

const DIGITS: &str = "0123456789|";
/// 2 Japanese letters, each 2 cells wide, drawn by another font.
const WIDE: &str = "ab漢字cd|";
const WIDE_RUN: Range<usize> = 2..8;
/// 3 narrow letters of a font with no fixed width, between letters of
/// the mono font.
const OTHER_FONT: &str = "ailia|aa";
const OTHER_RUN: Range<usize> = 1..4;
/// An `e` with a mark that joins it, 2 chars in 1 cell.
const MARK: &str = "ae\u{301}b|";

const RENDERED: &str = r"
             320    4 - #597c95
             592    4 - #597c95
             148   24 - #000000
              40   28 - #597c95
              92   28 - #000000
             132   28 - #40596b
             204   28 - #000000
              60   44 - #bcbcbc
             100   44 - #bcbcbc
             124   44 - #bcbcbc
             144   44 - #e7e7e7
             192   44 - #bcbcbc
             220   48 - #e7e7e7
              36   52 - #e7e7e7
              80   52 - #e7e7e7
             164   56 - #bcbcbc
             508   56 - #ffffff
             116   60 - #555555
             180   60 - #e7e7e7
             204   60 - #bcbcbc
              48   64 - #4d4d4d
             112   68 - #343434
             148   68 - #e7e7e7
             412   68 - #ffffff
              68   72 - #bcbcbc
              88   72 - #bcbcbc
             192   72 - #bcbcbc
             104   76 - #e7e7e7
             132   76 - #bcbcbc
              40   80 - #e7e7e7
             160   80 - #bcbcbc
             220   80 - #e7e7e7
             576   80 - #ffffff
             320  132 - #597c95
              72  136 - #2e404d
              88  136 - #151d22
             204  136 - #1d2830
              40  140 - #597c95
              88  140 - #151d22
             540  148 - #597c95
             156  156 - #bcbcbc
              76  160 - #e7e7e7
             128  160 - #bcbcbc
              56  164 - #000000
             168  164 - #000000
              36  168 - #e7e7e7
             112  168 - #303030
              92  172 - #000000
             180  172 - #000000
             436  172 - #ffffff
              56  176 - #000000
             128  176 - #000000
              80  180 - #505050
             168  180 - #000000
              44  188 - #e7e7e7
              64  188 - #bcbcbc
             100  188 - #e7e7e7
             148  188 - #e7e7e7
             188  188 - #e7e7e7
             324  244 - #374d5c
              64  248 - #000000
              96  248 - #597c95
             140  248 - #263540
             156  248 - #374c5c
             192  248 - #374d5c
             224  248 - #49657a
             248  248 - #0a0f12
             268  248 - #2d3e4b
             276  248 - #2d3e4b
             284  248 - #597c95
             176  252 - #597c95
             364  252 - #597c95
              92  264 - #bcbcbc
              36  268 - #e7e7e7
             112  268 - #e7e7e7
             156  272 - #bcbcbc
              72  276 - #2b2b2b
              76  276 - #959595
             128  276 - #484848
              56  280 - #010101
              72  280 - #2b2b2b
              76  280 - #959595
              92  280 - #010101
             128  280 - #484848
             476  280 - #ffffff
              76  284 - #959595
             128  284 - #484848
              72  288 - #2b2b2b
              76  288 - #959595
             128  288 - #484848
             164  288 - #bcbcbc
             592  288 - #597c95
             128  292 - #484848
             128  296 - #484848
              40  300 - #e7e7e7
              60  300 - #bcbcbc
             100  300 - #bcbcbc
             148  300 - #e7e7e7
             368  352 - #597c95
              72  356 - #2d3e4b
              60  360 - #11181d
             116  360 - #000001
             148  360 - #324553
             152  360 - #12191e
             180  360 - #597c95
             216  360 - #000000
             268  360 - #12191e
              36  376 - #e7e7e7
              96  376 - #bcbcbc
              56  380 - #bcbcbc
              72  384 - #5b5b5b
              72  388 - #5b5b5b
              96  388 - #6d6d6d
              96  392 - #6d6d6d
              60  396 - #343434
              72  396 - #5b5b5b
              96  396 - #6d6d6d
             460  396 - #ffffff
              72  400 - #5b5b5b
              96  400 - #6d6d6d
              96  404 - #6d6d6d
             576  404 - #ffffff
              36  408 - #e7e7e7
             100  464 - #0e1317
             292  464 - #151d22
              64  468 - #020304
             120  468 - #3e5769
             152  468 - #597c95
             196  468 - #141b21
             208  468 - #597c95
             260  468 - #000000
             268  468 - #2d3e4b
             292  468 - #151d22
             304  468 - #1c272f
              40  472 - #597c95
             232  472 - #597c95
             496  484 - #ffffff
             408  488 - #ffffff
             104  496 - #616161
              56  500 - #010101
              72  500 - #171717
             104  500 - #616161
              72  504 - #171717
              88  504 - #6e6e6e
             104  504 - #616161
              44  508 - #ffffff
              72  508 - #171717
              84  508 - #ffffff
              88  508 - #ffffff
             104  508 - #616161
             120  508 - #ffffff
             136  508 - #ffffff
              44  512 - #010101
              80  512 - #ffffff
             104  516 - #616161
             576  520 - #ffffff
             328  564 - #597c95
             236  592 - #597c95
             420  592 - #597c95
             516  592 - #597c95
";

/// A label on a cell grid puts every char into its own cell, like a
/// terminal does. A wide letter takes 2 cells, a mark none, and a glyph
/// of another font moves nothing after it out of its column. The gray
/// stripes are the cells, the last row is the text of the third row
/// with no grid.
#[view]
struct LabelCellGrid {
    #[init]
    digits_title: Label,
    digits:       Label,
    wide_title:   Label,
    wide:         Label,
    other_title:  Label,
    other:        Label,
    mark_title:   Label,
    mark:         Label,
    flow_title:   Label,
    flow:         Label,
}

impl LabelCellGrid {
    /// Every second cell of a grid label in gray, so the cells show.
    fn stripe(label: Weak<Label>, cells: &[Range<usize>]) {
        label.set_background_runs(cells.iter().enumerate().map(|(index, cell)| {
            let color = if index % 2 == 0 { LIGHT_GRAY } else { GRAY };
            (cell.clone(), UIColor::from(color))
        }));
    }

    /// The byte range of every char of `text`. A mark goes with the char
    /// before it.
    fn cells(text: &str) -> Vec<Range<usize>> {
        let mut cells: Vec<Range<usize>> = Vec::new();
        for (byte, char) in text.char_indices() {
            let end = byte + char.len_utf8();
            match cells.last_mut() {
                Some(last) if char == '\u{301}' => last.end = end,
                _ => cells.push(byte..end),
            }
        }
        cells
    }
}

impl Setup for LabelCellGrid {
    fn setup(self: Weak<Self>) {
        let rows = [
            (
                self.digits_title,
                self.digits,
                "the mono font, 1 cell per char",
                DIGITS,
            ),
            (self.wide_title, self.wide, "a wide letter takes 2 cells", WIDE),
            (
                self.other_title,
                self.other,
                "the narrow i, l, i of another font keep 1 cell each",
                OTHER_FONT,
            ),
            (
                self.mark_title,
                self.mark,
                "a mark shares the cell of its letter",
                MARK,
            ),
            (
                self.flow_title,
                self.flow,
                "no grid: the narrow i, l, i pull the rest left",
                OTHER_FONT,
            ),
        ];

        let mut y = 16.0;
        for (title, label, role, text) in rows {
            title.set_frame((20.0, y, 560.0, 24.0));
            title.set_text(role).set_text_size(16).set_alignment(TextAlignment::Left);
            label
                .set_text(text)
                .set_text_size(TEXT_SIZE)
                .set_font(Font::mono())
                .set_alignment(TextAlignment::Left);
            label.set_color(WHITE);
            label.set_text_color(BLACK);
            label.set_frame((20.0, y + 28.0, 560.0, label.cell_size().height));
            y += 110.0;
        }

        for label in [self.digits, self.wide, self.other, self.mark] {
            label.set_cell_grid(true);
            label.set_frame((20.0, label.frame().y(), 560.0, label.cell_size().height));
        }

        let japanese = RunStyle::font(Font::get("DotGothic16-Regular.ttf"));
        self.wide.set_font_runs([(WIDE_RUN, japanese)]);
        for label in [self.other, self.flow] {
            label.set_font_runs([(OTHER_RUN, RunStyle::font(Font::get("OpenSans.ttf")))]);
        }

        Self::stripe(self.digits, &Self::cells(DIGITS));
        Self::stripe(self.wide, &Self::cells(WIDE));
        Self::stripe(self.other, &Self::cells(OTHER_FONT));
        Self::stripe(self.mark, &Self::cells(MARK));
    }
}

/// Where the char at `byte` of the text of a label starts, in points
/// from the start of the text.
fn x_of(label: Weak<Label>, byte: usize) -> f32 {
    label.text_layout_for(label.text()).x_on_line(0, byte)
}

fn on_cell(label: Weak<Label>, byte: usize, cell: usize) {
    let cells: f32 = cell.lossy_convert();
    let wanted = label.cell_size().width * cells;
    let got = x_of(label, byte);
    assert!(
        (got - wanted).abs() < 0.01,
        "byte {byte} of {:?} starts at {got}, cell {cell} starts at {wanted}",
        label.text()
    );
}

impl ViewTest for LabelCellGrid {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);

        from_main(move || {
            let cell = view.digits.cell_size();
            let scale = UIManager::scale();
            for side in [cell.width, cell.height] {
                assert!(
                    side > 0.0 && (side * scale).fract().abs() < 0.001,
                    "a cell side of {side} points is no whole number of pixels at scale {scale}"
                );
            }

            for byte in 0..=DIGITS.len() {
                on_cell(view.digits, byte, byte);
            }
            // a b 漢 字 c d |
            for (byte, cell) in [(0, 0), (1, 1), (2, 2), (5, 4), (8, 6), (9, 7), (10, 8), (11, 9)] {
                on_cell(view.wide, byte, cell);
            }
            for byte in 0..=OTHER_FONT.len() {
                on_cell(view.other, byte, byte);
            }
            // a e+mark b |
            for (byte, cell) in [(0, 0), (1, 1), (4, 2), (5, 3), (6, 4)] {
                on_cell(view.mark, byte, cell);
            }

            let flowed = x_of(view.flow, 5);
            let grid = x_of(view.other, 5);
            assert!(
                (flowed - grid).abs() > 2.0,
                "the text with no grid is laid out like the one on the grid, {flowed} and {grid}"
            );
        });

        check_colors(RENDERED)?;

        Ok(())
    }
}
