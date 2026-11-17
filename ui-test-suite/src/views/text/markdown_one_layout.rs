use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, Color, Container, Label, MarkdownView, Setup, TextAlignment, VerticalAlignment, ViewData,
        ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

const FIRST: &str = r"The **first** text of the row.

- an item
- a second item
";

const SECOND: &str = r"The **second** text is longer, so the row under it has to grow by some lines.

1. step one
2. step two
3. step three

A last paragraph after the list.
";

const WIDTH: f32 = 360.0;
const LEFT: f32 = 20.0;
const TOP: f32 = 64.0;

/// Recorded with `--record-colors`.
const FIRST_SHOWN: &str = r"
             392    4 - #ffffff
             128   16 - #b1b1b1
              48   20 - #363636
              88   20 - #ffffff
             100   20 - #9d9d9d
             116   20 - #ffffff
             128   20 - #b1b1b1
             168   20 - #ffffff
             180   20 - #ffffff
             200   20 - #050505
             212   20 - #0b0b0b
              60   24 - #ffffff
             160   24 - #ffffff
             188   24 - #ffffff
              60   36 - #ffffff
              80   36 - #cecece
             104   36 - #ffffff
             120   36 - #bababa
             156   36 - #4b4b4b
             164   36 - #000000
             172   36 - #e7e7e7
             196   36 - #ffffff
             208   36 - #cecece
             220   36 - #010101
             228   36 - #ffffff
             236   36 - #ffffff
             244   36 - #cecece
             252   36 - #1b1b1b
             264   36 - #4b4b4b
             296   36 - #151515
             312   36 - #ffffff
             340   36 - #ffffff
             140   68 - #c9d0dc
             392   68 - #ffffff
              40   72 - #27282b
              64   72 - #3a3c40
              68   72 - #a0a6af
              72   72 - #3e4044
              84   72 - #dde5f2
             100   72 - #92979f
             112   72 - #60646a
             128   72 - #888d95
             136   72 - #bdc4cf
             140   72 - #c9d0dc
              64  100 - #83888f
              76  100 - #000000
              96  100 - #080809
             100  100 - #3c3e42
             132  120 - #80858c
             216  120 - #e8f0fe
             132  124 - #80858c
             304  124 - #e8f0fe
               4  132 - #ffffff
             392  132 - #ffffff
              60  164 - #ffffff
             176  168 - #ffffff
             384  184 - #ffffff
             256  196 - #ffffff
             332  196 - #ffffff
               4  200 - #ffffff
             100  212 - #ffffff
             180  232 - #ffffff
             296  240 - #ffffff
             392  240 - #ffffff
              60  252 - #ffffff
               4  264 - #ffffff
             212  276 - #ffffff
             336  280 - #ffffff
             264  284 - #ffffff
             112  304 - #000000
             120  304 - #878787
             136  304 - #323232
              60  308 - #ffffff
              68  308 - #ffffff
              84  308 - #9d9d9d
             100  308 - #ffffff
             112  308 - #000000
             120  308 - #878787
              44  312 - #ffffff
              76  312 - #ffffff
              48  320 - #ffffff
              84  320 - #ffffff
             112  320 - #ffffff
             184  320 - #ffffff
              56  324 - #ffffff
              96  324 - #000000
             120  324 - #515151
             144  324 - #010101
             152  324 - #ffffff
             160  324 - #ffffff
             168  324 - #cecece
             176  324 - #343434
             392  324 - #ffffff
               4  352 - #ffffff
             244  352 - #ffffff
             316  352 - #ffffff
";

