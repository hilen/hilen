use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Container, Label, MarkdownView, Setup, TextAlignment, VerticalAlignment, ViewData,
        ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

/// 1 word with no space, far wider than a column.
const WORD: &str = "System.Reflection.RuntimeMethodInfo.InternalInvoke(object[],System.Exception)";

/// A code block of a forum post, a log line with no space in its middle.
const CODE: &str = "```\nat System.Reflection.RuntimeMethodInfo.InternalInvoke(System.Reflection.RuntimeMethodInfo,object,object[],System.Exception)\n```\n";

const SHORT_CODE: &str = "```\nat Invoke\n```\n";

/// Recorded with `--record-colors`.
const COLORS: &str = r"
             632    4 - #ffffff
              84   16 - #979797
             416   16 - #bababa
              44   20 - #ffffff
              96   20 - #ffffff
             376   20 - #ffffff
             436   20 - #ffffff
             460   20 - #656565
              76   24 - #ffffff
              68   32 - #ffffff
             144   32 - #000000
             240   32 - #cacaca
             396   32 - #ffffff
             432   32 - #ffffff
             524   32 - #000000
             112   36 - #ffffff
             184   36 - #000000
             204   36 - #acacac
             220   36 - #515151
             264   36 - #ffffff
             420   36 - #000000
             472   36 - #050505
             504   36 - #ffffff
             592   60 - #bbc1cc
              48   64 - #e8f0fe
             244   64 - #54575c
             480   64 - #050505
             524   64 - #a3a8b2
              80   68 - #141516
             100   68 - #cbd2de
             160   68 - #d1d8e5
             192   68 - #a3a8b2
             216   68 - #010101
             260   68 - #bbc1cc
             544   68 - #e0e8f5
             576   68 - #54575c
             416   80 - #e0e8f5
             468   80 - #4d5055
             568   80 - #878b93
             376   84 - #e8f0fe
             416   84 - #e0e8f5
             468   84 - #4d5055
             504   84 - #e8f0fe
             544   84 - #8b9098
             560   84 - #666970
             604   84 - #e8f0fe
             416   88 - #e0e8f5
             568   92 - #e0e8f5
             396   96 - #d7deeb
               4  104 - #ffffff
             420  104 - #797d84
             440  104 - #1e1f21
             396  108 - #a3a8b2
             300  136 - #ffffff
             544  156 - #ffffff
             368  160 - #ffffff
             472  160 - #ffffff
              64  172 - #070707
             144  172 - #313131
             196  172 - #545454
              40  176 - #e1e1e1
             100  176 - #cdcdcd
             220  176 - #080808
             632  180 - #ffffff
               4  212 - #ffffff
             208  224 - #444546
             412  224 - #888a8c
             520  224 - #2c2c2d
             568  224 - #010101
             112  228 - #b9bbbe
             140  228 - #b6b8bb
             196  228 - #5c5e5f
             208  228 - #444546
             228  228 - #3a3b3c
             248  228 - #b2b5b8
             316  228 - #a8aaad
             356  228 - #1d1d1e
             380  228 - #2e2e2f
             392  228 - #515254
             504  228 - #5e5f61
             536  228 - #e4e7ea
             468  232 - #eef1f5
             152  240 - #eef1f5
              68  244 - #acaeb1
             116  244 - #666769
             132  244 - #303031
             180  244 - #a8aaad
             232  244 - #dadde1
             264  244 - #1c1c1d
             304  244 - #303031
             336  244 - #a9abae
             392  244 - #020202
             420  244 - #eef1f5
             444  244 - #939497
               4  292 - #ffffff
             632  292 - #ffffff
";

const WIDTH: f32 = 260.0;
const LEFT: f32 = 24.0;
const RIGHT: f32 = 356.0;
const TOP: f32 = 56.0;
const CODE_TOP: f32 = 190.0;
const CODE_WIDTH: f32 = 592.0;

#[view]
struct LabelLongWords {
    #[init]
    plain_caption:  Label,
    breaks_caption: Label,
    code_caption:   Label,
    plain_box:      Container,
    breaks_box:     Container,
    plain:          Label,
    breaks:         Label,
    code:           MarkdownView,
}

impl LabelLongWords {
    fn caption(label: Weak<Label>, text: &str, x: f32, y: f32) {
        label
            .set_multiline(true)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_text(text);
        label.set_text_color(BLACK);
        label.set_frame((x, y, CODE_WIDTH, 36.0));
    }

    fn word(label: Weak<Label>, background: Weak<Container>, x: f32) {
        label
            .set_multiline(true)
            .set_text_size(16)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_text(WORD);
        label.set_text_color(BLACK);
        let height = label.size_for_width(WIDTH).height;
        label.set_frame((x, TOP, WIDTH, height));
        background.set_color(Color::hex("#e8f0fe"));
        background.set_frame((x, TOP, WIDTH, height));
    }
}

impl Setup for LabelLongWords {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        Self::caption(
            self.plain_caption,
            "default label\nthe word is cut at the edge of the blue box",
            LEFT,
            12.0,
        );
        Self::caption(
            self.breaks_caption,
            "breaks long words\nthe word wraps inside the box",
            RIGHT,
            12.0,
        );
        Self::caption(
            self.code_caption,
            "markdown code block, the log line wraps",
            LEFT,
            CODE_TOP - 24.0,
        );

        self.breaks.set_breaks_long_words(true);
        Self::word(self.plain, self.plain_box, LEFT);
        Self::word(self.breaks, self.breaks_box, RIGHT);

        self.code.set_text_color(BLACK);
        self.code.set_text(CODE);
        let height = self.code.height_for_width(CODE_WIDTH);
        self.code.set_frame((LEFT, CODE_TOP, CODE_WIDTH, height));
    }
}

impl ViewTest for LabelLongWords {
    fn canvas() -> (u32, u32) {
        (640, 300)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // 3 times the default, the picture is mostly small text.
        set_record_probe_count(96);
        check_colors(COLORS)?;

        let (plain, breaks, line) = from_main(move || {
            let plain = view.plain.size_for_width(WIDTH).height;
            let breaks = view.breaks.size_for_width(WIDTH).height;
            view.plain.set_text("x");
            let line = view.plain.size_for_width(WIDTH).height;
            view.plain.set_text(WORD);
            (plain, breaks, line)
        });
        ensure!(
            plain < line * 1.5,
            "the default label wrapped the word, {plain} high, 1 line is {line}"
        );
        ensure!(
            breaks >= line * 2.5,
            "with breaks the word is {breaks} high, 1 line is {line}"
        );

        let (long, short) = from_main(move || {
            let long = view.code.height_for_width(CODE_WIDTH);
            view.code.set_text(SHORT_CODE);
            let short = view.code.height_for_width(CODE_WIDTH);
            view.code.set_text(CODE);
            (long, short)
        });
        ensure!(
            long > short * 1.4,
            "the code block is {long} high with the long line, {short} with a short one"
        );

        Ok(())
    }
}
