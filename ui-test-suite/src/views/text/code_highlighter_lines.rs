use std::ops::Range;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        CodeHighlighter, Color, DynamicColor, Font, Label, MarkdownStyle, MarkdownView, Setup, TextAlignment,
        Theme, ThemeMode, UIColor, VerticalAlignment, ViewData, ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

const BACKGROUND: DynamicColor = DynamicColor::new(Color::hex("#ffffff"), Color::hex("#14171c"));
const ADDED: DynamicColor = DynamicColor::new(Color::hex("#e3f5e8"), Color::hex("#16301f"));
const REMOVED: DynamicColor = DynamicColor::new(Color::hex("#fde8e8"), Color::hex("#3a1b1b"));

const LEFT: f32 = 16.0;
const WIDTH: f32 = 608.0;
const MARK_WIDTH: f32 = 20.0;
const NAME_WIDTH: f32 = 130.0;
const ROW: f32 = 18.0;
const CODE_SIZE: f32 = 13.0;

const DIFF_TOP: f32 = 76.0;
const ALONE_TOP: f32 = 270.0;
const MARKDOWN_TOP: f32 = 464.0;
const FILES_TOP: f32 = 660.0;

/// Which file of a diff a line belongs to.
#[derive(Clone, Copy, PartialEq)]
enum Side {
    Both,
    Old,
    New,
}

