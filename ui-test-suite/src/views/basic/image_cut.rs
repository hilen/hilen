use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{CutAxis, ImageView, Label, Setup, TextAlignment, ViewFrame, ViewTest, view},
    ui_test::check_colors,
};

/// The fixture picture is a wooden frame around a board, 900 by 171 pixels.
/// The first and the last 50 pixels hold the mitred corners and always show
/// whole. The grain is level from end to end, so a cut through the middle
/// joins the last end.
const END: f32 = 50.0;

#[view]
struct ImageCut {
    #[init]
    whole_title:      Label,
    whole:            ImageView,
    short_title:      Label,
    short:            ImageView,
    shorter:          ImageView,
    small_title:      Label,
    small:            ImageView,
    smaller:          ImageView,
    big_title:        Label,
    big:              ImageView,
    vertical_title:   Label,
    vertical:         ImageView,
    fractional_title: Label,
    fractional:       ImageView,
    long_title:       Label,
    long:             ImageView,
}

impl Setup for ImageCut {
    fn setup(self: Weak<Self>) {
        for image in [
            self.whole,
            self.short,
            self.shorter,
            self.small,
            self.smaller,
            self.big,
            self.fractional,
            self.long,
        ] {
            image.set_image("cut.png").set_cut(CutAxis::Horizontal, END, END);
        }
        self.vertical.set_image("cut.png").set_cut(CutAxis::Vertical, END, END);

        self.whole_title.set_text("as long as the picture, all of it shows");
        self.whole_title.set_frame((20, 8, 560, 22));
        self.whole.set_frame((20, 32, 314, 60));

        self.short_title
            .set_text("shorter, same height: the same ends, less of the middle");
        self.short_title.set_frame((20, 104, 600, 22));
        self.short.set_frame((20, 128, 160, 60));
        self.shorter.set_frame((320, 128, 90, 60));

        self.small_title.set_text("half the height: the ends are half the size too");
        self.small_title.set_frame((20, 200, 600, 22));
        self.small.set_frame((20, 224, 157, 30));
        self.smaller.set_frame((190, 224, 60, 30));

        self.big_title.set_text("higher: bigger ends, nothing stretched");
        self.big_title.set_frame((20, 266, 600, 22));
        self.big.set_frame((20, 290, 565, 108));

        self.vertical_title
            .set_text("cut along the height: the top and the bottom stay");
        self.vertical_title.set_frame((20, 410, 600, 22));
        self.vertical.set_frame((20, 434, 200, 30));

        self.fractional_title.set_text("on a fractional position, no seams");
        self.fractional_title.set_frame((20, 476, 600, 22));
        self.fractional.set_frame((20.3, 500.7, 170.4, 60.0));

        self.long_title.set_text("longer than the picture: the middle has to stretch");
        self.long_title.set_frame((20, 572, 600, 22));
        self.long.set_frame((20, 596, 600, 60));

        for title in [
            self.whole_title,
            self.short_title,
            self.small_title,
            self.big_title,
            self.vertical_title,
            self.fractional_title,
            self.long_title,
        ] {
            title.set_text_size(14).set_alignment(TextAlignment::Left);
        }
    }
}

fn initial() -> Result<()> {
    check_colors(
        r"
             184   16 - #06080a
              60   20 - #597c95
             276   40 - #926b42
             140  116 - #212e37
             232  116 - #000000
             336  116 - #3a5161
              28  144 - #d7af83
             276  208 - #24323c
             168  212 - #000000
              24  240 - #c49d74
             424  292 - #f4ddbd
             108  304 - #553516
             320  304 - #4f3216
             576  380 - #e3b885
             212  384 - #f6ecda
             380  396 - #b48656
             476  396 - #ac7f4b
              48  420 - #476276
             296  420 - #314552
             124  424 - #597c95
             100  508 - #7a5229
             152  508 - #79532d
              20  512 - #847e73
             176  552 - #b2906e
              76  560 - #8d8a7f
             320  584 - #527289
              36  604 - #906a43
             136  652 - #dbac78
             452  652 - #ebc396
             604  652 - #e4bb8e
             208  992 - #597c95
             632  992 - #597c95
        ",
    )
}

fn grown(view: Weak<ImageCut>) -> Result<()> {
    from_main(move || {
        view.short_title
            .set_text("grown to 280 wide: more of the middle shows, the ends did not move");
        view.short.set_frame((20, 128, 280, 60));
    });

    check_colors(
        r"
              80   20 - #3e5768
             244   20 - #597c95
              72  116 - #2d3f4c
             388  116 - #000000
             212  136 - #8e6943
             120  212 - #597c95
             284  212 - #314451
              28  228 - #bc956b
             424  292 - #f4ddbd
             540  292 - #f5dcbb
             136  304 - #452b11
             348  304 - #543517
              20  324 - #9f7446
             272  328 - #dab082
             576  380 - #e3b885
             504  388 - #d1a16d
             368  396 - #b38755
              44  420 - #151e24
             172  420 - #000000
             280  420 - #010202
             236  488 - #3b5262
              20  504 - #847e73
             100  508 - #7a5229
             152  508 - #79532d
             176  552 - #b2906e
              28  560 - #8c887c
             320  584 - #527289
             108  652 - #dbb07d
             452  652 - #ebc396
             604  652 - #e4bb8e
             128  992 - #597c95
             632  992 - #597c95
        ",
    )
}

impl ViewTest for ImageCut {
    fn canvas() -> (u32, u32) {
        (640, 1000)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        initial()?;
        grown(view)?;

        Ok(())
    }
}
