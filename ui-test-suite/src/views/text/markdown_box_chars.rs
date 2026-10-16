use anyhow::{Result, ensure};
use hilen::{
    AppRunner,
    dispatch::from_main,
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    ui::{
        BLACK, Font, Label, MarkdownFonts, MarkdownStyle, MarkdownView, Setup, TextAlignment, U8Color,
        VerticalAlignment, View, ViewData, ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
    window::Screenshot,
};

use crate::text_points::label_with;

/// 0xProto Nerd Font cut down to ASCII and the box chars, the font of
/// the chat of banda, where the gap in the tables was found.
const PROTO: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/fonts/0xProtoBoxChars.ttf"
));

/// A table of 3 rows drawn with box chars, the way Claude draws one.
const TABLE: &str = r"╭─────┬─────╮
│ box │ row │
├─────┼─────┤
│ a   │ 1   │
├─────┼─────┤
│ b   │ 2   │
╰─────┴─────╯";

/// The same table for `DotGothic16`, a Japanese font. Its box chars and
/// letters are 1 em wide and its space half of that, so the table needs
/// wide letters and wide digits to stand.
const WIDE_TABLE: &str = r"╭──┬──╮
│ 箱 │ 行 │
├──┼──┤
│ あ │ １ │
├──┼──┤
│ い │ ２ │
╰──┴──╯";

/// Recorded with `--record-colors`.
const COLORS: &str = r"
             632    4 - #ffffff
             128   48 - #b2b2b2
             148   48 - #212121
             432   48 - #646464
              36   52 - #ffffff
              80   52 - #e4e4e4
             164   52 - #e5e5e5
             348   52 - #ffffff
             380   52 - #e9e9e9
             432   52 - #646464
             444   52 - #ffffff
             108   92 - #939598
             476   96 - #000000
             376  104 - #c3c6c9
             432  104 - #e3e6ea
             148  108 - #7a7c7e
              64  112 - #7a7c7e
             148  120 - #7a7c7e
             392  124 - #010101
              64  136 - #141414
             492  136 - #babcbf
             148  148 - #7a7c7e
             384  148 - #e6e9ec
             424  148 - #e6e9ec
              64  156 - #7a7c7e
             148  156 - #7a7c7e
             148  160 - #7a7c7e
             452  172 - #e0e3e6
              64  176 - #141414
             148  176 - #141414
             148  188 - #7a7c7e
             376  188 - #c3c6c9
              64  192 - #7a7c7e
             492  192 - #babcbf
             148  196 - #7a7c7e
             424  196 - #000000
              64  208 - #141414
             108  216 - #d4d7da
             392  216 - #000000
             456  224 - #caccd0
             632  236 - #ffffff
             376  240 - #c3c6c9
             492  240 - #babcbf
             420  248 - #e4e7eb
             132  304 - #989898
             444  304 - #989898
              36  308 - #ffffff
              88  308 - #0b0b0b
             100  308 - #838383
             112  308 - #313131
             132  308 - #989898
             164  308 - #9d9d9d
             348  308 - #ffffff
             380  308 - #e9e9e9
             396  308 - #ffffff
             412  308 - #838383
             444  308 - #989898
             476  308 - #9d9d9d
             428  340 - #919191
             348  344 - #454545
             468  352 - #5d5d5d
             156  356 - #5d5d5d
             384  360 - #ffffff
              36  364 - #454545
             348  364 - #454545
              96  372 - #000000
              56  376 - #f7f7f7
             156  376 - #5d5d5d
             436  376 - #969696
              36  392 - #454545
             348  392 - #454545
             620  392 - #ffffff
             156  396 - #5d5d5d
             428  400 - #000000
             468  400 - #5d5d5d
             356  416 - #919191
             396  416 - #919191
             156  428 - #5d5d5d
              56  432 - #020202
             116  436 - #010101
             348  440 - #454545
             468  440 - #5d5d5d
              36  444 - #454545
             156  452 - #5d5d5d
             368  452 - #969696
             408  452 - #000000
              36  472 - #454545
             156  472 - #5d5d5d
              96  476 - #000000
              52  496 - #e0e0e0
             116  500 - #575757
             156  516 - #5d5d5d
             104  524 - #080808
             284  552 - #ffffff
             512  552 - #ffffff
             632  552 - #ffffff
            ";