/// A diff of a Rust file. The second line reads as code and sits inside
/// a block comment, so it shows whether the state of the line before
/// reached it.
const DIFF: [(Side, &str); 9] = [
    (Side::Both, "/* The sum of 2 numbers."),
    (Side::Both, "   let total = a + b; is the whole body."),
    (Side::Both, "   It never fails. */"),
    (Side::Old, "fn sum(a: u32) -> u32 {"),
    (Side::Old, "    a + 1 // adds one"),
    (Side::New, "fn sum(a: u32, b: u32) -> u32 {"),
    (Side::New, r#"    println!("{a} + {b}");"#),
    (Side::New, "    a + b // adds both"),
    (Side::Both, "}"),
];

/// The line of `DIFF` inside the block comment.
const INSIDE_COMMENT: usize = 1;

/// A file name and a line of its language. The last name has no language.
const FILES: [(&str, &str); 4] = [
    ("Makefile", "build: main.o # links the app"),
    ("web/x.tsx", "const row = <Row count={3} />; // a view"),
    ("conf.toml", r#"name = "banda" # the app"#),
    ("notes.unknown", "let plain = 1; // no language, no color"),
];

const LIGHT_STATUS: &str = r"light theme. In the first diff the 3 comment lines are all gray.
In the second one the line `let total` is colored as code.";
const DARK_STATUS: &str = r"dark theme. The same picture in the dark colors of the style,
no label got new color runs.";

type Runs = Vec<(Range<usize>, UIColor)>;

/// Lines of code, each in its own label, colored by `CodeHighlighter`.
/// From the top: a diff with the state carried from line to line, the
/// same diff with every line colored alone, the new side of the diff as
/// a code block of a `MarkdownView` for the same colors, and a line per
/// file name.
#[view]
struct CodeHighlighterLines {
    carried: Vec<Weak<Label>>,
    alone:   Vec<Weak<Label>>,
    files:   Vec<Weak<Label>>,

    #[init]
    status:           Label,
    carried_caption:  Label,
    alone_caption:    Label,
    markdown_caption: Label,
    files_caption:    Label,
    markdown:         MarkdownView,
}

impl CodeHighlighterLines {
    fn caption(label: Weak<Label>, text: &str, y: f32) {
        label.set_text_size(13).set_alignment(TextAlignment::Left);
        label.set_text(text);
        label.set_text_color(MarkdownStyle::DEFAULT.dim_text);
        label.set_frame((LEFT, y, WIDTH, ROW));
    }

    fn small(self: Weak<Self>, text: &str, frame: (f32, f32, f32, f32), color: UIColor) -> Weak<Label> {
        let label = self.add_view::<Label>();
        label.set_text_size(CODE_SIZE).set_alignment(TextAlignment::Left);
        label.set_font(Font::mono());
        label.set_text(text);
        label.set_text_color(MarkdownStyle::DEFAULT.dim_text);
        label.set_color(color);
        label.set_frame(frame);
        label
    }

    /// 1 line of code in its own label, the way a table cell shows it.
    fn code(self: Weak<Self>, text: &str, runs: Runs, x: f32, y: f32, color: UIColor) -> Weak<Label> {
        let label = self.small(text, (x, y, LEFT + WIDTH - x, ROW), color);
        label.set_text_color(MarkdownStyle::DEFAULT.text);
        label.set_color_runs(runs);
        label
    }

    fn diff(self: Weak<Self>, top: f32, mut runs: impl FnMut(Side, &str) -> Runs) -> Vec<Weak<Label>> {
        let mut y = top;
        let mut labels = vec![];
        for (side, line) in DIFF {
            let (mark, color) = match side {
                Side::Both => (" ", MarkdownStyle::DEFAULT.code_background),
                Side::Old => ("-", REMOVED.into()),
                Side::New => ("+", ADDED.into()),
            };
            self.small(mark, (LEFT, y, MARK_WIDTH, ROW), color)
                .set_text_color(MarkdownStyle::DEFAULT.text);
            labels.push(self.code(line, runs(side, line), LEFT + MARK_WIDTH, y, color));
            y += ROW;
        }
        labels
    }

    /// The old file and the new file of a diff are 2 texts, each with a
    /// highlighter of its own. A line of both goes into both.
    fn carried_diff(self: Weak<Self>) -> Vec<Weak<Label>> {
        let mut old = CodeHighlighter::for_file("src/sum.rs").expect("Rust has no language");
        let mut new = old.clone();
        self.diff(DIFF_TOP, |side, line| match side {
            Side::Old => old.color_runs(line),
            Side::New => new.color_runs(line),
            Side::Both => {
                old.color_runs(line);
                new.color_runs(line)
            }
        })
    }

    fn alone_diff(self: Weak<Self>) -> Vec<Weak<Label>> {
        let mut rust = CodeHighlighter::for_file("src/sum.rs").expect("Rust has no language");
        self.diff(ALONE_TOP, |_, line| {
            rust.reset();
            rust.color_runs(line)
        })
    }

    fn file_rows(self: Weak<Self>) -> Vec<Weak<Label>> {
        let style = MarkdownStyle::DEFAULT;
        let mut y = FILES_TOP;
        let mut labels = vec![];
        for (name, line) in FILES {
            self.small(name, (LEFT, y, NAME_WIDTH, ROW), BACKGROUND.into());
            let runs = match CodeHighlighter::for_file(name) {
                Some(mut highlighter) => highlighter.color_runs(line),
                None => vec![],
            };
            labels.push(self.code(line, runs, LEFT + NAME_WIDTH, y, style.code_background));
            y += ROW + 4.0;
        }
        labels
    }

    fn show_markdown(self: Weak<Self>) {
        let lines: Vec<&str> = DIFF
            .iter()
            .filter(|(side, _)| *side != Side::Old)
            .map(|(_, line)| *line)
            .collect();
        self.markdown.set_text(&format!("```rust\n{}\n```\n", lines.join("\n")));
        let height = self.markdown.height_for_width(WIDTH);
        self.markdown.set_frame((LEFT, MARKDOWN_TOP, WIDTH, height));
    }
}

impl Setup for CodeHighlighterLines {
    fn setup(mut self: Weak<Self>) {
        self.set_color(BACKGROUND);

        self.status
            .set_multiline(true)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        self.status.set_text(LIGHT_STATUS);
        self.status.set_text_color(MarkdownStyle::DEFAULT.text);
        self.status.set_frame((LEFT, 10.0, WIDTH, 36.0));

        Self::caption(
            self.carried_caption,
            "a diff, 1 label per line, the state goes from line to line",
            DIFF_TOP - 22.0,
        );
        Self::caption(
            self.alone_caption,
            "the same diff, every line colored alone",
            ALONE_TOP - 22.0,
        );
        Self::caption(
            self.markdown_caption,
            "the new side as a code block of MarkdownView, the same colors",
            MARKDOWN_TOP - 22.0,
        );
        Self::caption(
            self.files_caption,
            "the language comes from the file name",
            FILES_TOP - 22.0,
        );

        self.carried = self.carried_diff();
        self.alone = self.alone_diff();
        self.files = self.file_rows();
        self.show_markdown();
    }
}

impl ViewTest for CodeHighlighterLines {
    fn canvas() -> (u32, u32) {
        (640, 760)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The picture is small text, the default leaves most lines unpinned.
        set_record_probe_count(240);
        from_main(|| Theme::set_mode(ThemeMode::Light));

        let runs_of = |labels: &[Weak<Label>]| -> Vec<usize> {
            labels.iter().map(|label| label.color_runs_len()).collect()
        };
        let (carried, alone, files) =
            from_main(move || (runs_of(&view.carried), runs_of(&view.alone), runs_of(&view.files)));

        // Inside the comment the whole line is 1 comment run. Alone the
        // same line is code, with a keyword and more.
        ensure!(
            carried[INSIDE_COMMENT] == 1,
            "the line inside the comment has {} runs with the state carried, not 1",
            carried[INSIDE_COMMENT]
        );
        ensure!(
            alone[INSIDE_COMMENT] > 1,
            "the line inside the comment has {} runs alone, it was not colored as code",
            alone[INSIDE_COMMENT]
        );
        // A line of code outside the comment gets the same runs both ways.
        ensure!(
            carried[3..] == alone[3..],
            "the lines after the comment differ: {carried:?} carried, {alone:?} alone"
        );
        let (known, unknown) = files.split_at(FILES.len() - 1);
        ensure!(
            known.iter().all(|runs| *runs > 1),
            "a file with a language got no colors: {known:?}"
        );
        ensure!(unknown == [0], "a file with no language got colors: {unknown:?}");

        check_colors(CHECK_1)?;

        // The runs are pairs of the style, a theme switch needs no new call.
        from_main(move || {
            Theme::set_mode(ThemeMode::Dark);
            view.status.set_text(DARK_STATUS);
        });
        let dark = from_main(move || runs_of(&view.carried));
        ensure!(dark == carried, "the theme switch changed the runs");
        check_colors(CHECK_2)?;

        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}

const CHECK_1: &str = r"
             428    4 - #ffffff
             504    4 - #ffffff
             580    4 - #ffffff
             192   16 - #babbbd
              36   20 - #46484e
              60   20 - #ffffff
              84   20 - #35383e
              88   20 - #1a1d24
             108   20 - #1a1d24
             112   20 - #35383e
             144   20 - #1b1e25
             176   20 - #ccccce
             244   20 - #b6b7b9
             268   20 - #a8a9ac
             316   20 - #8c8e91
             352   20 - #1a1d24
             212   28 - #ffffff
             164   32 - #7d7e82
             188   32 - #1a1d24
             228   32 - #5d5f64
             292   32 - #1a1d24
              48   60 - #979ca6
              72   60 - #dedfe3
             132   60 - #707684
             244   60 - #6b7280
             104   64 - #8d939d
             124   64 - #6b7280
             160   64 - #ffffff
             204   64 - #d3d6da
             216   64 - #6b7280
             268   64 - #6b7280
             316   64 - #707684
             468   68 - #ffffff
              80   80 - #cfd3d9
             548   84 - #eef1f5
             120   88 - #6b7280
             148   88 - #a7acb6
             196   88 - #7d8491
              80  100 - #d4d8de
             256  100 - #6b7280
             632  100 - #ffffff
             136  104 - #9fa5af
             156  104 - #c3c7ce
             188  104 - #6b7280
             228  104 - #babfc7
             288  104 - #caced4
             328  104 - #eef1f5
             172  120 - #bdc2c9
             100  124 - #6b7280
             132  124 - #777d8b
             148  124 - #a7acb6
              92  140 - #518ede
             112  140 - #1d1f26
             136  140 - #a87ee2
             172  140 - #9a9093
             208  140 - #fde8e8
              16  168 - #e3f5e8
             320  168 - #e3f5e8
             392  168 - #e3f5e8
             488  168 - #e3f5e8
             544  168 - #e3f5e8
             608  168 - #e3f5e8
              88  176 - #a5cde4
              92  176 - #4992de
             112  176 - #1c2026
             136  176 - #a082e2
             172  176 - #e3f5e8
             208  176 - #e3f5e8
             220  176 - #1b1e25
             256  176 - #8250df
             260  176 - #a184e2
             264  176 - #e3f5e8
              84  196 - #4b93de
             108  196 - #247adc
             112  196 - #3283dd
             132  196 - #a6cee4
             136  196 - #3e8bdd
             448  200 - #e3f5e8
             116  212 - #5a6262
             164  212 - #e3f5e8
             172  212 - #e3f5e8
             196  212 - #e3f5e8
             204  212 - #e3f5e8
             300  216 - #e3f5e8
             352  216 - #e3f5e8
             508  216 - #e3f5e8
             576  216 - #e3f5e8
             632  224 - #ffffff
               4  252 - #ffffff
             152  256 - #797f8c
             236  256 - #d2d4d9
              84  280 - #9399a4
             120  280 - #6b7280
             148  280 - #a7acb6
             196  280 - #7d8491
             212  280 - #787e8b
             536  280 - #eef1f5
             408  284 - #eef1f5
             304  292 - #4f5258
             156  296 - #8554e0
             188  296 - #8250df
             256  296 - #1a1d24
             112  300 - #eef1f5
             256  300 - #1a1d24
             292  300 - #494c53
             100  316 - #1a1d24
             132  316 - #2d3037
             148  316 - #7b7e84
             164  316 - #c9ccd1
             208  332 - #fde8e8
              92  336 - #518ede
             136  336 - #a87ee2
             172  352 - #fde8e8
             196  352 - #fde8e8
             212  352 - #797e8a
             364  360 - #e3f5e8
             456  360 - #e3f5e8
             532  360 - #e3f5e8
             620  360 - #e3f5e8
              16  364 - #e3f5e8
             148  368 - #e3f5e8
             208  368 - #e3f5e8
             272  368 - #e3f5e8
              92  372 - #4992de
             136  372 - #a082e2
             212  372 - #e3f5e8
              84  388 - #4b93de
             104  388 - #e3f5e8
             108  388 - #247adc
             112  388 - #3283dd
             116  388 - #e3f5e8
             120  388 - #e3f5e8
             132  388 - #a6cee4
             148  388 - #1a1d24
             220  388 - #e3f5e8
              84  392 - #4b93de
             320  396 - #e3f5e8
             172  400 - #e3f5e8
             420  412 - #e3f5e8
             500  412 - #e3f5e8
             568  412 - #e3f5e8
             620  412 - #e3f5e8
             192  448 - #b5b8bf
             240  448 - #979ca6
             252  448 - #e1e2e5
             312  448 - #6b7280
              64  452 - #d3d6da
              96  452 - #6b7280
             140  452 - #ffffff
             176  452 - #c8cbd0
             204  452 - #ffffff
             252  452 - #e1e2e5
             276  452 - #a7abb3
             292  452 - #d3d6da
             320  452 - #6e7582
             352  452 - #e4e6e8
             372  452 - #ffffff
             388  452 - #ffffff
              84  484 - #ccd0d7
             184  484 - #6b7280
             204  484 - #aeb3bc
             224  484 - #6b7280
             448  492 - #eef1f5
             124  496 - #eef1f5
             548  496 - #eef1f5
               4  500 - #ffffff
             100  500 - #7a818e
             148  500 - #a0a5af
             256  500 - #7a818e
             272  500 - #ccd0d7
             312  500 - #babfc7
             332  500 - #7a808d
             232  504 - #868c98
              84  512 - #c1c6cd
              92  516 - #6b7280
             632  528 - #ffffff
             176  532 - #32353c
             280  532 - #eef1f5
              72  536 - #b297e9
             100  536 - #5998e3
             176  536 - #32353c
             204  536 - #8250df
             220  536 - #eef1f5
             132  548 - #0a6ada
             108  552 - #6ba3e6
             124  552 - #adcbed
             132  552 - #0a6ada
             496  564 - #eef1f5
             220  568 - #eef1f5
             384  592 - #eef1f5
             576  592 - #eef1f5
               4  600 - #ffffff
              64  648 - #d2d4d9
              84  648 - #b9bcc3
             104  648 - #d3d6da
             140  648 - #d3d6da
             164  648 - #ffffff
             192  648 - #b8bbc2
             244  648 - #b7bac1
             444  652 - #ffffff
             528  660 - #eef1f5
             188  664 - #428be1
             632  664 - #ffffff
              48  668 - #d5d7db
             172  668 - #d4e2f2
             176  672 - #7daee8
             220  672 - #2f7d33
             236  672 - #2f7d33
             308  672 - #6c7280
             348  672 - #d7dbe0
             264  688 - #89b9c7
              52  692 - #ffffff
             172  692 - #eef1f5
             196  692 - #8250df
             220  692 - #eef1f5
             244  692 - #cabcee
             264  692 - #89b9c7
             316  692 - #ba6422
             328  692 - #d1a380
             204  712 - #20232a
              48  716 - #d3d6da
              88  716 - #d2d4d8
             168  716 - #4492a9
             180  716 - #e3ebf0
             184  716 - #368aa2
             304  716 - #959ba6
             260  732 - #82858a
              60  736 - #7a808c
              80  736 - #6b7280
             104  736 - #b8bcc2
             180  736 - #5b5e64
             204  736 - #9fa2a7
             220  736 - #4a4d54
             244  736 - #a8abb0
             308  736 - #1b1e25
             340  736 - #c4c7cb
             384  736 - #2f3238
             416  736 - #eef1f5
             460  736 - #82858a
             568  752 - #ffffff
            ";

const CHECK_2: &str = r"
             576    4 - #14171c
              52   16 - #d1d4d9
             364   16 - #56595e
              80   20 - #e4e7ec
              84   20 - #e9ecf1
              88   20 - #b8bbc0
             112   20 - #acafb4
             148   20 - #a7aaaf
             152   20 - #dadde2
             172   20 - #acafb4
             208   20 - #eceff4
             296   20 - #808388
              32   32 - #2a2d32
              76   32 - #64676c
             104   32 - #eceff4
             168   32 - #b4b7bc
             172   32 - #e7eaef
              48   60 - #727985
              72   60 - #32363e
             132   60 - #969fad
             244   60 - #9aa3b2
             104   64 - #7b838f
             124   64 - #9aa3b2
             132   64 - #969fad
             148   64 - #3b4048
             172   64 - #3b4048
             204   64 - #3b4048
             216   64 - #9aa3b2
             244   64 - #9aa3b2
             248   64 - #99a2b1
             268   64 - #9aa3b2
             284   64 - #3a3f46
             316   64 - #969fad
             476   64 - #14171c
              80   80 - #292f37
              84   88 - #5b6370
             128   88 - #5a626f
             148   88 - #4a515d
             196   88 - #6d7684
             556   92 - #0f1319
             256  100 - #7c8696
              88  104 - #717b8a
             136  104 - #505864
             156  104 - #333942
             188  104 - #7c8696
             228  104 - #3a414b
             288  104 - #2d333c
             344  104 - #0f1319
              80  116 - #292f37
             420  116 - #0f1319
             172  120 - #383e48
             100  124 - #7c8696
             132  124 - #727c8b
             148  124 - #4a515d
             632  128 - #14171c
              60  140 - #47242d
              92  140 - #5683b1
             112  140 - #eaedf1
             136  140 - #9b5ca2
             172  140 - #877779
             208  140 - #3a1b1b
             204  160 - #3a1b1b
              60  176 - #273731
              92  176 - #4b8ab2
             112  176 - #e9edf1
             136  176 - #9062a3
             172  176 - #16301f
             196  176 - #16301f
             220  176 - #ebeef3
             256  176 - #c678dd
             260  176 - #8e61a1
             464  180 - #16301f
             556  180 - #16301f
             216  192 - #263925
              84  196 - #4a89b0
             108  196 - #58a0d6
             112  196 - #5397c8
             136  196 - #4f90bc
             216  196 - #263925
             364  208 - #16301f
             116  212 - #a8b2b0
             172  212 - #16301f
             196  212 - #16301f
               4  244 - #14171c
             152  256 - #8d96a4
             236  256 - #3c4149
             632  264 - #14171c
             528  268 - #14171c
              84  276 - #5b6370
             120  280 - #7c8696
             128  280 - #5a626f
             148  280 - #4a515d
             196  280 - #6d7684
             212  280 - #717b8a
             304  292 - #b5b8bd
              80  296 - #33273f
             156  296 - #c175d8
             188  296 - #c678dd
             256  296 - #eceff4
             112  300 - #0f1319
             256  300 - #eceff4
             260  300 - #92959b
             292  300 - #bbbec3
             100  316 - #eceff4
             132  316 - #d8dbe0
             148  316 - #878a90
             172  316 - #61656b
             436  320 - #0f1319
              60  332 - #47242d
             208  332 - #3a1b1b
              60  336 - #47242d
              92  336 - #5683b1
             136  336 - #9b5ca2
             116  352 - #3a1b1b
             172  352 - #3a1b1b
             212  352 - #767c8a
             620  352 - #3a1b1b
             360  364 - #16301f
              60  368 - #273731
             196  368 - #16301f
             272  368 - #16301f
              60  372 - #273731
              92  372 - #4b8ab2
             136  372 - #9062a3
             216  384 - #263925
              84  388 - #4a89b0
             108  388 - #58a0d6
             136  388 - #4f90bc
             148  388 - #eceff4
             216  388 - #263925
              84  392 - #4a89b0
             172  400 - #16301f
             460  400 - #16301f
             544  400 - #16301f
             632  440 - #14171c
             176  448 - #454b53
             192  448 - #575d67
             252  448 - #2f343b
             312  448 - #9aa3b2
              64  452 - #3b4048
              96  452 - #9aa3b2
             140  452 - #14171c
             168  452 - #9aa3b2
             236  452 - #99a2b1
             276  452 - #646a75
             292  452 - #3b4048
             312  452 - #9aa3b2
             320  452 - #97a0af
             344  452 - #9aa3b2
             352  452 - #2c3037
             372  452 - #14171c
             388  452 - #14171c
             484  480 - #0f1319
              84  484 - #2b3139
             132  484 - #575f6b
             188  484 - #525965
             204  484 - #444b56
             224  484 - #7c8696
             208  496 - #444a55
             100  500 - #6f7887
             116  500 - #444a55
             256  500 - #6f7887
             272  500 - #2b3139
             332  500 - #707988
             148  504 - #666f7d
             232  504 - #666f7d
             312  504 - #666f7d
              84  512 - #343a44
              92  516 - #7c8696
             176  532 - #d3d6db
             280  532 - #0f1319
             548  532 - #0f1319
              60  536 - #292135
              72  536 - #754b86
             100  536 - #4479a4
             176  536 - #d3d6db
             204  536 - #c678dd
             632  540 - #14171c
             132  548 - #61aeee
             148  548 - #1d2f3f
             108  552 - #3e6c94
             132  552 - #61aeee
             224  552 - #1f1e1f
             220  568 - #0f1319
             384  580 - #0f1319
             480  580 - #0f1319
               4  596 - #14171c
             576  628 - #14171c
              64  648 - #3c4149
              76  648 - #9aa3b2
              96  648 - #14171c
             120  648 - #14171c
             140  648 - #3b4048
             164  648 - #14171c
             184  648 - #555b64
             192  648 - #555b64
             248  648 - #373b43
             432  652 - #14171c
             188  664 - #4c88b9
              48  668 - #3a3f46
             176  672 - #386083
             220  672 - #97c279
             236  672 - #97c279
             304  672 - #59616e
             348  672 - #22272f
             512  676 - #0f1319
              44  692 - #8d95a3
             172  692 - #0f1319
             196  692 - #c678dd
             220  692 - #0f1319
             244  692 - #4b345a
             264  692 - #2f5d65
             316  692 - #bc8c5e
             328  692 - #6f563f
             336  692 - #4b345a
              76  712 - #191c22
             204  712 - #e6e9ee
              48  716 - #3b4048
              92  716 - #3b4048
             168  716 - #458e99
             184  716 - #4999a4
             308  716 - #7b8595
             632  716 - #14171c
              96  732 - #717884
              60  736 - #8d95a3
              80  736 - #9aa3b2
              88  736 - #8a92a0
             104  736 - #545a64
             180  736 - #a8acb1
             204  736 - #61656b
             220  736 - #babdc2
             244  736 - #585b61
             260  736 - #808389
             304  736 - #a5a8ae
             308  736 - #ebeef3
             340  736 - #3b3f45
             384  736 - #d6d9df
             416  736 - #0f1319
             460  736 - #808389
             548  752 - #14171c
            ";