/// Recorded with `--record-colors`.
const SECOND_SHOWN: &str = r"
             392    4 - #ffffff
             128   16 - #b1b1b1
              48   20 - #363636
             100   20 - #9d9d9d
             128   20 - #b1b1b1
             168   20 - #ffffff
             200   20 - #050505
             212   20 - #0b0b0b
             188   24 - #ffffff
              60   36 - #ffffff
              80   36 - #cecece
             104   36 - #ffffff
             120   36 - #bababa
             156   36 - #4b4b4b
             164   36 - #000000
             172   36 - #e7e7e7
             208   36 - #cecece
             220   36 - #010101
             244   36 - #cecece
             252   36 - #1b1b1b
             264   36 - #4b4b4b
             296   36 - #151515
             340   36 - #ffffff
             288   68 - #6d94db
              40   72 - #3d70ce
              72   72 - #1a56c4
             120   72 - #9bb7e8
             132   72 - #85a6e2
             140   72 - #3d70ce
             160   72 - #e8f0fe
             176   72 - #e8f0fe
             188   72 - #8eade5
             208   72 - #e8f0fe
             232   72 - #9bb7e8
             240   72 - #8eade5
             268   72 - #96b2e7
             344   72 - #3268cb
              92   76 - #e8f0fe
              72   84 - #678fda
             128   84 - #1a56c4
              44   88 - #cddbf6
              72   88 - #1a56c4
             104   88 - #7499dd
             128   88 - #1a56c4
             132   88 - #8eade5
             392  112 - #ffffff
              64  116 - #99b5e8
              72  116 - #8aaae4
              76  116 - #1a56c4
             100  116 - #1c58c5
              76  120 - #3b6fcd
              88  136 - #1a56c4
             336  136 - #e8f0fe
               4  140 - #ffffff
              72  140 - #e8f0fe
             228  140 - #e8f0fe
              88  160 - #1d58c5
             100  160 - #1a56c4
             392  172 - #ffffff
             288  180 - #e8f0fe
              40  192 - #c0d2f3
              76  192 - #1a56c4
              92  192 - #678fda
             108  192 - #1a56c4
             112  192 - #7c9fe0
             132  192 - #7c9fe0
             136  192 - #1b57c4
             168  192 - #ceddf7
             180  192 - #2961c8
             184  192 - #4677d0
             240  236 - #ffffff
             324  240 - #ffffff
               4  252 - #ffffff
             392  256 - #ffffff
             140  304 - #d4d4d4
             208  304 - #222222
              44  308 - #ffffff
              76  308 - #ffffff
             104  308 - #6a6a6a
             132  308 - #000000
             140  308 - #d4d4d4
             176  308 - #0b0b0b
             184  308 - #000000
             212  308 - #000000
             232  308 - #515151
             292  308 - #989898
             308  308 - #000000
             328  308 - #7e7e7e
             336  308 - #000000
              56  324 - #ffffff
              96  324 - #000000
             120  324 - #515151
             144  324 - #010101
             168  324 - #cecece
             176  324 - #343434
             392  352 - #ffffff
";

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

/// The 2 frames after which a change made before them is laid out: the
/// first runs what waits for the main thread, its update pass lays out,
/// the second proves that pass is over.
fn frames() {
    wait_for_next_frame();
    wait_for_next_frame();
}

/// A text and its color lay out once, not once per call. The hidden view
/// measures a text the way a table asks for the height of a row: a color,
/// a text and a measure in a row. The shown view gets a color and a text
/// with no measure and lays out once in the next frame. The tinted box is
/// as high as the hidden view measured, the shown text has to end with it.
#[view]
struct MarkdownOneLayout {
    #[init]
    caption:    Label,
    background: Container,
    shown:      MarkdownView,
    measure:    MarkdownView,
    step:       Label,
}

impl MarkdownOneLayout {
    fn small(label: Weak<Label>, text: &str, y: f32) {
        label
            .set_multiline(true)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_text(text);
        label.set_text_color(BLACK);
        label.set_frame((LEFT, y, WIDTH, 40.0));
    }

    /// The row gets the height the hidden view measured.
    fn give_room(self: Weak<Self>, height: f32) {
        self.shown.set_frame((LEFT, TOP, WIDTH, height));
        self.background.set_frame((LEFT, TOP, WIDTH, height));
    }

    fn set_step(self: Weak<Self>, step: &'static str) {
        from_main(move || {
            let text = format!(
                "{step}\nlayouts: shown {}, hidden {}",
                self.shown.layout_count(),
                self.measure.layout_count()
            );
            self.step.set_text(text);
        });
    }

