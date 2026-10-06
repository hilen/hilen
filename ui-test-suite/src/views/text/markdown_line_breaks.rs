use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Container, Font, Label, MarkdownFonts, MarkdownStyle, MarkdownView, Setup,
        TextAlignment, VerticalAlignment, ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

/// A paragraph of 1 line, a table drawn with plain chars and no code fence
/// around it, so it is a paragraph of 5 lines, and a paragraph of 2 lines.
const TEXT: &str = r"A table of plain chars:

+------+-----+
| Case | Row |
+------+-----+
| Grid | 6   |
+------+-----+

first line
second line
";

/// Recorded with `--record-colors`.
const COLORS: &str = r"
             284    4 - #ffffff
             616    4 - #ffffff
             344   16 - #e5e5e5
             432   16 - #3a3a3a
              36   20 - #ffffff
              48   20 - #282828
              68   20 - #ffffff
              84   20 - #363636
              92   20 - #ffffff
             344   20 - #e5e5e5
             368   20 - #ffffff
             388   20 - #b2b2b2
             408   20 - #ffffff
             416   20 - #ffffff
             420   20 - #0b0b0b
             436   20 - #ffffff
              68   24 - #ffffff
              76   24 - #ffffff
              92   24 - #ffffff
              96   24 - #ffffff
             372   24 - #ffffff
             416   24 - #ffffff
              44   32 - #878787
              92   32 - #656565
             428   32 - #000000
              44   36 - #878787
              60   36 - #cecece
              92   36 - #656565
             128   36 - #ffffff
             140   36 - #656565
             156   36 - #000000
             172   36 - #ffffff
             188   36 - #010101
             220   36 - #ffffff
             236   36 - #ffffff
             368   36 - #ffffff
             380   36 - #ffffff
             420   36 - #9e9e9e
             428   36 - #000000
             464   36 - #6b6b6b
             500   36 - #959595
             532   36 - #9e9e9e
             592   64 - #e8f0fe
              52   68 - #040404
             136   68 - #a4a9b3
             184   68 - #5b5e64
             288   68 - #e8f0fe
             364   68 - #040404
             448   68 - #a4a9b3
             496   68 - #5b5e64
              36   72 - #ccd3df
             128   72 - #e8f0fe
             160   72 - #7f848b
             164   72 - #040404
             184   72 - #5b5e64
             212   72 - #e8f0fe
             348   72 - #ccd3df
             440   72 - #e8f0fe
             472   72 - #7f848b
             476   72 - #040404
             496   72 - #5b5e64
             524   72 - #e8f0fe
             220   96 - #73777e
             236   96 - #d2d9e6
              36  100 - #232426
             220  100 - #73777e
             348  100 - #232426
             188  112 - #e8f0fe
              44  116 - #a9afb9
              60  116 - #a9afb9
             112  116 - #a9afb9
             120  116 - #a9afb9
             128  116 - #a9afb9
             136  116 - #a9afb9
             196  116 - #37383c
             220  116 - #73777e
             348  116 - #43464a
             364  116 - #e8f0fe
             420  116 - #c3c9d5
             456  116 - #cfd7e3
             632  116 - #ffffff
             220  120 - #73777e
             348  120 - #43464a
             456  120 - #cfd7e3
             564  132 - #e8f0fe
              44  136 - #abb1bb
              60  136 - #abb1bb
             112  136 - #abb1bb
             120  136 - #abb1bb
             128  136 - #abb1bb
             136  136 - #abb1bb
             284  136 - #e8f0fe
             356  136 - #abb1bb
             364  136 - #abb1bb
             372  136 - #abb1bb
             424  136 - #abb1bb
             432  136 - #abb1bb
             440  136 - #abb1bb
             448  136 - #abb1bb
             348  152 - #43464a
             364  152 - #e8f0fe
             392  152 - #000000
             456  152 - #cfd7e3
             348  156 - #43464a
             372  156 - #424448
             392  156 - #000000
             456  156 - #cfd7e3
             512  156 - #e8f0fe
             188  160 - #9398a1
              44  164 - #47494e
              68  164 - #777b82
              92  164 - #e8f0fe
             100  164 - #5b5e64
             136  164 - #1d1e20
             148  164 - #e8f0fe
             164  164 - #040404
             172  164 - #4c4f54
             188  164 - #9398a1
             196  164 - #37383c
             212  164 - #1d1e20
              44  168 - #74787f
              88  168 - #74787f
              96  168 - #74787f
             188  168 - #74787f
             196  168 - #74787f
             356  172 - #1e1f21
             364  172 - #1e1f21
             372  172 - #1e1f21
             424  172 - #1e1f21
             432  172 - #1e1f21
             440  172 - #1e1f21
             448  172 - #1e1f21
             628  192 - #ffffff
             280  200 - #ffffff
             404  200 - #e8f0fe
             412  200 - #5b5e64
             552  200 - #e8f0fe
             416  204 - #e8f0fe
               4  212 - #ffffff
             492  212 - #e8f0fe
             392  216 - #000000
             356  220 - #4e5156
             364  220 - #e8f0fe
             372  220 - #e8f0fe
             380  220 - #dce4f1
             384  220 - #1c1d1f
             392  220 - #010101
             412  220 - #e8f0fe
             416  220 - #9398a1
             432  220 - #4e5156
              60  232 - #ffffff
             188  240 - #ffffff
             568  264 - #ffffff
               4  272 - #ffffff
             120  272 - #ffffff
             252  272 - #ffffff
             316  272 - #ffffff
             448  272 - #ffffff
             504  272 - #ffffff
             632  272 - #ffffff
            ";

