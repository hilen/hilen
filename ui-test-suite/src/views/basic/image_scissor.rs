use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{ImageMode, ImageView, Setup, ViewFrame, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct ImageScissor {
    #[init]
    image: ImageView,
}

impl Setup for ImageScissor {
    fn setup(mut self: Weak<Self>) {
        self.image.set_image("cat.png").set_frame((20, 20, 100, 400));
        self.image.mode = ImageMode::AspectFill;
    }
}

impl ViewTest for ImageScissor {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        initial_frame()?;
        from_main(move || {
            view.image.set_size(200, 50);
        });

        resized()?;
        from_main(move || {
            view.image.set_position((-20, -40));
        });

        moved_off_screen()?;
        from_main(move || {
            view.image.set_size(200, 1000);
        });

        tall()?;
        from_main(move || {
            view.image.set_frame((20, 500, 200, 200));
        });

        reframed()?;

        Ok(())
    }
}

fn initial_frame() -> Result<()> {
    check_colors(
        r"
             592    4 - #597c95
             348   12 - #597c95
              24   24 - #eac3c4
              92   24 - #e0b8b9
              60   40 - #e0b8b9
             116   52 - #dcb2b3
              56   76 - #e3b9ba
             116   88 - #deb0b0
              84  100 - #deb0b0
              40  136 - #d2b6a1
             116  144 - #c0a68d
              60  188 - #d4b69c
             592  188 - #597c95
             292  196 - #597c95
             108  200 - #100605
             108  204 - #0e0402
              40  240 - #bf8f7b
             104  256 - #c3957e
              84  288 - #caa68e
             444  300 - #597c95
              48  304 - #cfaf9a
             116  312 - #c9a994
              80  328 - #cfaf98
              24  360 - #d6b19e
              92  364 - #cdaa96
              60  376 - #d1ab98
              84  400 - #d2af9b
             592  408 - #597c95
              52  416 - #d1a491
             116  416 - #af8c76
             320  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn resized() -> Result<()> {
    check_colors(
        r"
              24   24 - #e9bbbd
              40   24 - #e8bcbd
              64   24 - #f2d7cc
             116   24 - #caac92
             196   24 - #cb9998
             216   24 - #cd9699
             164   28 - #b6967d
              52   36 - #f5dad1
             192   40 - #ca9897
              32   44 - #e2b6b7
              72   44 - #e7c3b5
              52   48 - #f4d9d2
             216   52 - #c89697
              48   56 - #f3dad3
             164   56 - #bf9d84
              48   60 - #f2dfd5
              52   60 - #f2dcd1
             592   60 - #597c95
              24   64 - #e3b1b2
              44   64 - #f4ddd5
             140   64 - #be997f
              44   68 - #f6d9d3
              68   68 - #e5c7bd
             192   68 - #ac8777
             212   68 - #c89697
             404  128 - #597c95
             300  300 - #597c95
               4  328 - #597c95
             564  328 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn moved_off_screen() -> Result<()> {
    check_colors(
        r"
               4    4 - #f4ddd5
              16    4 - #f2d7ce
              28    4 - #e2c4b9
              32    4 - #e0bfb0
             100    4 - #be997f
             124    4 - #c4a289
             156    4 - #c59394
             172    4 - #c99597
             592    4 - #597c95
               4    8 - #f6d9d3
               8    8 - #f2dcce
              12    8 - #f2dccf
              20    8 - #eed0c6
              28    8 - #e5c7bd
             120    8 - #c19d85
             132    8 - #c3a08a
             152    8 - #ac8777
             164    8 - #c89496
             168    8 - #c99597
             172    8 - #c89697
             176    8 - #c89697
             384   92 - #597c95
             532  152 - #597c95
             156  188 - #597c95
               8  300 - #597c95
             300  300 - #597c95
             592  300 - #597c95
             444  444 - #597c95
             152  448 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn tall() -> Result<()> {
    check_colors(
        r"
               4    4 - #e8c0c1
             144    4 - #dfb5b6
             592    4 - #597c95
             176  140 - #deb0b0
               4  188 - #cea998
             452  252 - #597c95
              72  256 - #ccb49c
             168  272 - #a88e75
              16  344 - #332a13
             104  348 - #937458
              28  360 - #64553b
             152  364 - #94765c
               4  368 - #0e0500
             164  396 - #3b2910
             152  408 - #605537
             176  412 - #100404
             132  420 - #0f0000
             164  428 - #0e0402
             152  440 - #4f3e22
             176  440 - #403222
             164  444 - #503c21
              36  464 - #643b2b
              92  468 - #755a44
              52  472 - #904d30
              88  472 - #6d503e
              60  476 - #8f4a31
             160  492 - #9c7a5f
             592  504 - #597c95
              36  512 - #92644d
             112  540 - #bb8d71
             176  572 - #ba8e75
               4  592 - #cda691
        ",
    )?;

    Ok(())
}

fn reframed() -> Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             296    8 - #597c95
              52  252 - #597c95
             572  276 - #597c95
             300  300 - #597c95
             412  464 - #597c95
              24  504 - #edc6cb
              60  504 - #e6bebf
             152  504 - #dfb1b1
             196  504 - #d09e9d
             216  504 - #d4a2a3
             124  512 - #dfb1b1
             184  520 - #d3a09f
             160  528 - #d6a3a2
              52  532 - #e6bebf
             104  532 - #d6baac
             216  536 - #cc9a99
              24  540 - #e9c0c4
             208  548 - #cb9998
             592  548 - #597c95
              72  560 - #e5cabe
             192  560 - #965b49
              40  564 - #e7bdbe
             136  564 - #c1a58d
             216  564 - #cb9998
             196  584 - #ca9897
              24  592 - #e3bbbc
              60  592 - #efd3c7
              96  592 - #ceae95
             172  592 - #bc9c83
             216  592 - #c99796
        ",
    )?;

    Ok(())
}
