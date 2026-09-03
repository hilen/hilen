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
             592    4 - #597c95
             268  260 - #0096e6
             380  260 - #0096e6
               4  360 - #597c95
             288  360 - #ffffff
             388  372 - #0096e6
             200  504 - #eac6cb
             280  504 - #dcb1b2
             396  504 - #ffffff
             232  528 - #b58375
             236  528 - #c7968c
             268  532 - #deb0b0
             296  536 - #d19f9f
             252  556 - #c5a992
             288  556 - #a0715d
             292  560 - #9d705b
             200  564 - #e7bfbf
             288  564 - #a46957
             332  564 - #000000
             396  564 - #ffffff
             284  572 - #9a6854
             288  572 - #9f6451
             336  576 - #000000
             276  580 - #a8846e
             324  580 - #000000
             340  584 - #000000
             240  588 - #472f1e
             264  588 - #362215
             260  592 - #51462d
             296  592 - #c99796
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
             312    4 - #597c95
             592    4 - #597c95
             336  260 - #0096e6
               4  268 - #597c95
             292  280 - #ffffff
             372  296 - #fdfeff
             276  300 - #fefeff
             260  328 - #0096e6
             348  340 - #ffffff
             296  352 - #ffffff
             388  352 - #0096e6
             260  372 - #0096e6
             336  376 - #0096e6
             552  504 - #ebc7cd
             588  504 - #e8c1c2
             584  532 - #cc998c
             552  536 - #eec7cc
             580  536 - #ce8c77
             592  540 - #cea296
             580  544 - #c57f6f
             592  548 - #cca996
             560  556 - #e6bebf
             592  556 - #d5beab
             580  564 - #dabdaa
             588  564 - #d7bbad
             592  572 - #c8ac95
             552  580 - #e6bcbe
             580  580 - #dabfa7
             584  588 - #c0a58b
             592  588 - #442c1d
               4  592 - #597c95
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
             468    4 - #f3d8cf
             492    4 - #cea78a
             560    4 - #ffffff
             584    4 - #ffffff
             512    8 - #a58368
             560    8 - #ffffff
             568    8 - #000000
             576   12 - #010101
             592   12 - #010101
             564   16 - #000000
             464   20 - #f4dbd1
             532   24 - #af8f7a
             556   32 - #010101
             492   36 - #c9a58d
             528   48 - #846857
             452   52 - #e2a7a9
             516   56 - #cbaa97
             548   56 - #967a66
             496   72 - #d1ac99
             528   72 - #8a6e59
             528   92 - #9c7c65
             452   96 - #dea6a6
             492   96 - #d2a391
             592  100 - #ffffff
             388  264 - #0096e6
             288  280 - #ffffff
             344  312 - #0096e6
             280  348 - #fdfeff
             344  376 - #0096e6
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
              60    4 - #ffffff
              84    4 - #ffffff
             100    4 - #010101
             592    4 - #597c95
              12    8 - #a58368
              60    8 - #ffffff
              68    8 - #000000
              76    8 - #dcdcdc
              92    8 - #dcdcdc
              64   16 - #000000
              84   16 - #6d6d6d
             104   16 - #000000
              36   20 - #aa8d78
             108   28 - #010101
              60   32 - #010101
              12   44 - #cbaf96
              48   48 - #9b7f6a
              28   52 - #876d5a
               4   64 - #ceae99
              40   64 - #a0846c
               4   80 - #cfac98
              28   84 - #94775f
              96   84 - #ffffff
               8   96 - #caaa95
              44  100 - #bfa08e
             148  100 - #ffffff
             260  268 - #0096e6
             360  276 - #ffffff
             272  376 - #0096e6
             384  376 - #0096e6
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
             328    4 - #597c95
             592    4 - #597c95
               4  204 - #dfb7b8
             148  204 - #ffffff
              48  216 - #d7a9ab
               4  248 - #c9a99a
              44  260 - #af8271
             268  260 - #0096e6
              40  264 - #a06756
              84  264 - #000000
              88  276 - #000000
              32  288 - #a47e64
              12  292 - #413828
              80  296 - #ffffff
              88  296 - #ffffff
             372  296 - #fdfeff
              12  308 - #a58368
              76  308 - #dcdcdc
              92  308 - #dcdcdc
             104  312 - #000000
              72  316 - #6d6d6d
              88  316 - #6d6d6d
             108  328 - #010101
              56  332 - #010101
              28  348 - #846857
              48  348 - #9b7f6a
              28  372 - #8a6e59
             260  372 - #0096e6
              16  400 - #b38f7c
              44  400 - #bfa08e
             148  400 - #ffffff
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
             276  300 - #fefeff
             388  332 - #0096e6
             316  376 - #0096e6
               4  504 - #dfb7b8
              28  504 - #dcb1b2
             148  504 - #ffffff
              48  512 - #d9abad
              32  536 - #d49e9e
              48  540 - #d29fa0
               4  548 - #c9a99a
              28  560 - #b9927d
              40  560 - #a16d5a
              44  560 - #af8271
              40  564 - #a06756
             148  564 - #ffffff
              16  568 - #b39980
              40  568 - #9d6451
              84  568 - #000000
              36  572 - #97614c
              80  576 - #000000
              92  580 - #000000
               4  584 - #b39478
              28  584 - #98735c
              76  584 - #000000
              32  588 - #a47e64
              12  592 - #413828
              72  592 - #000000
              96  592 - #000000
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
