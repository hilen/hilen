use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, Color, Container, Label, MarkdownListMarkers, MarkdownStyle, MarkdownView, Setup,
        TextAlignment, TextSelection, UIManager, VerticalAlignment, View, ViewData, ViewFrame, ViewSubviews,
        ViewTest, WHITE, WeakView, view,
    },
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

use crate::text_points::{drag, label_with, point_after, point_of, shown};

/// A numbered list that goes from 1 digit to 2, with an item that wraps
/// and a list inside the last item, then a list of 2 tasks and a bullet.
/// The bullet is last, its marker is as high as the line the item takes.
const TEXT: &str = r"8. Eight
9. Nine
10. Ten is a long item, it wraps and its second line starts under its first
11. Eleven
    - Inside eleven
    - Inside too

- [x] A done task
- [ ] An open task
- A bullet
";

/// Recorded with `--record-colors`, both lists with nothing selected.
const LISTS: &str = r"
             632    4 - #ffffff
              56   16 - #070707
             100   16 - #464646
             212   16 - #bababa
             356   16 - #878787
             472   16 - #535353
             552   16 - #a1a1a1
              36   20 - #ffffff
              84   20 - #3b3b3b
             156   20 - #eaeaea
             180   20 - #ffffff
             192   20 - #c6c6c6
             204   20 - #0f0f0f
             432   20 - #6a6a6a
             460   20 - #ffffff
             488   20 - #ffffff
             508   20 - #9c9c9c
             540   20 - #ffffff
              72   24 - #ffffff
             112   24 - #ffffff
             400   24 - #ffffff
             524   24 - #ffffff
             368   32 - #ffffff
             448   32 - #ffffff
             536   32 - #4c4c4c
              52   36 - #cbcbcb
             156   36 - #ffffff
             396   36 - #959595
             420   36 - #131313
             436   36 - #ffffff
             488   36 - #7f7f7f
             496   36 - #313131
             284   52 - #ffffff
             360   68 - #3c3e42
              44   72 - #e8f0fe
              76   72 - #a2a8b1
             368   72 - #0f1011
             376   72 - #000000
             384   72 - #010101
             632   84 - #ffffff
              44   92 - #e8f0fe
             360   92 - #3c3e42
              60   96 - #7a7e85
              72   96 - #55585d
              36  116 - #939ba9
             140  116 - #e1e8f6
             420  116 - #3d3f43
              36  120 - #939ba9
              72  120 - #e8f0fe
             104  120 - #000000
             140  120 - #e1e8f6
             156  120 - #52555a
             176  120 - #b1b8c2
             196  120 - #b1b8c2
             232  120 - #000000
             240  120 - #1c1d1f
             348  120 - #6d7482
             384  120 - #b3b9c4
             436  120 - #61656b
             484  120 - #000000
             544  120 - #010101
             564  120 - #83888f
              88  136 - #e0e8f5
             108  136 - #27282b
             180  136 - #e0e8f5
              88  140 - #e0e8f5
             108  140 - #27282b
             112  140 - #26282a
             152  140 - #c9d0dc
             180  140 - #e0e8f5
             212  140 - #56585e
             236  140 - #515459
             420  140 - #6c7077
             428  140 - #484a4f
             500  140 - #e8f0fe
             540  140 - #2c2d30
             544  140 - #2c2d30
             548  140 - #28292c
             628  156 - #ffffff
              36  160 - #939ba9
             348  160 - #6d7482
              36  164 - #939ba9
              44  164 - #7a818f
              68  164 - #9a9fa8
              92  164 - #000000
             356  164 - #7a818f
             372  164 - #e8f0fe
             400  164 - #9ba0a9
              80  184 - #64686e
             404  184 - #bbc1cc
              80  188 - #64686e
              88  188 - #61656b
             124  188 - #e8f0fe
             152  188 - #26282a
             156  188 - #a5aab4
             380  188 - #010101
             384  188 - #606369
             420  188 - #e8f0fe
             284  192 - #e8f0fe
              80  208 - #64686e
             376  208 - #52555a
             404  208 - #bbc1cc
              80  212 - #64686e
              88  212 - #61656b
             112  212 - #e8f0fe
             384  212 - #606369
             424  212 - #e8f0fe
             632  228 - #ffffff
              36  236 - #2f7df6
              40  236 - #2f7df6
              44  236 - #2f7df6
             120  236 - #3e4044
             348  236 - #2f7df6
             352  236 - #2f7df6
             356  236 - #2f7df6
             380  236 - #010101
             428  236 - #e0e8f5
              32  240 - #2f7df6
              44  240 - #2f7df6
              76  240 - #e8f0fe
             100  240 - #787c83
             344  240 - #2f7df6
             356  240 - #2f7df6
             412  240 - #e8f0fe
             428  240 - #e0e8f5
              36  244 - #2f7df6
              40  244 - #2f7df6
              44  244 - #2f7df6
             348  244 - #2f7df6
             352  244 - #2f7df6
             356  244 - #2f7df6
             240  248 - #e8f0fe
             532  252 - #e8f0fe
             128  260 - #83888f
              32  264 - #6b7280
              76  264 - #e8f0fe
              96  264 - #83888f
             108  264 - #323437
             344  264 - #6b7280
             356  264 - #aab1bf
             436  264 - #9ba0a9
              84  284 - #9ba0a9
              68  288 - #e8f0fe
              96  288 - #000000
             368  288 - #e8f0fe
             396  288 - #040404
             188  312 - #ffffff
              52  316 - #000000
             568  316 - #ffffff
              36  320 - #ffffff
              76  320 - #1d1d1d
              60  336 - #ffffff
              76  336 - #ffffff
              92  336 - #6b6b6b
             112  336 - #818181
             256  352 - #ffffff
             332  352 - #ffffff
             432  352 - #ffffff
             504  352 - #ffffff
             632  352 - #ffffff
            ";

