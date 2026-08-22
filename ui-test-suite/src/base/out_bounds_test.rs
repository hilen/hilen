use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{
        Anchor::{self, Height, Width, Y},
        ImageView, Label, NumberView, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE,
        ui_test::{helpers::check_colors, inject_touches},
        view,
    },
};

#[view]
struct OutBounds {
    #[init]
    test: Label,
    x:    NumberView,
    y:    NumberView,
}

impl Setup for OutBounds {
    fn setup(mut self: Weak<Self>) {
        self.test
            .set_color(WHITE)
            .set_text("AA")
            .set_text_size(100)
            .set_frame((200, 200, 200, 200));

        let image = self.test.add_view::<ImageView>();
        image.set_image("cat.png");
        image.place().left_half();

        self.x.set_step(50.0);
        self.x
            .on_change(move |val| {
                self.test.set_x(200.0 + val);
            })
            .place()
            .size(60, 120)
            .t(260)
            .l(260);

        self.y.set_step(50.0);
        self.y
            .on_change(move |val| {
                self.test.set_y(200.0 + val);
            })
            .place()
            .same([Y, Width, Height], self.x)
            .anchor(Anchor::Left, self.x, 10);
    }
}

impl ViewTest for OutBounds {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_out_bottom()?;
        check_out_bottom_right()?;
        check_out_top_right()?;
        check_out_top_left()?;
        check_out_left()?;
        check_out_bottom_left()?;

        Ok(())
    }
}

fn check_out_bottom() -> Result<()> {
    inject_touches(
        r"
            372  307  b
            371  307  e
            372  304  b
            372  304  e
            373  303  b
            373  303  e
            373  303  b
            373  303  e
            373  303  b
            373  303  e
            373  304  b
            373  304  e
        ",
    );

    check_colors(
        r"
               4    4 - #597c95
             324    4 - #597c95
             592    4 - #597c95
             268  260 - #0096e6
             384  260 - #0096e6
               4  292 - #597c95
             284  356 - #ffffff
             388  376 - #0096e6
             200  504 - #ebc8ce
             296  504 - #d6a8aa
             396  504 - #ffffff
             236  528 - #c7978d
             272  528 - #ddafaf
             296  532 - #d3a1a2
             200  536 - #ecc5ca
             244  544 - #c6a48d
             228  548 - #bf7461
             292  556 - #aa806a
             288  564 - #a36858
             332  564 - #000000
             396  564 - #ffffff
             248  568 - #c1a58d
             332  568 - #000000
             264  572 - #b79d86
             328  572 - #000000
             336  576 - #000000
             236  580 - #584d35
             324  580 - #000000
             340  584 - #000000
             200  592 - #e7b9bc
             288  592 - #ca9897
             320  592 - #000000
        ",
    )?;

    Ok(())
}

fn check_out_bottom_right() -> Result<()> {
    inject_touches(
        r"
            308  305  b
            308  305  e
            308  304  b
            308  304  e
            307  304  b
            307  304  e
            307  304  b
            307  304  e
            307  304  b
            307  304  e
            307  304  b
            307  304  e
            307  304  b
            307  304  e
        ",
    );

    check_colors(
        r"
               4    4 - #597c95
             268    4 - #597c95
             592    4 - #597c95
             268  260 - #0096e6
             336  260 - #0096e6
               4  264 - #597c95
             360  276 - #ffffff
             388  288 - #0096e6
             304  300 - #ffffff
             348  300 - #ffffff
             260  312 - #0096e6
             372  344 - #ffffff
             292  360 - #ffffff
             336  376 - #0096e6
             552  504 - #eac7cd
             576  504 - #ebc4c9
             584  524 - #bf9585
             556  528 - #eac3c8
             584  532 - #ce9c90
             588  532 - #d1a79e
             592  536 - #c79a8d
             588  540 - #d9aca6
             592  544 - #ceaa9b
             592  552 - #d5b9ab
             552  556 - #e9c0c6
             592  560 - #d7bea8
             592  568 - #d3b6a4
             588  572 - #cfae9d
             584  580 - #4f2f21
             592  588 - #4b3024
               4  592 - #597c95
             552  592 - #e5babd
        ",
    )?;

    Ok(())
}

