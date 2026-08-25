use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, VerticalAlignment, ViewData, ViewTest, WHITE, view},
    ui_test::check_colors,
};

/// A label's lines sit in the middle of its frame unless it asks for
/// `VerticalAlignment::Top`, which is what a text area needs.
const CHECK_1: &str = r"
              24   20 - #ffffff
             296   20 - #ffffff
              72  100 - #000000
             180  108 - #909090
             128  112 - #ffffff
             212  116 - #000000
             100  120 - #ffffff
             244  124 - #000000
             160  152 - #8a8a8a
             124  156 - #4c4c4c
             112  160 - #2b2b2b
             124  160 - #4c4c4c
             112  164 - #2b2b2b
             124  164 - #4c4c4c
             184  164 - #cecece
             200  164 - #e1e1e1
             208  164 - #e1e1e1
             112  168 - #2b2b2b
             124  168 - #4c4c4c
             188  168 - #909090
             132  200 - #000000
              68  204 - #ffffff
             184  204 - #000000
             224  208 - #787878
             252  208 - #0c0c0c
             104  212 - #000000
             224  216 - #787878
             148  220 - #747474
             592  248 - #597c95
              20  296 - #ffffff
               4  592 - #597c95
             592  592 - #597c95
";

const CHECK_2: &str = r"
             592    4 - #597c95
              72   28 - #acacac
              88   28 - #acacac
             212   32 - #000000
             152   40 - #010101
             240   44 - #9c9c9c
             120   56 - #010101
             124   84 - #4c4c4c
             112   88 - #2b2b2b
             112   92 - #2b2b2b
             124   92 - #4c4c4c
             184   92 - #cecece
             188   92 - #909090
             112   96 - #2b2b2b
             188   96 - #909090
             112  100 - #2b2b2b
             124  100 - #4c4c4c
             184  100 - #cecece
             188  100 - #909090
             184  120 - #dcdcdc
              92  136 - #ffffff
             224  140 - #787878
              68  144 - #ffffff
             176  144 - #ffffff
             224  144 - #787878
             128  148 - #000000
             224  148 - #787878
             256  148 - #000000
             296  288 - #ffffff
             152  296 - #ffffff
               4  592 - #597c95
             592  592 - #597c95
";

const CHECK_3: &str = r"
              24   20 - #ffffff
             296   20 - #ffffff
              72  100 - #000000
             180  108 - #909090
             128  112 - #ffffff
             212  116 - #000000
             100  120 - #ffffff
             244  124 - #000000
             160  152 - #8a8a8a
             124  156 - #4c4c4c
             112  160 - #2b2b2b
             124  160 - #4c4c4c
             112  164 - #2b2b2b
             124  164 - #4c4c4c
             184  164 - #cecece
             200  164 - #e1e1e1
             208  164 - #e1e1e1
             112  168 - #2b2b2b
             124  168 - #4c4c4c
             188  168 - #909090
             132  200 - #000000
              68  204 - #ffffff
             184  204 - #000000
             224  208 - #787878
             252  208 - #0c0c0c
             104  212 - #000000
             224  216 - #787878
             148  220 - #747474
             592  248 - #597c95
              20  296 - #ffffff
               4  592 - #597c95
             592  592 - #597c95
";

#[view]
struct LabelVerticalAlignment {
    #[init]
    label: Label,
}

impl Setup for LabelVerticalAlignment {
    fn setup(self: Weak<Self>) {
        self.label.place().tl(20).size(280, 280);
        self.label.set_text_size(40).set_color(WHITE).set_multiline(true);
        self.label.set_text("Top of the frame\nsecond line");
    }
}

impl ViewTest for LabelVerticalAlignment {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(CHECK_1)?;

        from_main(move || {
            view.label.set_vertical_alignment(VerticalAlignment::Top);
        });

        check_colors(CHECK_2)?;

        from_main(move || {
            view.label.set_vertical_alignment(VerticalAlignment::Center);
        });

        check_colors(CHECK_3)?;

        Ok(())
    }
}