const WIDTH: f32 = 296.0;
const LEFT: f32 = 16.0;
const RIGHT: f32 = 328.0;
const TOP: f32 = 60.0;

/// The 3 paragraphs of `TEXT` in the order the view makes them.
const ONE_LINE: usize = 0;
const TABLE: usize = 1;
const TABLE_LINES: f32 = 5.0;
const TWO_LINES: usize = 2;
const VIEWS: usize = 3;

/// Puts the default style back when it goes away, also when the code that
/// runs under another style panics. The style is global, a test that left
/// its own behind would draw every markdown view after it with it.
struct DefaultStyleBack;

impl Drop for DefaultStyleBack {
    fn drop(&mut self) {
        MarkdownStyle::DEFAULT.apply_globally();
    }
}

/// Runs `action` with a mono font as the global style, so the chars of the
/// table line up, and with the line breaks kept or not.
fn with_mono_style<T>(keeps_line_breaks: bool, action: impl FnOnce() -> T) -> T {
    let back = DefaultStyleBack;
    MarkdownStyle {
        keeps_line_breaks,
        fonts: Some(MarkdownFonts {
            regular: Font::mono(),
            ..MarkdownFonts::bundled()
        }),
        ..MarkdownStyle::DEFAULT
    }
    .apply_globally();
    let result = action();
    drop(back);
    result
}

/// The same text 2 times in a mono font. Left the way markdown reads it,
/// a line break inside a paragraph is a space. Right with the style that
/// keeps the line breaks, the way a terminal shows the text. The tinted
/// box behind each text is as high as `height_for_width` said.
#[view]
struct MarkdownLineBreaks {
    #[init]
    joined_caption: Label,
    kept_caption:   Label,
    joined_box:     Container,
    kept_box:       Container,
    joined_text:    MarkdownView,
    kept_text:      MarkdownView,
}

impl MarkdownLineBreaks {
    fn caption(label: Weak<Label>, text: &str, x: f32) {
        label
            .set_multiline(true)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_text(text);
        label.set_text_color(BLACK);
        label.set_frame((x, 12.0, WIDTH, 40.0));
    }

    /// Lays the text out with the style that is global right now.
    fn show(text: Weak<MarkdownView>, background: Weak<Container>, x: f32) {
        text.set_text_color(BLACK);
        text.set_text(TEXT);
        let height = text.height_for_width(WIDTH);
        text.set_frame((x, TOP, WIDTH, height));
        background.set_color(Color::hex("#e8f0fe"));
        background.set_frame((x, TOP, WIDTH, height));
    }

    fn heights(text: Weak<MarkdownView>) -> Vec<f32> {
        text.subviews().iter().map(|view| view.height()).collect()
    }
}

impl Setup for MarkdownLineBreaks {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        Self::caption(
            self.joined_caption,
            "default style\na line break in a paragraph is a space",
            LEFT,
        );
        Self::caption(
            self.kept_caption,
            "keeps line breaks\nthe table stands, 2 lines stay 2 lines",
            RIGHT,
        );

        with_mono_style(false, || Self::show(self.joined_text, self.joined_box, LEFT));
        with_mono_style(true, || Self::show(self.kept_text, self.kept_box, RIGHT));
    }
}

impl ViewTest for MarkdownLineBreaks {
    fn canvas() -> (u32, u32) {
        (640, 280)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // 5 times the default, the picture is mostly small text.
        set_record_probe_count(160);
        check_colors(COLORS)?;

        let (joined, kept) =
            from_main(move || (Self::heights(view.joined_text), Self::heights(view.kept_text)));
        ensure!(
            joined.len() == VIEWS && kept.len() == VIEWS,
            "{} and {} views, not {VIEWS} each",
            joined.len(),
            kept.len()
        );

        // How many lines a paragraph takes, by the paragraph of 1 line.
        let lines = |heights: &[f32], index: usize| heights[index] / heights[ONE_LINE];
        let whole = |count: f32, wanted: f32| (count - wanted).abs() < 0.2;

        ensure!(
            whole(lines(&kept, TABLE), TABLE_LINES),
            "kept: the table takes {} lines, not {TABLE_LINES}",
            lines(&kept, TABLE)
        );
        ensure!(
            whole(lines(&kept, TWO_LINES), 2.0),
            "kept: the 2 lines take {} lines",
            lines(&kept, TWO_LINES)
        );
        ensure!(
            whole(lines(&joined, TWO_LINES), 1.0),
            "default style: the 2 lines take {} lines, not 1",
            lines(&joined, TWO_LINES)
        );
        ensure!(
            joined[TABLE] < kept[TABLE],
            "default style: the table is {} high, with its line breaks {}",
            joined[TABLE],
            kept[TABLE]
        );

        Ok(())
    }
}