fn check_out_top_right() -> Result<()> {
    inject_touches(
        r"
            365  376  b
            365  376  e
            365  377  b
            365  377  e
            365  377  b
            365  377  e
            365  377  b
            365  377  e
            365  377  b
            365  377  e
            365  377  b
            365  377  e
            365  377  b
            365  377  e
            365  377  b
            365  377  e
            365  377  b
            365  377  e
            365  377  b
            365  377  e
            365  377  b
            365  377  e
            365  377  b
            365  377  e
            299  376  b
            299  376  e
            299  375  b
            300  375  e
        ",
    );

    check_colors(
        r"
               4    4 - #597c95
             468    4 - #f3d6ce
             516    4 - #ac9176
             560    4 - #ffffff
             584    4 - #ffffff
             560    8 - #ffffff
             568    8 - #000000
             576   12 - #010101
             592   12 - #010101
             564   16 - #000000
             464   20 - #f2dace
             532   24 - #af8f7a
             556   32 - #010101
             456   36 - #da9c9f
             488   36 - #d1ae9a
             528   48 - #775e4d
             548   52 - #967c6a
             528   64 - #8f735e
             484   68 - #d8b5a3
             540   68 - #a0826a
             592   76 - #ffffff
             528   80 - #957963
             452   96 - #dda5a6
             500  100 - #d0ab97
             548  100 - #b79883
             360  276 - #ffffff
             260  284 - #0096e6
             372  340 - #ffffff
             260  344 - #0096e6
             316  368 - #0096e6
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_out_top_left() -> Result<()> {
    inject_touches(
        r"
            298  380  b
            298  380  e
            297  380  b
            297  380  e
            297  380  b
            297  380  e
            298  380  b
            298  380  e
            298  380  b
            298  380  e
            298  380  b
            298  380  e
            298  380  b
            298  380  e
            298  380  b
            298  380  e
            298  380  b
            298  380  e
            298  380  b
            298  380  e
        ",
    );

    check_colors(
        r"
               8    4 - #b89a7e
              60    4 - #ffffff
              84    4 - #ffffff
             100    4 - #010101
             592    4 - #597c95
              60    8 - #ffffff
              68    8 - #000000
              76    8 - #dcdcdc
              92    8 - #dcdcdc
              64   16 - #000000
              84   16 - #6d6d6d
             104   16 - #000000
              32   20 - #ab8c77
             108   28 - #010101
              16   32 - #ceab97
              60   32 - #010101
              44   44 - #9c806b
              28   48 - #775e4d
               4   60 - #ceab95
              28   64 - #8f735e
              44   64 - #9f8169
              28   80 - #957963
               4   88 - #d2af9b
              48  100 - #b79883
             148  100 - #ffffff
             260  268 - #0096e6
             360  276 - #ffffff
             304  300 - #ffffff
             272  376 - #0096e6
             388  376 - #0096e6
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_out_left() -> Result<()> {
    inject_touches(
        r"
            303  377  b
            303  377  e
            294  302  b
            294  302  e
            374  293  b
            374  293  e
            373  293  b
            372  293  e
            371  293  b
            372  293  e
            373  293  b
            373  293  e
            373  293  b
            373  293  e
            373  293  b
            373  293  e
        ",
    );

    check_colors(
        r"
             592    4 - #597c95
               4  204 - #dfb7b8
             148  204 - #ffffff
              40  228 - #d2a09f
               4  236 - #e1b3b5
             268  260 - #0096e6
              20  264 - #c9af96
              36  264 - #a2725f
              40  264 - #9d6556
              84  268 - #000000
              36  272 - #955c48
              76  284 - #000000
               4  288 - #ac9176
              80  296 - #ffffff
              88  296 - #ffffff
             372  296 - #ffffff
               8  300 - #b69b7e
              76  308 - #dcdcdc
              92  308 - #dcdcdc
             104  312 - #000000
              32  316 - #b4957e
              72  316 - #6d6d6d
              88  316 - #6d6d6d
              60  328 - #010101
             108  328 - #010101
              28  348 - #775e4d
              44  364 - #9f8169
             260  372 - #0096e6
              28  380 - #957963
               4  396 - #cdaa96
              48  400 - #b79883
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_out_bottom_left() -> Result<()> {
    inject_touches(
        r"
            372  364  b
            372  364  e
            381  275  b
            381  275  e
            378  293  b
            378  293  e
            378  293  b
            378  293  e
            378  293  b
            378  293  e
            378  293  b
            378  293  e
            378  292  b
            378  292  e
            378  292  b
            378  292  e
        ",
    );

    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             348  260 - #0096e6
             276  300 - #ffffff
             388  332 - #0096e6
             316  376 - #0096e6
               4  504 - #dfb7b8
              48  504 - #d7a9ab
             148  504 - #ffffff
              40  528 - #d2a09f
               4  536 - #e1b3b5
              48  540 - #d09fa0
              40  556 - #986955
              20  564 - #c9af96
              36  564 - #a2725f
              40  564 - #9d6556
             148  564 - #ffffff
              36  568 - #9b6853
              84  568 - #000000
              20  572 - #b29680
              36  572 - #955c48
              80  576 - #000000
              88  576 - #000000
              92  580 - #000000
              24  584 - #b09077
              76  584 - #000000
               4  588 - #ac9176
              48  588 - #cc989a
              28  592 - #b8987f
              72  592 - #000000
              96  592 - #000000
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