/// Recorded with `--record-colors`, a selection in the right list.
const SELECTED: &str = r"
             632    4 - #ffffff
              56   16 - #070707
             100   16 - #464646
             160   16 - #b2b2b2
             356   16 - #878787
             472   16 - #535353
             552   16 - #a1a1a1
              36   20 - #ffffff
              84   20 - #3b3b3b
             124   20 - #dedede
             180   20 - #ffffff
             208   20 - #000000
             432   20 - #6a6a6a
             488   20 - #ffffff
             508   20 - #9c9c9c
             400   24 - #ffffff
             368   32 - #ffffff
             536   32 - #4c4c4c
              52   36 - #cbcbcb
             156   36 - #ffffff
             396   36 - #959595
             420   36 - #131313
             452   36 - #ffffff
             496   36 - #313131
             284   52 - #ffffff
             360   68 - #3c3e42
              44   72 - #e8f0fe
              76   72 - #a2a8b1
             380   72 - #6e7279
             376   84 - #a9c9fe
             364   88 - #a9c9fe
             392   88 - #a9c9fe
             632   88 - #ffffff
             356   92 - #a9c9fe
             360   92 - #212d42
             372   92 - #a9c9fe
             380   92 - #a9c9fe
             388   92 - #a9c9fe
              60   96 - #7a7e85
             360   96 - #212d42
             376   96 - #a9c9fe
             384   96 - #a9c9fe
             360  100 - #80affe
             368  100 - #a9c9fe
             380  100 - #a9c9fe
             392  100 - #a9c9fe
             348  108 - #a9c9fe
             364  108 - #a9c9fe
             372  108 - #a9c9fe
             352  112 - #a9c9fe
             380  112 - #a9c9fe
             388  112 - #a9c9fe
              36  116 - #939ba9
             140  116 - #e1e8f6
             344  116 - #a9c9fe
             356  116 - #a9c9fe
             368  116 - #80affe
             420  116 - #3d3f43
              36  120 - #939ba9
             104  120 - #000000
             140  120 - #e1e8f6
             176  120 - #b1b8c2
             196  120 - #b1b8c2
             232  120 - #000000
             240  120 - #1c1d1f
             368  120 - #80affe
             384  120 - #839bc4
             388  120 - #252c38
             436  120 - #61656b
             484  120 - #000000
             556  120 - #000000
             348  124 - #a9c9fe
             356  124 - #a9c9fe
             368  124 - #80affe
             380  124 - #a9c9fe
              88  136 - #e0e8f5
             108  136 - #27282b
             180  136 - #e0e8f5
              88  140 - #e0e8f5
             112  140 - #26282a
             152  140 - #c9d0dc
             180  140 - #e0e8f5
             212  140 - #56585e
             236  140 - #515459
             420  140 - #6c7077
             428  140 - #484a4f
             500  140 - #e8f0fe
             540  140 - #2c2d30
             548  140 - #28292c
              36  160 - #939ba9
              68  160 - #9a9fa8
             348  160 - #6d7482
              36  164 - #939ba9
              92  164 - #000000
             372  164 - #e8f0fe
             400  164 - #9ba0a9
              80  184 - #64686e
             404  184 - #bbc1cc
              80  188 - #64686e
              88  188 - #61656b
             124  188 - #e8f0fe
             156  188 - #a5aab4
             384  188 - #606369
             424  188 - #848890
             632  204 - #ffffff
              80  208 - #64686e
             404  208 - #bbc1cc
              88  212 - #61656b
             112  212 - #e8f0fe
             384  212 - #606369
             432  212 - #e8f0fe
             248  228 - #e8f0fe
              36  236 - #2f7df6
              44  236 - #2f7df6
             120  236 - #3e4044
             348  236 - #2f7df6
             352  236 - #2f7df6
             356  236 - #2f7df6
             380  236 - #010101
             428  236 - #e0e8f5
              32  240 - #2f7df6
              76  240 - #e8f0fe
             100  240 - #787c83
             344  240 - #2f7df6
             356  240 - #2f7df6
             428  240 - #e0e8f5
              36  244 - #2f7df6
              44  244 - #2f7df6
             348  244 - #2f7df6
             352  244 - #2f7df6
             356  244 - #2f7df6
             536  252 - #e8f0fe
             128  260 - #83888f
              32  264 - #6b7280
              84  264 - #e8f0fe
             108  264 - #323437
             344  264 - #6b7280
             436  264 - #9ba0a9
             608  276 - #e8f0fe
              84  284 - #9ba0a9
              68  288 - #e8f0fe
              96  288 - #000000
             384  288 - #c9d0dc
              80  316 - #080808
             184  316 - #000000
              36  320 - #ffffff
             120  320 - #505050
             148  320 - #282828
             208  320 - #1e1e1e
             224  320 - #ffffff
             260  320 - #ffffff
             276  320 - #939393
              68  324 - #ffffff
              88  332 - #606060
             100  336 - #989898
             136  336 - #ffffff
             344  352 - #ffffff
             424  352 - #ffffff
             500  352 - #ffffff
             632  352 - #ffffff
            ";

