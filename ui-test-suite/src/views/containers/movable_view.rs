use anyhow::Result;
use test_engine::{
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
               4    4 - #597c95
             160   68 - #ffffff
             420   68 - #ffffff
             500   68 - #ffffff
             316   80 - #ffffff
             292   88 - #ffffff
             224   92 - #ffffff
             252   92 - #ffffff
             348   92 - #ffffff
             388   92 - #ffffff
             104  100 - #ffffff
             184  100 - #ffffff
             448  100 - #ffffff
             104  188 - #00ff00
             248  236 - #00ff00
             592  256 - #597c95
             380  288 - #00ff00
             204  372 - #00ff00
               4  388 - #597c95
             500  440 - #444444
             496  444 - #444444
             332  448 - #00ff00
             492  452 - #444444
             496  452 - #444444
             484  456 - #444444
             492  456 - #444444
             112  460 - #00ff00
             480  460 - #444444
             500  460 - #444444
               4  592 - #597c95
             284  592 - #597c95
             592  592 - #597c95
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
             592    4 - #597c95
             152   68 - #ffffff
             256   68 - #ffffff
             320   68 - #ffffff
             204   72 - #ffffff
             184   88 - #ffffff
             124   92 - #ffffff
             164   92 - #ffffff
             220   92 - #ffffff
             288   92 - #ffffff
             104  100 - #ffffff
             252  100 - #ffffff
             320  100 - #ffffff
             204  140 - #00ff00
             108  148 - #00ff00
             256  148 - #00ff00
             156  152 - #00ff00
             320  176 - #444444
             312  188 - #444444
             316  188 - #444444
             304  192 - #444444
             312  192 - #444444
             104  196 - #00ff00
             208  196 - #00ff00
             300  196 - #444444
             308  196 - #444444
             320  196 - #444444
             592  300 - #597c95
             108  408 - #597c95
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
             140   68 - #ffffff
             180   68 - #ffffff
             200   68 - #ffffff
             104   72 - #ffffff
             148   72 - #ffffff
             124   80 - #ffffff
             168   80 - #ffffff
             144   84 - #ffffff
             112   92 - #ffffff
             196   92 - #ffffff
             140   96 - #ffffff
             164  100 - #ffffff
             184  116 - #00ff00
             104  120 - #00ff00
             156  120 - #00ff00
             132  132 - #00ff00
             200  140 - #444444
             200  148 - #444444
             192  152 - #444444
             196  152 - #444444
             192  156 - #444444
             104  160 - #00ff00
             140  160 - #00ff00
             180  160 - #444444
             188  160 - #444444
             200  160 - #444444
             544  296 - #597c95
               4  364 - #597c95
             300  464 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
