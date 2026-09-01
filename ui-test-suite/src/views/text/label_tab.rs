use anyhow::Result;
use hilen::{
    AppRunner,
    dispatch::from_main,
    refs::{Weak, manage::DataManager},
    ui::{Font, Label, Screenshot, Setup, TextAlignment, U8Color, ViewFrame, ViewTest, view},
    ui_test::check_colors,
};

const TAB_FRAME: (u32, u32, u32, u32) = (20, 20, 560, 40);
const SPACES_FRAME: (u32, u32, u32, u32) = (20, 70, 560, 40);
const ONLY_TABS_FRAME: (u32, u32, u32, u32) = (20, 320, 560, 40);
const EMPTY_FRAME: (u32, u32, u32, u32) = (20, 370, 560, 40);

const RENDERED: &str = r"
         152   32 - #212e37
         128   36 - #202d36
         192   36 - #3e5667
         220   36 - #3e5667
         208   40 - #3d5667
         248   40 - #597c95
         116   48 - #000000
         156   84 - #1b252d
         228   84 - #1b252d
         188   88 - #000001
         256   88 - #000000
         212   92 - #0d1216
         128   96 - #202d36
         112  100 - #374c5c
          56  132 - #1c272f
         116  140 - #000000
          44  144 - #597c95
          56  188 - #1c272f
         116  188 - #000000
         592  208 - #597c95
          56  232 - #1c272f
         104  232 - #000000
          44  244 - #597c95
         192  244 - #000000
         112  280 - #000000
          92  292 - #3b5263
         140  292 - #597c95
         124  296 - #597c95
         160  296 - #283844
           4  552 - #597c95
         272  592 - #597c95
         592  592 - #597c95
";

/// A tab used to shape to the notdef box of the font. It draws as a
/// space stretched to the next 4 column tab stop.
#[view]
struct LabelTab {
    #[init]
    tab:       Label,
    spaces:    Label,
    two:       Label,
    three:     Label,
    four:      Label,
    only_tabs: Label,
    empty:     Label,
    roboto:    Label,
}

impl Setup for LabelTab {
    fn setup(self: Weak<Self>) {
        let mono = Font::get("DroidSansMono.ttf");

        for (label, text, y) in [
            (self.tab, "\tint fetch;", 20),
            (self.spaces, "    int fetch;", 70),
            (self.two, "ab\tX", 120),
            (self.three, "abc\tX", 170),
            (self.four, "abcd\tX", 220),
            (self.only_tabs, "\t\t", 320),
            (self.empty, "", 370),
        ] {
            label.set_frame((20, y, 560, 40));
            label
                .set_text(text)
                .set_text_size(30)
                .set_font(mono)
                .set_alignment(TextAlignment::Left);
        }

        self.roboto.set_frame((20, 270, 560, 40));
        self.roboto
            .set_text("\tint fetch;")
            .set_text_size(30)
            .set_alignment(TextAlignment::Left);
    }
}

impl ViewTest for LabelTab {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let shot = AppRunner::take_screenshot()?;

        assert!(
            region(&shot, TAB_FRAME) == region(&shot, SPACES_FRAME),
            "a leading tab renders unlike 4 spaces"
        );
        assert!(
            region(&shot, ONLY_TABS_FRAME) == region(&shot, EMPTY_FRAME),
            "a tab only label draws ink"
        );

        from_main(move || {
            let width = |label: Weak<Label>| label.content_size().width;
            let two = width(view.two);
            let three = width(view.three);
            let four = width(view.four);
            assert!(
                (two - three).abs() < 0.5,
                "tab after 2 and 3 chars reached different stops: {two} vs {three}"
            );
            assert!(
                four - three > 1.0,
                "tab after 4 chars did not reach the next stop: {four} vs {three}"
            );

            let spaces = from_measure(view.roboto, "    x");
            let tab = from_measure(view.roboto, "\tx");
            assert!(
                (tab - spaces).abs() < 0.5,
                "a tab in a proportional font is not 4 spaces: {tab} vs {spaces}"
            );
            view.roboto.set_text("\tint fetch;");
        });

        check_colors(RENDERED)?;

        Ok(())
    }
}

fn from_measure(label: Weak<Label>, text: &str) -> f32 {
    label.set_text(text);
    label.content_size().width
}

fn region(shot: &Screenshot, frame: (u32, u32, u32, u32)) -> Vec<U8Color> {
    let (x, y, w, h) = frame;
    let mut pixels = Vec::with_capacity((w * h) as usize);

    for row in y..y + h {
        for col in x..x + w {
            pixels.push(shot.get_pixel((col, row)));
        }
    }

    pixels
}
