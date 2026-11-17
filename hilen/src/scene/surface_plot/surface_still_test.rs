use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    scene::SurfacePlotView,
    ui::{ImageView, Label, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, set_record_probe_count},
};

const SIZE: (u32, u32) = (280, 220);

/// 2 still pictures of a surface plot, drawn with no scene running. Left
/// the saddle `z = (x² - y²) / 10`, colored by height, over the floor grid
/// with the 3 axes. Right a function with no finite value, only the floor.
#[view]
struct SurfaceStill {
    #[init]
    status:      Label,
    saddle:      ImageView,
    empty:       ImageView,
    saddle_name: Label,
    empty_name:  Label,
}

impl Setup for SurfaceStill {
    fn setup(self: Weak<Self>) {
        self.set_color("#FFFFFF");

        self.status
            .set_text("still pictures of a surface plot, no scene runs")
            .set_text_size(18)
            .set_text_color("#1B1F24");
        self.status.place().lrt(0).h(44);

        self.saddle.place().l(12).t(80).size(SIZE.0, SIZE.1);
        self.saddle.set_image(SurfacePlotView::picture(
            |x, y| ((x * x - y * y) / 10.0, None),
            10.0,
            "surface-still-saddle",
            SIZE,
        ));
        self.saddle_name.set_text("saddle").set_text_size(14).set_text_color("#1B1F24");
        self.saddle_name.place().l(12).t(52).size(SIZE.0, 24);

        self.empty.place().l(24 + SIZE.0).t(80).size(SIZE.0, SIZE.1);
        self.empty.set_image(SurfacePlotView::picture(
            |_, _| (f64::NAN, None),
            10.0,
            "surface-still-empty",
            SIZE,
        ));
        self.empty_name
            .set_text("no value, only the floor")
            .set_text_size(14)
            .set_text_color("#1B1F24");
        self.empty_name.place().l(24 + SIZE.0).t(52).size(SIZE.0, 24);
    }
}

impl ViewTest for SurfaceStill {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);
        let sizes = from_main(move || (view.saddle.image().size, view.empty.image().size));
        assert_eq!(sizes.0, SIZE.into());
        assert_eq!(sizes.1, SIZE.into());
        check_colors(CHECK_1)?;
        Ok(())
    }
}

const CHECK_1: &str = r"
     144   16 - #9e9fa1
     188   20 - #1b1f24
     272   20 - #595c5f
     448   20 - #787a7d
     136   24 - #9a9c9e
     168   24 - #1c2025
     208   24 - #ffffff
     300   24 - #4f5256
     352   24 - #373a3f
     372   24 - #2e3136
     424   24 - #45484c
     464   24 - #45484c
     164   64 - #c9cacb
     380   64 - #1c2025
     408   64 - #e1e1e2
     464   64 - #929496
     504   64 - #ffffff
     152  112 - #46a758
     444  124 - #46a758
     112  128 - #be3030
     100  132 - #c83837
     124  136 - #b64741
     116  140 - #c74f48
      88  144 - #d2534d
     108  144 - #d05950
     220  144 - #7c2423
     212  148 - #772423
     116  152 - #d47364
     128  152 - #c77766
     220  152 - #883531
     224  156 - #92433c
      92  160 - #dc8371
     108  160 - #de8a76
     136  160 - #c2917a
     208  160 - #7c3a34
     216  160 - #8c463e
     444  160 - #46a758
     124  164 - #de9d85
     144  168 - #b6aa8d
     204  168 - #814e43
     216  168 - #966153
      76  172 - #e0a188
     192  172 - #a14b43
     224  172 - #a37966
      92  176 - #e5b497
     132  176 - #e8c8a6
     208  176 - #967663
     116  180 - #ebcbaa
     188  180 - #a6564c
     196  180 - #86705d
     200  180 - #8f7965
     212  184 - #a59e82
      72  188 - #e6d1ae
     404  188 - #c7ccd4
     136  192 - #eeedc5
     180  192 - #b07362
     440  192 - #3e8ed0
     100  196 - #e8e8c3
     196  196 - #76847c
     368  196 - #f4f5f7
     460  196 - #e5484d
     112  200 - #e0e5c6
     168  200 - #c3a489
     188  200 - #5f7376
     208  200 - #708685
     472  200 - #e5484d
      76  204 - #ced8bd
     184  204 - #4a6673
      88  208 - #c4d2be
     196  208 - #476a7e
     128  212 - #c5d6c4
     216  212 - #e5484d
     408  212 - #3e8ed0
     508  212 - #e5484d
     140  216 - #c0cfbc
     168  216 - #bfa88c
     152  220 - #c0c7ad
      88  224 - #93b5b9
     104  224 - #99b9bd
     388  224 - #3e8ed0
     124  232 - #8fb3ba
     172  232 - #baa488
     140  236 - #97b1af
     152  244 - #9daea3
     460  244 - #f4f5f7
     116  248 - #6094b4
     104  252 - #4a87b2
     128  252 - #6695af
     156  256 - #97a79d
     116  264 - #3b7aad
     148  264 - #79989f
     124  268 - #3f7ba9
     300  524 - #ffffff
       4  592 - #ffffff
     592  592 - #ffffff
";
