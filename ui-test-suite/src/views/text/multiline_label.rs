use anyhow::Result;
use hilen::{
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
             128   96 - #cccccc
             276  104 - #000000
             232  112 - #ffffff
              28  116 - #4f4f4f
              60  116 - #2d2d2d
             160  116 - #383838
             192  116 - #9e9e9e
              76  124 - #363636
             128  124 - #000000
             244  124 - #1f1f1f
             108  148 - #b4b4b4
             280  148 - #4c4c4c
              72  156 - #cbcbcb
             228  156 - #181818
              52  160 - #ffffff
             104  160 - #a6a6a6
             152  160 - #ffffff
              76  164 - #939393
             176  164 - #e1e1e1
             180  164 - #e1e1e1
             228  164 - #181818
              20  168 - #000000
              76  168 - #939393
             108  168 - #b4b4b4
             228  168 - #181818
             280  168 - #4c4c4c
             264  172 - #000000
             168  216 - #000000
             108  296 - #ffffff
             296  296 - #ffffff
             260  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        from_main(move || {
            view.label.set_multiline(true);
        });

        check_colors(
            r"
             592    4 - #597c95
             124   32 - #ffffff
              40   36 - #000000
             188   44 - #8b8b8b
             220   48 - #dedede
             192   52 - #d2d2d2
             244   56 - #848484
             196   84 - #707070
             124   88 - #ffffff
             196  100 - #707070
              40  120 - #3c3c3c
             240  124 - #4f4f4f
             260  132 - #e9e9e9
             264  132 - #e9e9e9
             104  140 - #d0d0d0
             132  140 - #515151
             108  148 - #929292
             152  148 - #333333
             240  148 - #4f4f4f
             176  176 - #000000
             232  180 - #949494
             120  184 - #4c4c4c
             228  188 - #cacaca
             232  192 - #949494
             200  220 - #6a6a6a
              76  224 - #010101
             148  224 - #363636
             240  232 - #000000
              20  260 - #ffffff
             160  280 - #555555
             296  296 - #ffffff
             592  592 - #597c95
            ",
        )?;

        Ok(())
    }
}
