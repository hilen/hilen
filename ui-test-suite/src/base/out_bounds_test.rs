use anyhow::Result;
use test_engine::{
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
             264  264 - #0096e6
             388  264 - #0096e6
               4  296 - #597c95
             332  304 - #0096e6
             288  360 - #ffffff
             388  372 - #0096e6
             204  504 - #eac5cc
             276  504 - #dab0b1
             336  504 - #ffffff
             396  504 - #ffffff
             240  508 - #e7bfc0
             296  508 - #d7a9ab
             296  532 - #d3a1a2
             220  540 - #e6bebe
             260  544 - #ddadad
             396  560 - #ffffff
             288  564 - #a26757
             328  564 - #000000
             336  564 - #000000
             328  568 - #000000
             296  572 - #cb9998
             324  576 - #000000
             344  576 - #000000
             204  580 - #e6bcbd
             296  584 - #ca9899
             320  588 - #000000
             336  588 - #ffffff
             276  592 - #ba9a81
             348  592 - #000000
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
             324    4 - #597c95
             592    4 - #597c95
             592  216 - #597c95
             264  264 - #0096e6
             344  264 - #0096e6
             388  264 - #0096e6
             360  280 - #ffffff
             292  284 - #ffffff
               4  288 - #597c95
             324  300 - #597c95
             368  300 - #ffffff
             264  308 - #0096e6
             264  340 - #0096e6
             296  340 - #ffffff
             348  340 - #ffffff
             372  340 - #ffffff
             288  360 - #ffffff
             360  360 - #ffffff
             388  372 - #0096e6
             264  376 - #0096e6
             316  376 - #0096e6
             552  504 - #eac7cd
             576  504 - #ebc4c9
             592  524 - #e6bebe
             568  528 - #e7bfc0
             552  548 - #eac3c8
             572  548 - #e3bbbc
             552  572 - #e7bfc0
             572  588 - #f2d6cb
               4  592 - #597c95
             224  592 - #597c95
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
             556    4 - #ffffff
             572    4 - #000000
             592    4 - #000000
             556    8 - #ffffff
             564    8 - #000000
             464   12 - #f3dcd4
             560   16 - #000000
             468   20 - #f0d8cc
             520   20 - #c7a18a
             464   24 - #f1d6cd
             556   24 - #000000
             500   44 - #c7a58c
             540   44 - #a4846f
             452   52 - #e3a7a9
             536   56 - #9f836d
             592   68 - #ffffff
             476   72 - #e6c1b8
             532   80 - #aa8c74
             500   92 - #d3ae9b
             460   96 - #dba1a0
             544   96 - #b99d87
             388  264 - #0096e6
             288  280 - #ffffff
             360  284 - #ffffff
               4  300 - #597c95
             344  312 - #0096e6
             388  340 - #0096e6
             284  352 - #ffffff
             352  376 - #0096e6
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
              56    4 - #ffffff
              72    4 - #000000
              88    4 - #000000
              96    4 - #000000
             360    4 - #597c95
             592    4 - #597c95
              56    8 - #ffffff
             104    8 - #000000
              60   16 - #000000
             108   16 - #000000
              20   20 - #c7a18a
              56   24 - #000000
             112   24 - #000000
               8   44 - #c9a992
              40   44 - #a4846f
              44   56 - #9c8068
              36   64 - #a2866e
             100   76 - #ffffff
              32   80 - #aa8c74
               8   92 - #ccac97
              48   96 - #b49882
             148   96 - #ffffff
             316  264 - #0096e6
             360  276 - #ffffff
             280  292 - #ffffff
             388  324 - #0096e6
             332  336 - #0096e6
              24  344 - #597c95
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
             224    4 - #597c95
             592    4 - #597c95
               4  204 - #dfb7b8
              48  204 - #d7a9ab
             148  204 - #ffffff
              32  232 - #d29f9e
               4  236 - #e1b3b5
              84  260 - #000000
             332  264 - #0096e6
              76  276 - #000000
              80  288 - #ffffff
              88  288 - #ffffff
             284  288 - #ffffff
              36  292 - #ca9897
             372  300 - #ffffff
              64  304 - #000000
             104  304 - #000000
             108  316 - #000000
              60  324 - #000000
             112  324 - #000000
             308  336 - #0096e6
              16  340 - #c7a991
              40  344 - #a4846f
              36  364 - #a2866e
              48  376 - #b59983
             264  376 - #0096e6
             344  376 - #0096e6
              48  384 - #b69a84
               4  388 - #d2af9b
             148  396 - #ffffff
             164  592 - #597c95
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
             276    4 - #597c95
             592    4 - #597c95
              28  256 - #597c95
             344  264 - #0096e6
             276  300 - #ffffff
             388  328 - #0096e6
             264  356 - #0096e6
             316  376 - #0096e6
               4  504 - #dfb7b8
              96  504 - #ffffff
             148  504 - #ffffff
              48  512 - #d7a9ab
              24  520 - #ddafb1
               4  536 - #e1b3b5
              48  540 - #d0a0a0
              28  552 - #d59f9d
              84  560 - #000000
             148  560 - #ffffff
              48  568 - #cc9a99
              88  568 - #000000
              80  572 - #000000
              92  572 - #000000
              76  576 - #000000
              72  580 - #000000
              96  580 - #000000
              80  588 - #ffffff
              88  588 - #ffffff
              36  592 - #ca9897
              68  592 - #000000
             100  592 - #000000
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
