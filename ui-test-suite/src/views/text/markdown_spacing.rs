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

/// A heading, 2 paragraphs that wrap and a list of 3 items, the second
/// item wraps too.
const TEXT: &str = r"## Gaps and line pitch

The first paragraph is long enough to wrap, so the space between its lines shows next to the space under it.

The second paragraph wraps as well, it starts one block gap below the last line of the first one.

- The first item
- The second item is long, it wraps and shows the line pitch inside an item
- The third item
";

/// Recorded with `--record-colors`, both texts side by side.
const COLORS: &str = r"
             632    4 - #ffffff
             540   16 - #535353
              48   20 - #282828
              84   20 - #363636
             368   20 - #9c9c9c
             380   20 - #989898
             464   20 - #ffffff
             180   32 - #212121
             224   32 - #b2b2b2
             352   32 - #4b4b4b
             492   32 - #212121
             536   32 - #b2b2b2
              32   36 - #e3e3e3
             112   36 - #cacaca
             128   36 - #d6d6d6
             136   36 - #626262
             196   36 - #cecece
             228   36 - #eaeaea
             244   36 - #c1c1c1
             264   36 - #cecece
             288   36 - #000000
             344   36 - #e3e3e3
             396   36 - #ffffff
             424   36 - #cacaca
             444   36 - #9e9e9e
             508   36 - #cecece
             540   36 - #eaeaea
             120   68 - #959aa3
             432   68 - #959aa3
              56   72 - #202123
             100   72 - #c5cbd7
             136   72 - #adb3bd
             172   72 - #c9d0dc
             192   72 - #010101
             364   72 - #010101
             368   72 - #202123
             412   72 - #c5cbd7
             448   72 - #adb3bd
             484   72 - #c9d0dc
              40   76 - #000000
             368   76 - #202123
             144   96 - #9ca2ab
             168   96 - #6c7077
              60  100 - #5a5d63
              80  100 - #1b1c1e
             120  100 - #000000
             232  100 - #e8f0fe
             256  100 - #e8f0fe
              44  104 - #e8f0fe
             480  108 - #6c7077
             348  112 - #27282b
             372  112 - #5a5d63
             392  112 - #1b1c1e
             432  112 - #000000
             528  112 - #e8f0fe
             596  112 - #000000
             160  116 - #e0e8f5
             192  116 - #b1b8c2
             268  116 - #000000
             288  116 - #e8f0fe
             160  120 - #e0e8f5
             504  124 - #b1b8c2
             364  128 - #1b1c1e
             444  128 - #343639
             468  128 - #1d1e20
             472  128 - #e0e8f5
             504  128 - #b1b8c2
             520  128 - #1d1e20
             552  128 - #e8f0fe
             112  132 - #d9e1ee
             144  136 - #010101
             424  144 - #d9e1ee
             456  144 - #000000
             256  156 - #6c7077
             108  160 - #dfe7f4
             124  160 - #6c7077
             248  160 - #1d1e20
             256  160 - #6c7077
             108  164 - #dfe7f4
             156  164 - #6a6e75
             228  172 - #6b6f76
              40  176 - #8f949c
              80  176 - #b1b8c2
             108  176 - #55585d
             184  176 - #e8f0fe
             204  176 - #60646a
             228  176 - #6b6f76
             256  176 - #e1e8f6
             288  176 - #3a3c40
             420  180 - #dfe7f4
             568  180 - #6c7077
             356  184 - #e8f0fe
             420  184 - #dfe7f4
             436  184 - #6c7077
             456  184 - #55585d
             476  184 - #55585d
              60  192 - #888d95
             540  196 - #6b6f76
             568  196 - #e1e8f6
             364  200 - #55585d
             392  200 - #b1b8c2
             520  200 - #83888f
             568  200 - #e1e8f6
             576  200 - #bcc3ce
             600  200 - #3a3c40
             360  216 - #e8f0fe
             372  216 - #888d95
             408  216 - #767a81
              56  220 - #27282b
              72  220 - #959aa3
              84  220 - #5a5d63
             100  220 - #1b1c1e
             132  220 - #3c3e42
              56  240 - #27282b
             112  244 - #e8f0fe
             148  244 - #d2d9e6
             188  244 - #e8f0fe
             208  244 - #26282a
             280  244 - #000000
             392  252 - #5a5d63
             412  252 - #1b1c1e
             168  256 - #c9d0dc
              64  260 - #a5aab4
             128  260 - #9ca2ab
             168  260 - #c9d0dc
             232  260 - #bcc3ce
             256  260 - #ccd3df
             264  260 - #000000
             368  268 - #27282b
             456  272 - #52555a
             460  272 - #d2d9e6
             520  272 - #26282a
             592  272 - #000000
              84  276 - #6c7077
              72  280 - #c4cad6
              92  280 - #56585e
             128  280 - #b5bbc6
             480  284 - #c9d0dc
             376  288 - #a5aab4
             432  288 - #56585e
             444  288 - #1c1d1f
             476  288 - #e8f0fe
             500  288 - #030303
             544  288 - #bcc3ce
             568  288 - #ccd3df
             396  300 - #6c7077
             368  304 - #27282b
             628  340 - #ffffff
             184  348 - #ffffff
               4  352 - #ffffff
             348  364 - #ffffff
             268  368 - #ffffff
             548  380 - #ffffff
             428  388 - #ffffff
             104  392 - #ffffff
             632  416 - #ffffff
             348  444 - #ffffff
               4  452 - #ffffff
             200  452 - #ffffff
             496  452 - #ffffff
            ";