    /// Where the lowest view of the shown text ends.
    fn shown_bottom(self: Weak<Self>) -> f32 {
        from_main(move || self.shown.subviews().iter().map(|view| view.max_y()).fold(0.0, f32::max))
    }
}

impl Setup for MarkdownOneLayout {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        Self::small(
            self.caption,
            "a text and its color lay out once\nthe tinted box is as high as the hidden view measured",
            12.0,
        );

        // Small and hidden, like the view a chat measures its rows with.
        self.measure.set_frame((0.0, 0.0, 10.0, 10.0));
        self.measure.set_hidden(true);

        self.background.set_color(Color::hex("#e8f0fe"));
        self.shown.set_text_color(BLACK);
        self.shown.set_text(FIRST);
        let height = self.shown.height_for_width(WIDTH);
        self.give_room(height);

        Self::small(self.step, "the first text\nlayouts: none counted yet", 300.0);
    }
}

impl ViewTest for MarkdownOneLayout {
    fn canvas() -> (u32, u32) {
        (400, 360)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // 3 times the default, the picture is mostly small text.
        set_record_probe_count(96);

        frames();
        let start = from_main(move || view.shown.layout_count());
        ensure!(start == 1, "the first text was laid out {start} times, not once");
        view.set_step("the first text, black");
        check_colors(FIRST_SHOWN)?;

        // A row is measured: a color, a text and a measure in a row.
        let (before, after, first_height) = from_main(move || {
            let before = view.measure.layout_count();
            view.measure.set_text_color(BLACK);
            view.measure.set_text(FIRST);
            let height = view.measure.height_for_width(WIDTH);
            (before, view.measure.layout_count(), height)
        });
        ensure!(
            after == before + 1,
            "a color, a text and a measure made {} layouts, not 1",
            after - before
        );

        // The next row, the color is the one the view has.
        let (before, after, second_height) = from_main(move || {
            let before = view.measure.layout_count();
            view.measure.set_text_color(BLACK);
            view.measure.set_text(SECOND);
            let height = view.measure.height_for_width(WIDTH);
            (before, view.measure.layout_count(), height)
        });
        ensure!(
            after == before + 1,
            "the second measure made {} layouts, not 1",
            after - before
        );
        ensure!(
            second_height > first_height + 20.0,
            "the longer text measured {second_height}, the short one {first_height}"
        );

        // The measured view is 10 wide and hidden, no frame lays it out
        // at that width.
        frames();
        let later = from_main(move || view.measure.layout_count());
        ensure!(
            later == after,
            "the hidden view was laid out {} more times with no change",
            later - after
        );

        let bottom = view.shown_bottom();
        ensure!(
            near(bottom, first_height),
            "the shown first text ends at {bottom}, the hidden view measured {first_height}"
        );

        // The color the view already has is no change.
        from_main(move || {
            view.shown.set_text_color(BLACK);
        });
        frames();
        let same = from_main(move || view.shown.layout_count());
        ensure!(same == start, "the same color made {} layouts", same - start);

        // A new color and a new text with no measure: nothing is laid out
        // at the call, 1 layout in the next frame.
        let at_call = from_main(move || {
            view.shown.set_text_color(Color::hex("#1a56c4"));
            view.shown.set_text(SECOND);
            view.give_room(second_height);
            view.shown.layout_count()
        });
        ensure!(
            at_call == start,
            "the layout did not wait, {} ran inside the calls",
            at_call - start
        );
        frames();
        let done = from_main(move || view.shown.layout_count());
        ensure!(
            done == start + 1,
            "a color and a text made {} layouts, not 1",
            done - start
        );

        view.set_step("the second text, blue, the box has the measured height");
        check_colors(SECOND_SHOWN)?;

        let bottom = view.shown_bottom();
        ensure!(
            near(bottom, second_height),
            "the shown second text ends at {bottom}, the hidden view measured {second_height}"
        );
        // The measure of a text that is laid out at that width is free.
        let (height, count) =
            from_main(move || (view.shown.height_for_width(WIDTH), view.shown.layout_count()));
        ensure!(
            near(height, second_height) && count == done,
            "the shown view says {height} after {count} layouts, the hidden one measured {second_height}"
        );

        Ok(())
    }
}