const WIDTH: f32 = 296.0;
const LEFT: f32 = 16.0;
const RIGHT: f32 = 328.0;
const TOP: f32 = 44.0;
const BOTTOM: f32 = 300.0;
const CAPTION: f32 = 30.0;

/// The darkest pixel near a line of the table is lighter than this at a
/// gap or a seam. The background of a code block is 241, the soft edge of
/// a stroke of 0xProto was 190 at a seam before the box chars joined, a
/// stroke that joins is under 120.
const GAP: u8 = 160;

/// Puts the default style back when it goes away, also when the code that
/// runs under another style panics.
struct DefaultStyleBack;

impl Drop for DefaultStyleBack {
    fn drop(&mut self) {
        MarkdownStyle::DEFAULT.apply_globally();
    }
}

/// How 1 of the 4 texts is drawn.
#[derive(Clone, Copy)]
struct Case {
    table:       &'static str,
    font:        Weak<Font>,
    size:        f32,
    /// A plain text with its line breaks kept and this line height. A code
    /// block with the pitch of its font when `None`.
    line_height: Option<f32>,
}

impl Case {
    fn markdown(self) -> String {
        match self.line_height {
            Some(_) => self.table.to_string(),
            None => format!("```\n{}\n```", self.table),
        }
    }

    /// Lays `text` out with the style of this case.
    fn show(self, text: Weak<MarkdownView>, x: f32, y: f32) {
        let back = DefaultStyleBack;
        MarkdownStyle {
            text_size: self.size,
            code_size: self.size,
            line_height: self.line_height,
            keeps_line_breaks: self.line_height.is_some(),
            fonts: Some(MarkdownFonts {
                regular: self.font,
                mono: self.font,
                ..MarkdownFonts::bundled()
            }),
            ..MarkdownStyle::DEFAULT
        }
        .apply_globally();
        text.set_text_color(BLACK);
        text.set_text(&self.markdown());
        let height = text.height_for_width(WIDTH);
        text.set_frame((x, y + CAPTION, WIDTH, height));
        drop(back);
    }
}

/// The same table of box chars 4 times: in a code block of a font whose
/// box chars are shorter than its line pitch, in a code block of the font
/// of banda, and as a plain text with its line breaks kept, with a line
/// height taller and shorter than the box chars. In every one the lines
/// of the table join, with no gap and no lighter seam between 2 rows and
/// between 2 cells.
#[view]
struct MarkdownBoxChars {
    #[init]
    gothic_caption: Label,
    proto_caption:  Label,
    tall_caption:   Label,
    short_caption:  Label,
    gothic:         MarkdownView,
    proto:          MarkdownView,
    tall:           MarkdownView,
    short:          MarkdownView,
}

impl MarkdownBoxChars {
    fn caption(label: Weak<Label>, text: &str, x: f32, y: f32) {
        label
            .set_multiline(true)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_text(text);
        label.set_text_color(BLACK);
        label.set_frame((x, y, WIDTH, CAPTION));
    }

    fn texts(self: Weak<Self>) -> [Weak<MarkdownView>; 4] {
        [self.gothic, self.proto, self.tall, self.short]
    }
}

impl Setup for MarkdownBoxChars {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        let gothic = Font::get("DotGothic16-Regular.ttf");
        let proto = Font::with_variations_darkened("0xproto-box-chars", PROTO, &[], 0.3)
            .expect("the 0xProto font loads");

        Self::caption(self.gothic_caption, "DotGothic16 code block", LEFT, TOP);
        Self::caption(self.proto_caption, "0xProto code block", RIGHT, TOP);
        Self::caption(self.tall_caption, "0xProto text, line height 30", LEFT, BOTTOM);
        Self::caption(self.short_caption, "0xProto text, line height 19", RIGHT, BOTTOM);

        let code = |table, font, size| Case {
            table,
            font,
            size,
            line_height: None,
        };
        let text = |line_height| Case {
            table:       TABLE,
            font:        proto,
            size:        16.0,
            line_height: Some(line_height),
        };
        code(WIDE_TABLE, gothic, 14.0).show(self.gothic, LEFT, TOP);
        code(TABLE, proto, 16.0).show(self.proto, RIGHT, TOP);
        text(30.0).show(self.tall, LEFT, BOTTOM);
        text(19.0).show(self.short, RIGHT, BOTTOM);
    }
}