const BLOCK_GAP: f32 = 22.0;
const ITEM_GAP: f32 = 2.0;
const LINE_HEIGHT: f32 = 16.0;

const SPACED: MarkdownStyle = MarkdownStyle {
    block_gap: BLOCK_GAP,
    item_gap: ITEM_GAP,
    line_height: Some(LINE_HEIGHT),
    ..MarkdownStyle::DEFAULT
};

const WIDTH: f32 = 296.0;
const LEFT: f32 = 16.0;
const RIGHT: f32 = 328.0;
const TOP: f32 = 60.0;

/// The blocks of `TEXT` in the order the view makes them: the heading and
/// the 2 paragraphs, then the 3 markers, then the 3 item texts.
const HEADING: usize = 0;
const FIRST_PARAGRAPH: usize = 1;
const SECOND_PARAGRAPH: usize = 2;
const FIRST_MARKER: usize = 3;
const FIRST_ITEM: usize = 6;
const VIEWS: usize = 9;

/// Puts the default style back when it goes away, also when the code that
/// runs under another style panics. The style is global, a test that left
/// its own behind would draw every markdown view after it with it.
struct DefaultStyleBack;

impl Drop for DefaultStyleBack {
    fn drop(&mut self) {
        MarkdownStyle::DEFAULT.apply_globally();
    }
}

/// Runs `action` with the spaced style as the global one.
fn with_spaced_style<T>(action: impl FnOnce() -> T) -> T {
    let back = DefaultStyleBack;
    SPACED.apply_globally();
    let result = action();
    drop(back);
    result
}

/// The top and the bottom of every view a markdown view made, and the
/// height it gave for its width.
struct Laid {
    frames: Vec<(f32, f32)>,
    height: f32,
}

impl Laid {
    fn of(text: Weak<MarkdownView>, height: f32) -> Self {
        Self {
            frames: text.subviews().iter().map(|view| (view.y(), view.max_y())).collect(),
            height,
        }
    }

    fn gap(&self, above: usize) -> f32 {
        self.frames[above + 1].0 - self.frames[above].1
    }

    fn height_of(&self, index: usize) -> f32 {
        self.frames[index].1 - self.frames[index].0
    }