/// A line pitch in whole points keeps every row on a whole pixel. With
/// the pitch of the font the box of a task is on a part of a pixel, and a
/// probe on its edge does not hold with 1 sample.
const LINE: f32 = 20.0;

const COLUMN: MarkdownStyle = MarkdownStyle {
    line_height: Some(LINE),
    ..MarkdownStyle::DEFAULT
};

const INLINE: MarkdownStyle = MarkdownStyle {
    list_markers: MarkdownListMarkers::Inline,
    ..COLUMN
};

const WIDTH: f32 = 296.0;
const LEFT: f32 = 16.0;
const RIGHT: f32 = 328.0;
const TOP: f32 = 60.0;

/// The texts of the numbered items, each one a part of 1 label only.
const NUMBERED: [(&str, &str); 4] = [
    ("8.", "Eight"),
    ("9.", "Nine"),
    ("10.", "Ten is"),
    ("11.", "Eleven"),
];

/// Puts the default style back when it goes away, also when the code that
/// runs under another style panics. The style is global, a test that left
/// its own behind would draw every markdown view after it with it.
struct DefaultStyleBack;

impl Drop for DefaultStyleBack {
    fn drop(&mut self) {
        MarkdownStyle::DEFAULT.apply_globally();
    }
}

/// Runs `action` with `style` as the global one.
fn with_style<T>(style: &MarkdownStyle, action: impl FnOnce() -> T) -> T {
    let back = DefaultStyleBack;
    style.apply_globally();
    let result = action();
    drop(back);
    result
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// Where a marker and the text of its item are, in the points of the
/// markdown view.
struct Row {
    name:        &'static str,
    marker_x:    f32,
    /// The right end of the ink of the marker.
    marker_end:  f32,
    /// The left end of the ink of the text.
    text_start:  f32,
    text_height: f32,
}

impl Row {
    /// `inline` says which of the 2 looks the view was laid out with, a
    /// marker of the default look has its ink at the right of its frame.
    fn of(root: WeakView, inline: bool, marker: &'static str, text: &'static str) -> Self {
        let marker = label_with(root, marker);
        let label = label_with(root, text);
        // The ink a label would need on 1 line, a width no marker reaches.
        let ink = marker.size_for_width(10_000.0).width - marker.text_inset();
        let ink_start = if inline {
            marker.x() + marker.text_inset()
        } else {
            marker.max_x() - marker.text_inset() - ink
        };
        Self {
            name:        text,
            marker_x:    ink_start,
            marker_end:  ink_start + ink,
            text_start:  label.x() + label.text_inset(),
            text_height: label.height(),
        }
    }

    /// The empty room between the marker and the text.
    fn gap(&self) -> f32 {
        self.text_start - self.marker_end
    }
}

/// What the test reads from 1 of the 2 views.
struct Laid {
    numbered: Vec<Row>,
    /// The first bullet of the list inside item 11.
    nested:   Row,
    /// The lowest edge of all the views the text made.
    bottom:   f32,
}

impl Laid {
    fn of(text: Weak<MarkdownView>, inline: bool) -> Self {
        let root = text.weak_view();
        Self {
            numbered: NUMBERED
                .iter()
                .map(|(marker, item)| Row::of(root, inline, marker, item))
                .collect(),
            nested:   Row::of(root, inline, "•", "Inside eleven"),
            bottom:   text.subviews().iter().map(|view| view.max_y()).fold(0.0, f32::max),
        }
    }

    fn check_column(&self) -> Result<()> {
        // In a column the markers end at 1 place and the texts start at
        // 1 place.
        let first = &self.numbered[0];
        for row in &self.numbered {
            ensure!(
                near(row.marker_end, first.marker_end) && near(row.text_start, first.text_start),
                "column: the marker of `{}` ends at {} and its text starts at {}, `{}` has {} and {}",
                row.name,
                row.marker_end,
                row.text_start,
                first.name,
                first.marker_end,
                first.text_start
            );
        }
        ensure!(
            self.numbered[0].marker_x > self.numbered[2].marker_x,
            "column: the marker 8. starts at {}, the marker 10. at {}",
            self.numbered[0].marker_x,
            self.numbered[2].marker_x
        );
        Ok(())
    }

    fn check_inline(&self) -> Result<()> {
        // Inline: every marker starts where the first one starts, and
        // every text starts the same small room after its own marker.
        let first = &self.numbered[0];
        ensure!(
            first.gap() > 2.0 && first.gap() < 6.0,
            "inline: the text of `{}` starts {} after its marker, not 1 space",
            first.name,
            first.gap()
        );
        for row in self.numbered.iter().chain([&self.nested]) {
            ensure!(
                near(row.gap(), first.gap()),
                "inline: the text of `{}` starts {} after its marker, `{}` has {}",
                row.name,
                row.gap(),
                first.name,
                first.gap()
            );
        }
        for row in &self.numbered {
            ensure!(
                near(row.marker_x, first.marker_x),
                "inline: the marker of `{}` starts at {}, the first one at {}",
                row.name,
                row.marker_x,
                first.marker_x
            );
        }
        let (nine, ten) = (&self.numbered[1], &self.numbered[2]);
        ensure!(
            ten.text_start > nine.text_start + 4.0,
            "inline: the text of item 10 starts at {}, the text of item 9 at {}",
            ten.text_start,
            nine.text_start
        );
        // The item that wraps is 1 label of 2 lines, so its second line
        // starts where its first line starts.
        ensure!(
            near(ten.text_height, nine.text_height * 2.0),
            "inline: the item that wraps is {} high, 1 line is {}",
            ten.text_height,
            nine.text_height
        );
        // A list inside an item starts where the text of the item starts.
        ensure!(
            near(self.nested.marker_x, self.numbered[3].text_start),
            "inline: the list inside item 11 starts at {}, the text of item 11 at {}",
            self.nested.marker_x,
            self.numbered[3].text_start
        );
        Ok(())
    }
}

/// The same text 2 times, left with the markers in 1 column, the default,
/// and right with the markers of a terminal. The tinted box behind each text is
/// as high as `height_for_width` said. The right text can be selected.
#[view]
struct MarkdownInlineMarkers {
    column_height: f32,
    inline_height: f32,

    #[init]
    column_caption: Label,
    inline_caption: Label,
    column_box:     Container,
    inline_box:     Container,
    column_text:    MarkdownView,
    inline_text:    MarkdownView,
    step:           Label,
}

impl MarkdownInlineMarkers {
    fn caption(label: Weak<Label>, text: &str, x: f32, y: f32) {
        label
            .set_multiline(true)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_text(text);
        label.set_text_color(BLACK);
        label.set_frame((x, y, WIDTH, 40.0));
    }

    /// Lays the text out with the style that is global right now.
    fn show(text: Weak<MarkdownView>, background: Weak<Container>, x: f32) -> f32 {
        text.set_text_color(BLACK);
        text.set_text(TEXT);
        let height = text.height_for_width(WIDTH);
        text.set_frame((x, TOP, WIDTH, height));
        background.set_color(Color::hex("#e8f0fe"));
        background.set_frame((x, TOP, WIDTH, height));
        height
    }

    fn set_step(self: Weak<Self>, step: &'static str) -> String {
        from_main(move || {
            let text = TextSelection::text();
            self.step.set_text(format!("{step}\nselected: {}", shown(&text)));
            text
        })
    }
}

impl Setup for MarkdownInlineMarkers {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);

        Self::caption(
            self.column_caption,
            "markers in 1 column, the default\nevery text starts at 1 place",
            LEFT,
            12.0,
        );
        Self::caption(
            self.inline_caption,
            "inline markers, the look of a terminal\na text starts 1 space after its marker",
            RIGHT,
            12.0,
        );

        self.column_height = with_style(&COLUMN, || Self::show(self.column_text, self.column_box, LEFT));
        self.inline_text.set_selectable(true);
        self.inline_height = with_style(&INLINE, || Self::show(self.inline_text, self.inline_box, RIGHT));

        Self::caption(self.step, "both lists\nselected: nothing", LEFT, 312.0);
    }
}

