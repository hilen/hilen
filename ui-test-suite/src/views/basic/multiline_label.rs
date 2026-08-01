use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::check_colors,
};

#[view]
struct MultilineLabel {
    #[init]
    label: Label,
}

impl Setup for MultilineLabel {
    fn setup(self: Weak<Self>) {
        self.label.place().tl(20).size(280, 280);
        self.label.set_text_size(40).set_color(WHITE);
        self.label
            .set_text("|       Plati mne dengi bistrenko pliz.\nJa kuplu dengushki.\nA");
    }
}

impl ViewTest for MultilineLabel {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             592    4 - #597c95
              24   24 - #ffffff
             188   24 - #ffffff
             296   24 - #ffffff
             128  116 - #ffffff
             192  116 - #ffffff
             268  116 - #ffffff
              56  120 - #000000
              92  124 - #ffffff
              56  128 - #000000
             476  148 - #597c95
             236  152 - #000000
             192  156 - #000000
              52  160 - #ffffff
             236  160 - #000000
             292  160 - #010101
              92  164 - #ffffff
             236  164 - #000000
             264  164 - #ffffff
             292  164 - #010101
             292  168 - #010101
             160  184 - #000000
             168  204 - #010101
              24  256 - #ffffff
             212  280 - #ffffff
             128  296 - #ffffff
             296  296 - #ffffff
             588  300 - #597c95
             428  460 - #597c95
               4  532 - #597c95
             264  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        from_main(move || {
            view.label.set_multiline(true);
        });

        check_colors(
            r"
             532    4 - #597c95
              24   24 - #ffffff
             108   24 - #ffffff
             192   24 - #ffffff
             168   64 - #ffffff
             276   64 - #ffffff
             136  100 - #010101
             188  100 - #ffffff
              56  140 - #010101
             116  140 - #010101
             192  140 - #ffffff
             276  148 - #000000
             448  164 - #597c95
              72  180 - #010101
              72  184 - #010101
             144  184 - #ffffff
             208  184 - #ffffff
             272  184 - #ffffff
              24  228 - #ffffff
             160  244 - #000000
             240  248 - #ffffff
             160  256 - #ffffff
             592  260 - #597c95
             168  264 - #010101
              24  296 - #ffffff
             100  296 - #ffffff
             296  296 - #ffffff
             476  424 - #597c95
             132  472 - #597c95
             300  524 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
