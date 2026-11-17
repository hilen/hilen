use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Container, Label, MarkdownBoxTables, MarkdownStyle, MarkdownView, Setup, TextAlignment,
        VerticalAlignment, ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

/// A table drawn with box chars in a code block, the way a chat writes
/// one, in a column as narrow as a phone. Until 2026-11-16 each line of
/// it wrapped on a phone and the table fell apart. With
/// `MarkdownBoxTables::Table` it is a real table, its cells wrap inside
/// their columns and a word wider than its cell breaks.
const TEXT: &str = "\
The steps:

```
╭──────┬───────┬───────────┬───────┬──────────────────╮
│ Step │ Name  │ Leibniz   │ Unit  │ Also             │
├──────┼───────┼───────────┼───────┼──────────────────┤
│ 1    │ speed │ dx/dt     │ m/s   │ rate             │
├──────┼───────┼───────────┼───────┼──────────────────┤
│ 3    │ jerk  │ d^3x/dt^3 │ m/s^3 │ third_derivative │
├──────┼───────┼───────────┼───────┼──────────────────┤
│ 4    │ snap  │ d^4x/dt^4 │ m/s^4 │ jounce           │
╰──────┴───────┴───────────┴───────┴──────────────────╯
```
";

const HEAD: [&str; 5] = ["Step", "Name", "Leibniz", "Unit", "Also"];
/// A word with no place to wrap, wider than its column, it breaks inside
/// its cell.
const LONG: &str = "third_derivative";
/// Short words of the table, each one fits its column whole.
const WHOLE: [&str; 5] = ["Step", "Name", "speed", "snap", "jounce"];

const LEFT: f32 = 16.0;
const TOP: f32 = 40.0;
const WIDTH: f32 = 300.0;

const STYLE: MarkdownStyle = MarkdownStyle {
    keeps_line_breaks: true,
    box_tables: MarkdownBoxTables::Table,
    ..MarkdownStyle::DEFAULT
};

/// Recorded with `--record-colors`.
const COLORS: &str = r"
             396    4 - #ffffff
             176   16 - #b1b1b1
             216   16 - #323232
             296   16 - #212121
             320   16 - #b1b1b1
              52   20 - #ffffff
              68   20 - #000000
              80   20 - #ffffff
             100   20 - #545454
             140   20 - #ffffff
             164   20 - #ffffff
             176   20 - #b1b1b1
             184   20 - #000000
             284   20 - #0b0b0b
             296   20 - #212121
             320   20 - #b1b1b1
              36   48 - #27282b
              68   48 - #1c1d1f
              32   72 - #dce0e6
             148   76 - #848688
              48   80 - #000000
              80   80 - #000000
             148   80 - #848688
             156   80 - #636466
             164   80 - #6e7071
             188   80 - #000000
             196   80 - #d9dbdf
             204   80 - #e3e6e9
             236   80 - #000000
             412   80 - #ffffff
              60   84 - #000000
             112   84 - #000000
             132   84 - #000000
             140   84 - #010101
             152   84 - #000000
             308   84 - #e8f0fe
              32   92 - #dce0e6
             156  108 - #757980
              32  112 - #dce0e6
             112  112 - #e8f0fe
             132  112 - #e8f0fe
             160  112 - #bfc6d1
             188  112 - #000000
             236  112 - #d4dbe8
              32  124 - #dce0e6
              32  132 - #dce0e6
             356  136 - #ffffff
              32  140 - #dce0e6
              96  140 - #3e4044
             252  140 - #bbc1cc
              80  144 - #000000
             132  144 - #e8f0fe
             188  144 - #000000
             192  144 - #757980
             228  144 - #e8f0fe
             236  144 - #90959d
             252  144 - #bbc1cc
             272  144 - #dbe3f0
             280  144 - #9ba0a9
              32  148 - #dce0e6
             256  148 - #83888f
              32  160 - #dce0e6
             132  160 - #e8f0fe
             236  160 - #e8f0fe
             252  160 - #7e838a
              32  168 - #dce0e6
              32  180 - #dce0e6
              44  184 - #c2c8d4
              32  188 - #dce0e6
              92  188 - #050505
             100  188 - #606369
             132  188 - #e8f0fe
             156  188 - #e8f0fe
             188  188 - #000000
             196  188 - #e8f0fe
             232  188 - #e8f0fe
             256  188 - #e8f0fe
             264  188 - #222325
             144  192 - #e8f0fe
             412  192 - #ffffff
              32  196 - #dce0e6
             344  200 - #ffffff
              32  204 - #dce0e6
             132  204 - #e8f0fe
              32  212 - #dee2e9
             264  256 - #ffffff
             404  260 - #ffffff
             332  264 - #ffffff
             200  272 - #ffffff
              60  284 - #ffffff
             128  316 - #ffffff
               4  332 - #ffffff
             188  332 - #ffffff
             256  332 - #ffffff
             340  332 - #ffffff
             412  332 - #ffffff
            ";

