use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    ui::{
        BLACK, Color, Container, Font, Label, MarkdownFonts, MarkdownStyle, MarkdownView, Setup,
        TextAlignment, VerticalAlignment, ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

/// Pasted terminal output: 2 empty lines first, a line that wraps, an
/// empty line in the middle, a last line with an emoji of a fallback
/// font, and a line break after it.
const TEXT: &str = r"

line 1, after 2 empty lines
line 2 is long, it wraps and takes more than 1 line of the box

line 4, after 1 empty line
      | All done!
last line 🚀 58s62ms
";

const WIDTH: f32 = 296.0;
const LEFT: f32 = 16.0;
const RIGHT: f32 = 328.0;
const TOP: f32 = 60.0;
const NEXT_ROW: f32 = 28.0;

const TEXT_SIZE: f32 = 16.0;
const LINE_HEIGHT: f32 = 22.0;

/// Puts the default style back when it goes away, also when the code that
/// runs under another style panics.
struct DefaultStyleBack;

impl Drop for DefaultStyleBack {
    fn drop(&mut self) {
        MarkdownStyle::DEFAULT.apply_globally();
    }
}

/// Runs `action` with the look of a terminal as the global style: a mono
/// font and a line pitch of its own.
fn with_terminal_style<T>(action: impl FnOnce() -> T) -> T {
    let back = DefaultStyleBack;
    MarkdownStyle {
        text_size: TEXT_SIZE,
        line_height: Some(LINE_HEIGHT),
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

/// What the layout of the one label of a plain text says, next to the
/// height the view gave for its width.
struct Laid {
    lines:      usize,
    line_pitch: f32,
    /// The room the lines take from the top of the first one to the
    /// bottom of the last one.
    needed:     f32,
    height:     f32,
}

impl Laid {
    fn of(mut text: Weak<MarkdownView>, height: f32) -> Self {
        let label = text.subviews_weak()[0].downcast::<Label>().expect("a plain text is 1 label");
        let layout = label.text_layout_for(label.text());
        let lines = layout.line_count();
        let above: f32 = (lines - 1).lossy_convert();
        Self {
            lines,
            line_pitch: layout.line_height,
            needed: above * layout.line_height + layout.ascent - layout.descent,
            height,
        }
    }

    fn check(&self, name: &str) -> Result<()> {
        ensure!(
            self.lines >= 9,
            "{name}: the layout has {} lines, the text has 8 line breaks",
            self.lines
        );
        ensure!(
            self.height >= self.needed - 0.5,
            "{name}: the measured height is {}, the {} lines with a pitch of {} need {}",
            self.height,
            self.lines,
            self.line_pitch,
            self.needed
        );
        ensure!(
            self.height <= self.needed + self.line_pitch,
            "{name}: the measured height is {}, more than 1 line over the {} the lines need",
            self.height,
            self.needed
        );
        Ok(())
    }
}

/// The same plain text 2 times, left with the default style and right as
/// a terminal shows it. The tinted box behind each text is as high as
/// `height_for_width` said, and the dark row under it starts where the
/// box ends, like the next line of a chat. Every line of the text has to
/// be inside its box.
#[view]
struct MarkdownPlainHeight {
    default_height:  f32,
    terminal_height: f32,

    #[init]
    default_caption:  Label,
    terminal_caption: Label,
    default_box:      Container,
    terminal_box:     Container,
    default_text:     MarkdownView,
    terminal_text:    MarkdownView,
    default_next:     Label,
    terminal_next:    Label,
}

impl MarkdownPlainHeight {
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
    fn show(text: Weak<MarkdownView>, background: Weak<Container>, next: Weak<Label>, x: f32) -> f32 {
        text.set_text_color(BLACK);
        text.set_plain_text(TEXT);
        let height = text.height_for_width(WIDTH);
        text.set_frame((x, TOP, WIDTH, height));
        background.set_color(Color::hex("#e8f0fe"));
        background.set_frame((x, TOP, WIDTH, height));

        next.set_text_size(13).set_alignment(TextAlignment::Left);
        next.set_text("the next row starts here");
        next.set_text_color(WHITE);
        next.set_color(Color::hex("#3b4252"));
        next.set_frame((x, TOP + height, WIDTH, NEXT_ROW));
        height
    }
}

impl Setup for MarkdownPlainHeight {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        Font::set_fallbacks([Font::get("TwemojiColr0.ttf")]);

        Self::caption(
            self.default_caption,
            "default style, pitch of the font\n2 empty lines above, 1 below",
            LEFT,
        );
        Self::caption(
            self.terminal_caption,
            "mono font, line pitch 22\n2 empty lines above, 1 below",
            RIGHT,
        );

        self.default_height = Self::show(self.default_text, self.default_box, self.default_next, LEFT);
        self.terminal_height = with_terminal_style(|| {
            Self::show(self.terminal_text, self.terminal_box, self.terminal_next, RIGHT)
        });
    }
}

impl ViewTest for MarkdownPlainHeight {
    fn canvas() -> (u32, u32) {
        (640, 460)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(120);
        check_colors(PROBES)?;
        checkpoint("every line is inside its tinted box, the last line with the rocket too")?;

        let (default, terminal) = from_main(move || {
            (
                Laid::of(view.default_text, view.default_height),
                Laid::of(view.terminal_text, view.terminal_height),
            )
        });
        default.check("default style")?;
        terminal.check("terminal style")?;

        // With a line pitch of its own the text is whole line boxes high.
        let boxes: f32 = terminal.lines.lossy_convert();
        ensure!(
            (terminal.height - boxes * LINE_HEIGHT).abs() < 0.01,
            "terminal style: the measured height is {}, {} lines of {LINE_HEIGHT} are {}",
            terminal.height,
            terminal.lines,
            boxes * LINE_HEIGHT
        );

        Ok(())
    }
}

const PROBES: &str = r"
     632    4 - #ffffff
     160   16 - #cbcbcb
      48   20 - #282828
      84   20 - #363636
     116   20 - #e4e4e4
     156   20 - #eaeaea
     196   20 - #ffffff
     344   20 - #e1e1e1
     368   20 - #000000
     388   20 - #ffffff
     436   20 - #ffffff
     468   24 - #e6e6e6
     476   24 - #e6e6e6
     144   36 - #cecece
     156   36 - #282828
     456   36 - #cecece
     468   36 - #282828
     496   36 - #ffffff
      40  100 - #000000
      96  100 - #e0e8f5
     124  100 - #000000
     160  100 - #c8cfdb
      96  104 - #e0e8f5
     348  112 - #6b6f76
     560  112 - #000000
     568  112 - #aeb4be
      36  116 - #6c7077
     364  116 - #82878e
     396  116 - #8c9199
     444  116 - #333538
     512  116 - #484a4f
     516  116 - #d3dae7
     588  116 - #2e3033
      44  120 - #484a4f
     100  120 - #000000
     140  120 - #9ca2ab
     208  120 - #83888f
     304  120 - #61656b
      36  136 - #6c7077
      68  136 - #afb5bf
      80  136 - #6e7279
     348  136 - #6b6f76
     444  136 - #6b6f76
     476  136 - #000000
     364  140 - #82878e
     416  140 - #000000
     460  140 - #82878e
     596  140 - #8a8f97
     556  144 - #80858c
     348  160 - #333538
     364  160 - #868a92
     452  160 - #d8dfec
     480  160 - #010101
     532  160 - #a1a7b0
     572  160 - #010101
     160  164 - #c8cfdb
      40  168 - #010101
      80  168 - #9fa5ae
      92  168 - #dbe3f0
      96  168 - #e0e8f5
     120  168 - #dbe3f0
     128  168 - #757980
     160  168 - #c8cfdb
     176  168 - #dbe3f0
      64  184 - #1b1c1e
     100  184 - #767a81
     356  184 - #8a8f97
     396  184 - #e8f0fe
      52  200 - #1c1d1f
      64  200 - #e0e8f5
      76  200 - #222325
     104  200 - #e8f0fe
     140  200 - #c5ccd8
     136  204 - #bbc1cc
     200  224 - #3d4353
     356  224 - #e7effd
     484  224 - #81868d
     576  224 - #000000
     304  228 - #3b4252
     512  228 - #484a4f
     516  228 - #d3dae7
     532  228 - #e8f0fe
     252  232 - #3b4252
      40  240 - #3b4252
      64  240 - #3b4252
      84  240 - #3b4252
     128  240 - #3b4252
     148  240 - #c1c3c8
     436  244 - #a1a7b0
     180  248 - #3b4252
     224  248 - #3b4252
     280  248 - #3b4252
     436  248 - #a1a7b0
     464  248 - #e8f0fe
     492  248 - #2e3033
     348  268 - #6b6f76
     396  268 - #6b6f76
     504  268 - #e8f0fe
     412  272 - #82878e
     480  272 - #e8f0fe
     548  304 - #3b4252
     432  312 - #3b4252
     388  316 - #aeb1b8
     352  320 - #3b4252
     404  320 - #3b4252
     472  320 - #3b4252
     328  328 - #3b4252
     512  328 - #3b4252
     580  328 - #3b4252
     620  328 - #3b4252
     216  336 - #ffffff
       4  344 - #ffffff
     112  372 - #ffffff
     448  412 - #ffffff
     632  424 - #ffffff
       4  452 - #ffffff
     136  452 - #ffffff
     220  452 - #ffffff
     360  452 - #ffffff
     540  452 - #ffffff
";
