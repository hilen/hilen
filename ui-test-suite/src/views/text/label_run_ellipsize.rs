use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLACK, Font, Label, RunStyle, Setup, TextAlignment, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, checkpoint},
};

const TITLE: &str = "Wednesday, November 4, 2026";
/// The bold part of the title, every word but the year.
const BOLD: std::ops::Range<usize> = 0..22;
const SIZE: f32 = 20.0;
/// Narrower than the title, so it must end in an ellipsis.
const CUT_W: f32 = 233.0;

const TITLES: &str = r"
    92   28 - #c2cfd8
    132   28 - #809bae
    24   52 - #ffffff
    196   56 - #646464
    212   60 - #838383
    60   64 - #555555
    100   64 - #000000
    144   64 - #434343
    32  108 - #597c95
    88  108 - #597c95
    152  108 - #7a96aa
    164  108 - #a5b8c5
    292  124 - #ffffff
    244  132 - #ffffff
    196  136 - #646464
    100  140 - #000000
    136  140 - #e7e7e7
    20  144 - #ffffff
    56  144 - #000000
    196  144 - #646464
    52  188 - #e0e6eb
    104  188 - #597c95
    140  188 - #dbe2e8
    200  188 - #a4b7c5
    244  204 - #ffffff
    76  220 - #838383
    288  220 - #000000
    24  224 - #000000
    144  224 - #434343
    196  224 - #646464
    88  592 - #597c95
    592  592 - #597c95
";

/// Bold by a font run on the regular font, or bold as the whole font.
fn style(label: Weak<Label>, as_run: bool) {
    label.set_color(WHITE).set_text_color(BLACK).set_text_size(SIZE);
    label.set_alignment(TextAlignment::Left).set_text_inset(0.0);
    label.set_text(TITLE);
    if as_run {
        label.set_font(Font::roboto());
        label.set_font_runs([(BOLD, RunStyle::font(Font::bold()))]);
    } else {
        label.set_font(Font::bold());
    }
}

#[view]
struct LabelRunEllipsize {
    #[init]
    caption_cut:  Label,
    cut:          Label,
    caption_runs: Label,
    runs:         Label,
    caption_bold: Label,
    bold:         Label,
}

impl Setup for LabelRunEllipsize {
    fn setup(self: Weak<Self>) {
        for (caption, text, y) in [
            (self.caption_cut, "bold run, ellipsis on, 233 wide", 16),
            (self.caption_runs, "bold run, fitted to its measure", 96),
            (self.caption_bold, "bold font for the whole text, fitted", 176),
        ] {
            caption.set_text(text).set_text_size(14).set_text_color(WHITE);
            caption.set_alignment(TextAlignment::Left).set_text_inset(0.0);
            caption.set_frame((20, y, 400, 22));
        }

        style(self.cut, true);
        self.cut.set_ellipsize(true);
        self.cut.set_frame((20.0, 44.0, CUT_W, 32.0));

        style(self.runs, true);
        style(self.bold, false);
        for (label, y) in [(self.runs, 124.0), (self.bold, 204.0)] {
            // A whole point keeps the right edge off a fractional column,
            // where 1 sample and 4 samples draw it differently.
            let width = label.size_for_width(1000.0).width.ceil();
            label.set_frame((20.0, y, width, 32.0));
        }
    }
}

impl ViewTest for LabelRunEllipsize {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The bold glyphs of a run count in the measure, the way the
        // glyphs of a bold font do.
        let (runs_w, bold_w) = from_main(move || {
            (
                view.runs.size_for_width(1000.0).width,
                view.bold.size_for_width(1000.0).width,
            )
        });
        if (runs_w - bold_w).abs() > 1.0 {
            bail!("a bold run measures {runs_w} points, the same text in the bold font {bold_w}");
        }

        // So the cut title ends in an ellipsis that fits in its frame.
        let (drawn, drawn_w) = from_main(move || {
            let drawn = view.cut.display_text(CUT_W).to_string();
            let probe = view.bold;
            probe.set_text(&drawn);
            let width = probe.size_for_width(1000.0).width;
            probe.set_text(TITLE);
            (drawn, width)
        });
        ensure!(
            drawn.ends_with('…'),
            "the cut title drew {drawn:?}, with no ellipsis"
        );
        ensure!(
            drawn_w <= CUT_W + 1.0,
            "the cut title {drawn:?} is {drawn_w} points wide in a frame of {CUT_W}"
        );

        check_colors(TITLES)?;
        checkpoint("a cut title ending in an ellipsis, a fitted run and its bold twin of the same width")?;

        Ok(())
    }
}