/// Where the lines of the table of 1 text are on the screen, in pixels.
struct Lines {
    name:  &'static str,
    /// The middle of the left `│`.
    bar:   f32,
    /// The run of `─` and `┬` of the top border, from the right edge of
    /// `╭` to the left edge of `╮`.
    left:  f32,
    right: f32,
    /// The top of each line and the line pitch.
    top:   f32,
    pitch: f32,
    /// How many lines the table has.
    count: f32,
}

impl Lines {
    fn of(name: &'static str, text: Weak<MarkdownView>) -> Self {
        let label = label_with(text.weak_view(), "╭");
        let layout = label.text_layout_for(label.text());
        let frame = label.absolute_frame();
        let x = frame.x() + label.text_inset();
        let bar = '│'.len_utf8();
        let corner = layout.line_range(0).end - '╮'.len_utf8();
        let middle =
            |line, byte| x + f32::midpoint(layout.x_on_line(line, byte), layout.x_on_line(line, byte + bar));
        Self {
            name,
            bar: middle(1, layout.line_range(1).start),
            left: x + layout.x_on_line(0, layout.line_range(0).start + bar),
            right: x + layout.x_on_line(0, corner),
            top: frame.y(),
            pitch: layout.line_height,
            count: layout.line_count().lossy_convert(),
        }
    }

    /// The darkest pixel of every row from the middle of the first line
    /// to the middle of the last one, near the left `│`. It goes through
    /// the border between every 2 rows of the table.
    fn down(&self, shot: &Screenshot) -> Vec<(u32, u8)> {
        let x: u32 = self.bar.round().lossy_convert();
        let first: u32 = (self.top + self.pitch / 2.0).ceil().lossy_convert();
        let last: u32 = (self.top + self.pitch * (self.count - 0.5)).floor().lossy_convert();
        (first..=last)
            .map(|y| {
                (
                    y,
                    (x - 2..=x + 2).map(|x| ink(shot.get_pixel((x, y)))).min().unwrap_or(u8::MAX),
                )
            })
            .collect()
    }

    /// The darkest pixel of every column of the top line from the right
    /// edge of `╭` to the left edge of `╮`, through the border of every
    /// 2 cells.
    fn across(&self, shot: &Screenshot) -> Vec<(u32, u8)> {
        let first: u32 = self.left.ceil().lossy_convert();
        let last: u32 = self.right.floor().lossy_convert();
        let top: u32 = self.top.ceil().lossy_convert();
        let bottom: u32 = (self.top + self.pitch).floor().lossy_convert();
        (first..=last)
            .map(|x| {
                (
                    x,
                    (top..bottom).map(|y| ink(shot.get_pixel((x, y)))).min().unwrap_or(u8::MAX),
                )
            })
            .collect()
    }
}

/// How light a pixel is, the mean of its 3 channels. Black ink is 0.
fn ink(pixel: U8Color) -> u8 {
    let sum = u16::from(pixel.r) + u16::from(pixel.g) + u16::from(pixel.b);
    u8::try_from(sum / 3).unwrap_or(u8::MAX)
}

/// The places of a line of pixels with no stroke near, a gap or a seam in
/// a line of the table.
fn seams(darkest: &[(u32, u8)]) -> Vec<(u32, u8)> {
    darkest.iter().copied().filter(|(_, ink)| *ink > GAP).collect()
}

impl ViewTest for MarkdownBoxChars {
    fn canvas() -> (u32, u32) {
        (640, 560)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // 3 times the default, the lines of the 4 tables are thin.
        set_record_probe_count(96);
        check_colors(COLORS)?;

        let lines = from_main(move || {
            let names = [
                "DotGothic16 code block",
                "0xProto code block",
                "line height 30",
                "line height 19",
            ];
            names
                .into_iter()
                .zip(view.texts())
                .map(|(name, text)| Lines::of(name, text))
                .collect::<Vec<_>>()
        });

        let shot = AppRunner::take_screenshot()?;
        let mut wrong = Vec::new();
        for lines in &lines {
            let down = seams(&lines.down(&shot));
            if !down.is_empty() {
                wrong.push(format!("{}: the left │ has a gap at rows {down:?}", lines.name));
            }
            let across = seams(&lines.across(&shot));
            if !across.is_empty() {
                wrong.push(format!(
                    "{}: the top border has a gap at columns {across:?}",
                    lines.name
                ));
            }
        }
        ensure!(
            wrong.is_empty(),
            "the box chars do not join:\n{}",
            wrong.join("\n")
        );

        Ok(())
    }
}
