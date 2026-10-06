use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Container, Label, MarkdownStyle, MarkdownView, Setup, TextAlignment, VerticalAlignment,
        ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

/// 4 headings, each of another level, and a paragraph of 1 line.
const TEXT: &str = r"# Heading of level 1

## Heading of level 2

### Heading of level 3

#### Heading of level 4

A paragraph of body text.
";

/// Recorded with `--record-colors`.
const COLORS: &str = r"
             280    4 - #ffffff
             632    4 - #ffffff
             344   16 - #e5e5e5
              36   20 - #ffffff
              48   20 - #282828
              68   20 - #ffffff
              84   20 - #363636
              92   20 - #ffffff
             344   20 - #e5e5e5
             348   20 - #ffffff
             368   20 - #ffffff
             388   20 - #000000
             404   20 - #ffffff
             420   20 - #0b0b0b
             452   20 - #ffffff
              76   24 - #ffffff
              96   24 - #ffffff
             424   24 - #ffffff
             424   32 - #ffffff
             432   32 - #646464
              48   36 - #676767
              80   36 - #cdcdcd
              92   36 - #979797
             108   36 - #ffffff
             128   36 - #000000
             168   36 - #000000
             176   36 - #ffffff
             188   36 - #cecece
             196   36 - #000000
             212   36 - #ffffff
             360   36 - #676767
             384   36 - #080808
             392   36 - #cdcdcd
             396   36 - #ffffff
             404   36 - #979797
             420   36 - #ffffff
             428   36 - #ffffff
             432   36 - #646464
             448   36 - #ffffff
             456   36 - #ffffff
             480   36 - #c1c1c1
             488   36 - #ffffff
             500   36 - #cecece
             524   36 - #ffffff
             548   36 - #ffffff
              44   64 - #b1b7c1
             344   64 - #cfd6e2
             444   64 - #252729
             624   64 - #ffffff
              36   68 - #595c62
             172   68 - #b5bbc6
             204   68 - #000000
             268   68 - #e8f0fe
             344   68 - #cfd6e2
             348   68 - #46484d
             444   68 - #252729
             452   68 - #a6abb5
              68   72 - #000000
              84   72 - #e0e8f5
              88   72 - #d9e0ed
             112   72 - #000000
             160   72 - #000000
             180   72 - #e8f0fe
              36   76 - #595c62
              56   76 - #e8f0fe
              80   76 - #000000
              84   76 - #e0e8f5
              88   76 - #d9e0ed
             100   76 - #62666c
             136   76 - #000000
             148   76 - #000000
             180   76 - #e8f0fe
             188   76 - #000000
              44   80 - #8b9098
             344   92 - #cfd6e2
             444   92 - #252729
             344   96 - #cfd6e2
             368   96 - #c8cfdb
             424   96 - #a1a7b0
             444   96 - #252729
             572   96 - #e8f0fe
             180  100 - #010101
             508  100 - #e8f0fe
              44  104 - #bac0cb
             132  104 - #000000
             168  104 - #222325
              44  108 - #bac0cb
              64  108 - #242628
              80  108 - #959aa3
              88  108 - #848890
              96  108 - #e8f0fe
             112  108 - #e8f0fe
             120  108 - #3c3e42
             148  108 - #000000
             168  108 - #222325
             160  112 - #010101
             180  112 - #000000
             344  120 - #cfd6e2
             348  120 - #000000
             444  120 - #252729
             632  128 - #ffffff
              68  132 - #56585e
             116  132 - #000000
             156  132 - #000000
              40  136 - #000000
              68  136 - #56585e
              72  136 - #3b3d41
              68  140 - #56585e
              72  140 - #3b3d41
              76  140 - #35373a
             108  140 - #54575c
             344  144 - #cfd6e2
             444  144 - #252729
             344  148 - #cfd6e2
             356  148 - #000000
             368  148 - #c8cfdb
             372  148 - #e8f0fe
             392  148 - #e8f0fe
             404  148 - #e8f0fe
             424  148 - #010101
             444  148 - #252729
             248  156 - #e8f0fe
             540  156 - #e8f0fe
              32  164 - #dde5f2
              68  164 - #1e1f21
             112  164 - #81868d
             140  164 - #dee6f3
              32  168 - #dde5f2
              52  168 - #000000
              68  168 - #1e1f21
              76  168 - #8c9199
             112  168 - #81868d
             140  168 - #dee6f3
             392  172 - #e0e8f5
             412  172 - #e0e8f5
             440  172 - #6a6e75
             360  176 - #e8f0fe
             392  176 - #e0e8f5
             412  176 - #e0e8f5
             432  176 - #9fa5ae
             100  188 - #e0e8f5
             128  188 - #6a6e75
             148  188 - #d8dfec
             600  188 - #ffffff
              48  192 - #e8f0fe
              80  192 - #e0e8f5
             100  192 - #e0e8f5
             120  192 - #9fa5ae
             496  196 - #ffffff
             300  200 - #f2f6fe
             216  204 - #ffffff
             572  240 - #ffffff
               4  252 - #ffffff
              68  252 - #ffffff
             184  252 - #ffffff
             260  252 - #ffffff
             340  252 - #ffffff
             420  252 - #ffffff
             516  252 - #ffffff
             632  252 - #ffffff
            ";

