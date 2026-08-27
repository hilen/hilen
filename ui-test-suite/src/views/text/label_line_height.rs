use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::check_colors,
};

#[view]
struct LabelLineHeight {
    #[init]
    label: Label,
}

impl Setup for LabelLineHeight {
    fn setup(self: Weak<Self>) {
        self.label.place().tl(20).size(280, 200);
        self.label.set_text_size(20).set_color(WHITE);
        self.label.set_multiline(true);
        self.label.set_text("First line\nSecond line\nThird line");
    }
}

impl ViewTest for LabelLineHeight {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let natural = from_main(move || view.label.size_for_width(280.0).height);

        let custom = from_main(move || {
            view.label.set_line_height(40);
            view.label.size_for_width(280.0).height
        });

        assert!(
            (custom - 120.0).abs() < 0.01,
            "3 lines in 40 point boxes must measure 120: {custom}"
        );
        assert!(
            custom > natural,
            "40 point boxes must be taller than the natural height: {custom} vs {natural}"
        );

        check_colors(
            r"
             296   20 - #ffffff
              20   24 - #ffffff
             128   76 - #ffffff
             168   76 - #272727
             128   80 - #727272
             152   80 - #ffffff
             192   80 - #969696
             136   84 - #8e8e8e
             168   84 - #272727
             184   84 - #2c2c2c
             116  116 - #ffffff
             192  120 - #000000
             136  124 - #ffffff
             160  124 - #b8b8b8
             172  124 - #000000
             188  124 - #aaaaaa
             132  156 - #4d4d4d
             172  156 - #070707
             132  160 - #4b4b4b
             148  160 - #2e2e2e
             180  160 - #bdbdbd
             196  160 - #969696
             124  164 - #0a0a0a
             132  164 - #4d4d4d
             140  164 - #646464
             172  164 - #070707
             180  164 - #bdbdbd
             188  164 - #000000
              20  216 - #ffffff
             592  228 - #597c95
             180  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        Ok(())
    }
}