/// Puts the default style back when it goes away, also on a panic. The
/// style is global, a test that left its own behind would draw every
/// markdown view after it with it.
struct DefaultStyleBack;

impl Drop for DefaultStyleBack {
    fn drop(&mut self) {
        MarkdownStyle::DEFAULT.apply_globally();
    }
}

#[view]
struct MarkdownBoxTable {
    height: f32,

    #[init]
    caption:    Label,
    background: Container,
    text:       MarkdownView,
}

impl Setup for MarkdownBoxTable {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.caption
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        self.caption.set_text("a box table in a phone column, drawn as a real table");
        self.caption.set_text_color(BLACK);
        self.caption.set_frame((LEFT, 12.0, 400.0, 20.0));

        let back = DefaultStyleBack;
        STYLE.apply_globally();
        self.text.set_text_color(BLACK);
        self.text.set_text(TEXT);
        self.height = self.text.height_for_width(WIDTH);
        drop(back);
        self.text.set_frame((LEFT, TOP, WIDTH, self.height));
        self.background.set_color(Color::hex("#e8f0fe"));
        self.background.set_frame((LEFT, TOP, WIDTH, self.height));
    }
}

/// A label of the markdown view: its text, its frame as left, top, right
/// and bottom, and the widest of its lines.
#[derive(Debug)]
struct Text {
    text:   String,
    frame:  (f32, f32, f32, f32),
    lines:  usize,
    widest: f32,
}

impl Text {
    fn overlaps(&self, other: &Text) -> bool {
        let (a, b) = (self.frame, other.frame);
        a.0 < b.2 - 0.01 && b.0 < a.2 - 0.01 && a.1 < b.3 - 0.01 && b.1 < a.3 - 0.01
    }
}

impl ViewTest for MarkdownBoxTable {
    fn canvas() -> (u32, u32) {
        (420, 340)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);
        check_colors(COLORS)?;

        let (texts, height) = from_main(move || {
            let texts: Vec<Text> = view
                .text
                .subviews()
                .iter()
                .filter_map(|child| child.downcast_view::<Label>())
                .map(|label| {
                    let layout = label.text_layout_for(label.text());
                    let widest = (0..layout.line_count())
                        .map(|line| layout.x_on_line(line, layout.line_range(line).end))
                        .fold(0.0, f32::max);
                    Text {
                        text:   label.text().to_string(),
                        frame:  (label.x(), label.y(), label.max_x(), label.max_y()),
                        lines:  layout.line_count(),
                        // A label wraps in its frame less 1 inset.
                        widest: widest + label.text_inset(),
                    }
                })
                .collect();
            (texts, view.height)
        });

        // No line of the typed table is drawn, every cell is a label.
        let typed: Vec<&Text> = texts.iter().filter(|text| text.text.contains('│')).collect();
        ensure!(typed.is_empty(), "the table is drawn as typed: {typed:?}");
        for head in HEAD {
            ensure!(
                texts.iter().any(|text| text.text == head),
                "no cell holds the head {head}"
            );
        }

        // The cells stand apart and inside the column.
        let cells: Vec<&Text> = texts.iter().filter(|text| text.text != "The steps:").collect();
        for (index, one) in cells.iter().enumerate() {
            ensure!(
                one.frame.0 >= -0.01 && one.frame.2 <= WIDTH + 0.01,
                "the cell {:?} leaves the column of {WIDTH}",
                one.text
            );
            for other in &cells[index + 1..] {
                ensure!(!one.overlaps(other), "the cells {one:?} and {other:?} overlap");
            }
        }

        // A short word keeps its line, only a word wider than any column
        // breaks.
        for word in WHOLE {
            let cell = cells
                .iter()
                .find(|text| text.text == word)
                .ok_or_else(|| anyhow::anyhow!("no cell holds {word}"))?;
            ensure!(cell.lines == 1, "{word} broke into {} lines", cell.lines);
        }

        // The long word breaks and stays inside its cell.
        let long = cells
            .iter()
            .find(|text| text.text == LONG)
            .ok_or_else(|| anyhow::anyhow!("no cell holds {LONG}"))?;
        ensure!(long.lines >= 2, "{LONG} did not break, {} line", long.lines);
        let width = long.frame.2 - long.frame.0;
        ensure!(
            long.widest <= width + 0.5,
            "{LONG} is {} wide in a cell of {width}",
            long.widest
        );

        // Nothing runs out of the measured height.
        let lowest = texts.iter().map(|text| text.frame.3).fold(0.0, f32::max);
        ensure!(
            lowest <= height + 0.01,
            "a text ends at {lowest}, the measured height is {height}"
        );

        Ok(())
    }
}
