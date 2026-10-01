use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLACK, Label, Setup, TextAlignment, VerticalAlignment, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::check_colors,
};

const CHECK_1: &str = r"
    44   24 - #ffffff
    248   32 - #b2b2b2
    348   32 - #5a5a5a
    296   48 - #141414
    592   64 - #597c95
    76   72 - #5c5c5c
    336   72 - #7a7a7a
    200   80 - #969696
    80   96 - #474747
    140   96 - #6c6c6c
    252  104 - #464646
    48  120 - #202020
    48  128 - #202020
    148  192 - #868686
    380  192 - #c7c7c7
    56  196 - #000000
    264  196 - #dcdcdc
    264  244 - #dcdcdc
    48  264 - #474747
    336  264 - #9c9c9c
    168  268 - #050505
    288  316 - #000000
    336  344 - #9c9c9c
    180  348 - #545454
    48  372 - #ffffff
    288  372 - #c7c7c7
    400  372 - #d2d2d2
    164  432 - #c5c5c5
    72  436 - #000000
    20  516 - #ffffff
    384  516 - #ffffff
    592  592 - #597c95
";

const TEXT: &str = "A long description of a film that goes on and on, with far more words than fit in the \
                    few lines a card has room for, so the last line that still shows has to end in an ellipsis.";
const SHORT: &str = "Short enough to fit.";

/// The same long text in 4 multiline labels of one width: no limit, a limit
/// of 1, of 2 and of 3 lines, and a short text under a limit of 3. Proves
/// `set_max_lines` cuts the text to that many lines with an ellipsis at the
/// end of the last one, leaves a text that fits alone, and makes the label
/// measure as tall as what it draws.
#[view]
struct LabelMaxLines {
    #[init]
    free:  Label,
    one:   Label,
    two:   Label,
    three: Label,
    short: Label,
}

impl Setup for LabelMaxLines {
    fn setup(self: Weak<Self>) {
        let rows = [
            (self.free, 0, TEXT, 20, 150),
            (self.one, 1, TEXT, 180, 40),
            (self.two, 2, TEXT, 230, 70),
            (self.three, 3, TEXT, 310, 100),
            (self.short, 3, SHORT, 420, 100),
        ];
        for (label, lines, text, top, height) in rows {
            label.set_frame((20, top, 400, height));
            label
                .set_text(text)
                .set_text_size(20)
                .set_multiline(true)
                .set_max_lines(lines)
                .set_alignment(TextAlignment::Left)
                .set_vertical_alignment(VerticalAlignment::Top)
                .set_text_color(BLACK)
                .set_color(WHITE);
        }
    }
}

impl ViewTest for LabelMaxLines {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // What a label draws, how many lines that is, and the lines its
        // measured height holds.
        let seen = move |label: Weak<Label>| {
            from_main(move || {
                let text = label.display_text(label.width()).to_string();
                let layout = label.text_layout_for(&text);
                let rows = (label.content_size().height / layout.line_height).round();
                (text, layout.line_count(), rows)
            })
        };

        let (text, lines, _) = seen(view.free);
        ensure!(text == TEXT, "no limit draws the whole text");
        ensure!(lines > 3, "the text needs more than 3 lines here, it has {lines}");

        for (label, limit) in [(view.one, 1), (view.two, 2), (view.three, 3)] {
            let (text, lines, rows) = seen(label);
            ensure!(lines == limit, "a limit of {limit} draws {lines} lines: {text}");
            ensure!(text.ends_with('…'), "the cut text ends in an ellipsis: {text}");
            ensure!(!text.ends_with(" …"), "no space before the ellipsis: {text}");
            let kept = text.trim_end_matches('…');
            ensure!(
                TEXT.starts_with(kept),
                "the cut text is a start of the text: {text}"
            );
            ensure!(
                (rows - f32::from(u8::try_from(limit)?)).abs() < 0.01,
                "a limit of {limit} measures {rows} lines tall"
            );
        }

        let (text, lines, _) = seen(view.short);
        ensure!(text == SHORT && lines == 1, "a text that fits stays as it is");

        check_colors(CHECK_1)?;
        Ok(())
    }
}