impl ViewTest for MarkdownInlineMarkers {
    fn canvas() -> (u32, u32) {
        (640, 360)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // 5 times the default, the picture is mostly small text.
        set_record_probe_count(160);

        from_main(|| UIManager::set_drag_scrolling(false));
        // A selectable view lays its text out again on the turn after it
        // was made selectable.
        wait_for_next_frame();
        wait_for_next_frame();
        check_colors(LISTS)?;

        let (column, inline, column_height, inline_height) = from_main(move || {
            (
                Laid::of(view.column_text, false),
                Laid::of(view.inline_text, true),
                view.column_height,
                view.inline_height,
            )
        });

        column.check_column()?;
        inline.check_inline()?;
        for (name, laid, height) in [
            ("column", &column, column_height),
            ("inline", &inline, inline_height),
        ] {
            ensure!(
                near(laid.bottom, height),
                "{name}: the views end at {}, `height_for_width` said {height}",
                laid.bottom
            );
        }

        // The selection follows the markers to their new place.
        let root = view.inline_text.weak_view();
        let (from, to) = from_main(move || {
            (
                point_of(label_with(root, "Nine"), "Nine"),
                point_after(label_with(root, "Ten is"), "Ten"),
            )
        });
        inject_touches(drag(from, to));
        let selected = view.set_step("drag in the right list from Nine to the end of Ten");
        ensure!(
            selected == "Nine\n10. Ten",
            "the drag selected `{}`",
            shown(&selected)
        );
        check_colors(SELECTED)?;

        Ok(())
    }
}