const FLAT: MarkdownStyle = MarkdownStyle {
    heading_scales: [1.0; 6],
    ..MarkdownStyle::DEFAULT
};

const WIDTH: f32 = 296.0;
const LEFT: f32 = 16.0;
const RIGHT: f32 = 328.0;
const TOP: f32 = 60.0;

/// The views of `TEXT` in the order the view makes them: the 4 headings,
/// then the paragraph.
const HEADINGS: usize = 4;
const PARAGRAPH: usize = 4;

/// Puts the default style back when it goes away, also when the code that
/// runs under another style panics. The style is global, a test that left
/// its own behind would draw every markdown view after it with it.
struct DefaultStyleBack;

impl Drop for DefaultStyleBack {
    fn drop(&mut self) {
        MarkdownStyle::DEFAULT.apply_globally();
    }
}

/// Runs `action` with the flat style as the global one.
fn with_flat_style<T>(action: impl FnOnce() -> T) -> T {
    let back = DefaultStyleBack;
    FLAT.apply_globally();
    let result = action();
    drop(back);
    result
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// The same text 2 times, left with the default style and right with a
/// style that draws every heading at the size of the body text. The
/// tinted box behind each text is as high as `height_for_width` said.
#[view]
struct MarkdownHeadingSizes {
    #[init]
    default_caption: Label,
    flat_caption:    Label,
    default_box:     Container,
    flat_box:        Container,
    default_text:    MarkdownView,
    flat_text:       MarkdownView,
}

impl MarkdownHeadingSizes {
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

    /// How high the first views of a text are, the headings and the
    /// paragraph.
    fn heights(text: Weak<MarkdownView>) -> Vec<f32> {
        text.subviews().iter().take(PARAGRAPH + 1).map(|view| view.height()).collect()
    }
}

impl Setup for MarkdownHeadingSizes {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        Self::caption(
            self.default_caption,
            "default style\na heading is bigger than the body text",
            LEFT,
        );
        Self::caption(
            self.flat_caption,
            "heading scales all 1\na heading is bold text of the body size",
            RIGHT,
        );

        Self::show(self.default_text, self.default_box, LEFT);
        with_flat_style(|| Self::show(self.flat_text, self.flat_box, RIGHT));
    }
}

impl ViewTest for MarkdownHeadingSizes {
    fn canvas() -> (u32, u32) {
        (640, 260)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // 5 times the default, the picture is mostly small text.
        set_record_probe_count(160);
        check_colors(COLORS)?;

        let (default, flat) =
            from_main(move || (Self::heights(view.default_text), Self::heights(view.flat_text)));
        ensure!(
            default.len() == PARAGRAPH + 1 && flat.len() == PARAGRAPH + 1,
            "{} and {} views, not {} each",
            default.len(),
            flat.len(),
            PARAGRAPH + 1
        );

        // By default the first 3 levels step down to the body text.
        for level in 0..3 {
            ensure!(
                default[level] > default[level + 1],
                "default style: heading {} is {} high, the next one {}",
                level + 1,
                default[level],
                default[level + 1]
            );
        }
        ensure!(
            default[HEADINGS - 1] > default[PARAGRAPH],
            "default style: heading 4 is {} high, the paragraph {}",
            default[HEADINGS - 1],
            default[PARAGRAPH]
        );

        // With every scale at 1 a heading takes the room of a body line.
        for level in 0..HEADINGS {
            ensure!(
                near(flat[level], flat[PARAGRAPH]),
                "flat style: heading {} is {} high, the paragraph {}",
                level + 1,
                flat[level],
                flat[PARAGRAPH]
            );
        }
        ensure!(
            near(flat[PARAGRAPH], default[PARAGRAPH]),
            "the paragraph is {} high flat and {} by default",
            flat[PARAGRAPH],
            default[PARAGRAPH]
        );

        Ok(())
    }
}
