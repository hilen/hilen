use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Container, GREEN, MovableView, Setup, ViewData, ViewFrame, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

#[view]
struct MovableViewTestView {
    #[init]
    movable: MovableView<Container>,
}

impl Setup for MovableViewTestView {
    fn setup(mut self: Weak<Self>) {
        self.movable.set_title("Movable view");
        self.movable.set_frame((10, 10, 400, 400));
        self.movable.target_view.set_color(GREEN);
    }
}

impl ViewTest for MovableViewTestView {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_dragged_by_title()?;
        check_resized_from_corner()?;
        check_resized_to_min_size()?;

        Ok(())
    }
}

fn check_dragged_by_title() -> Result<()> {
    inject_touches(
        "
            346  36   b
            438  90   m
            438  90   e
        ",
    );

    check_colors(
        r"
             104   64 - #ffffff
             500   64 - #ffffff
             304   72 - #3e3e3e
             228   76 - #000000
             288   80 - #000000
             352   80 - #757575
             208   84 - #000000
             244   84 - #ffffff
             320   84 - #f0f0f0
             352   84 - #757575
             372   84 - #0a0a0a
             352   88 - #757575
             228   92 - #000000
             264   92 - #ffffff
             280   92 - #010101
             300   92 - #ffffff
             352   92 - #757575
             392   92 - #000000
             452  100 - #ffffff
               4  232 - #597c95
             160  232 - #00ff00
             592  252 - #597c95
             384  288 - #00ff00
             208  380 - #00ff00
             500  440 - #444444
             336  448 - #00ff00
             492  452 - #444444
             496  452 - #444444
             492  456 - #444444
             480  460 - #444444
               4  592 - #597c95
             292  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_resized_from_corner() -> Result<()> {
    inject_touches(
        "
            501  458  b
            323  192  m
            323  192  e
        ",
    );

    check_colors(
        r"
             172   64 - #ffffff
             244   64 - #ffffff
             280   64 - #ffffff
             320   64 - #ffffff
             216   72 - #000000
             140   76 - #000000
             124   80 - #000000
             248   80 - #000000
             192   84 - #8a8a8a
             156   88 - #ffffff
             192   88 - #8a8a8a
             224   88 - #050505
             280   88 - #ffffff
             120   92 - #000000
             136   92 - #ffffff
             252   92 - #000000
             264   92 - #000000
             296   92 - #ffffff
             176  144 - #00ff00
             104  148 - #00ff00
             228  176 - #00ff00
             320  176 - #444444
             312  188 - #444444
             316  188 - #444444
             312  192 - #444444
             144  196 - #00ff00
             300  196 - #444444
             320  196 - #444444
             592  300 - #597c95
             300  504 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_resized_to_min_size() -> Result<()> {
    inject_touches(
        "
            313  190  b
            78   78   m
            78   78   e
        ",
    );

    check_colors(
        r"
             592    4 - #597c95
             116   64 - #ffffff
             152   64 - #ffffff
             176   64 - #ffffff
             200   64 - #ffffff
             136   72 - #060606
             148   72 - #ffffff
             104   80 - #010101
             156   80 - #000000
             140   84 - #ffffff
             148   84 - #010101
             168   84 - #f0f0f0
             188   84 - #000000
             120   88 - #000000
             104   92 - #ffffff
             136   92 - #060606
             156   92 - #000000
             196   92 - #ffffff
             176  116 - #00ff00
             120  124 - #00ff00
             152  136 - #00ff00
             200  140 - #444444
             192  152 - #444444
             196  152 - #444444
             192  156 - #444444
             112  160 - #00ff00
             180  160 - #444444
             188  160 - #444444
             200  160 - #444444
             300  464 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