    /// The same numbers in the layout and in the measured height: the
    /// gaps are the ones of the style, and the lowest view ends where
    /// `height_for_width` says the text ends.
    fn check(&self, name: &str, block_gap: f32, item_gap: f32) -> Result<()> {
        ensure!(
            self.frames.len() == VIEWS,
            "{name}: {} views, not {VIEWS}",
            self.frames.len()
        );
        for above in [HEADING, FIRST_PARAGRAPH] {
            let gap = self.gap(above);
            ensure!(
                near(gap, block_gap),
                "{name}: the gap under block {above} is {gap}, not {block_gap}"
            );
        }
        for item in 0..2 {
            // An item is as high as its text and at least as high as its
            // marker, which takes 1 line.
            let end = self.frames[FIRST_ITEM + item].1.max(self.frames[FIRST_MARKER + item].1);
            let gap = self.frames[FIRST_ITEM + item + 1].0 - end;
            ensure!(
                near(gap, item_gap),
                "{name}: the gap under item {item} is {gap}, not {item_gap}"
            );
        }
        let bottom = self.frames.iter().map(|frame| frame.1).fold(0.0, f32::max);
        ensure!(
            near(bottom, self.height),
            "{name}: the lowest view ends at {bottom}, the measured height is {}",
            self.height
        );
        Ok(())
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// The same text 2 times, left with the default style and right with a
/// style that sets the gap between blocks, the gap between list items and
/// the line pitch. The tinted box behind each text is as high as
/// `height_for_width` said, so a text that ends above or below its box
/// shows a layout that does not agree with the measure.
#[view]
struct MarkdownSpacing {
    default_height: f32,
    spaced_height:  f32,

    #[init]
    default_caption: Label,
    spaced_caption:  Label,
    default_box:     Container,
    spaced_box:      Container,
    default_text:    MarkdownView,
    spaced_text:     MarkdownView,
}

impl MarkdownSpacing {
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
    fn show(text: Weak<MarkdownView>, background: Weak<Container>, x: f32) -> f32 {
        text.set_text_color(BLACK);
        text.set_text(TEXT);
        let height = text.height_for_width(WIDTH);
        text.set_frame((x, TOP, WIDTH, height));
        background.set_color(Color::hex("#e8f0fe"));
        background.set_frame((x, TOP, WIDTH, height));
        height
    }
}

impl Setup for MarkdownSpacing {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);

        Self::caption(
            self.default_caption,
            "default style\nblock gap 10, item gap 4, line pitch of the font",
            LEFT,
        );
        Self::caption(
            self.spaced_caption,
            "tight lines, wide gap between blocks\nblock gap 22, item gap 2, line pitch 16",
            RIGHT,
        );

        self.default_height = Self::show(self.default_text, self.default_box, LEFT);
        self.spaced_height = with_spaced_style(|| Self::show(self.spaced_text, self.spaced_box, RIGHT));
    }
}

impl ViewTest for MarkdownSpacing {
    fn canvas() -> (u32, u32) {
        (640, 460)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // 5 times the default, the picture is mostly small text.
        set_record_probe_count(160);
        check_colors(COLORS)?;

        let (default, spaced) = from_main(move || {
            (
                Laid::of(view.default_text, view.default_height),
                Laid::of(view.spaced_text, view.spaced_height),
            )
        });

        let style = MarkdownStyle::DEFAULT;
        default.check("default style", style.block_gap, style.item_gap)?;
        spaced.check("spaced style", BLOCK_GAP, ITEM_GAP)?;

        // Every line of running text is 1 line box high.
        for index in [FIRST_PARAGRAPH, SECOND_PARAGRAPH, FIRST_ITEM + 1] {
            let lines = spaced.height_of(index) / LINE_HEIGHT;
            ensure!(
                lines >= 2.0 && near(lines, lines.round()),
                "view {index} is {} high, not whole lines of {LINE_HEIGHT}",
                spaced.height_of(index)
            );
            ensure!(
                spaced.height_of(index) < default.height_of(index),
                "view {index} is not tighter than with the pitch of the font"
            );
        }
        // A heading has bigger text, the pitch grows with it.
        let heading = spaced.height_of(HEADING);
        ensure!(
            heading > LINE_HEIGHT,
            "the heading is {heading} high, the body pitch {LINE_HEIGHT} squeezed it"
        );

        // The style of the right text is gone again, a view laid out now
        // gets the default numbers.
        let again = from_main(move || {
            let mut text = view.add_view::<MarkdownView>();
            text.set_text(TEXT);
            let height = text.height_for_width(WIDTH);
            text.remove_from_superview();
            height
        });
        ensure!(
            near(again, default.height),
            "a new view measures {again}, the default style gives {}",
            default.height
        );

        // Measured again under the style, the right text keeps its height.
        let measured = from_main(move || {
            with_spaced_style(|| {
                let narrow = view.spaced_text.height_for_width(WIDTH / 2.0);
                let wide = view.spaced_text.height_for_width(WIDTH);
                (narrow, wide)
            })
        });
        ensure!(
            measured.0 > measured.1,
            "half the width gives {}, not more than {}",
            measured.0,
            measured.1
        );
        ensure!(
            near(measured.1, spaced.height),
            "the second measure gives {}, the first gave {}",
            measured.1,
            spaced.height
        );

        Ok(())
    }
}
