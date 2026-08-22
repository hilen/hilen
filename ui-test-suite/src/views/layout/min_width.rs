use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLUE, Container, GREEN, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct MinWidth {
    view: Weak<Container>,

    #[init]
    container: Container,
}

impl Setup for MinWidth {
    fn setup(mut self: Weak<Self>) {
        self.container.set_color(GREEN);
        self.container.set_size(400, 400).set_position((20, 20));

        self.view = self.container.add_view();
        self.view.set_color(BLUE);
        self.view.place().all_sides(150);
    }
}

impl ViewTest for MinWidth {
    fn perform_test(view: Weak<Self>) -> anyhow::Result<()> {
        check_initial_layout()?;
        check_min_width_center_x(view)?;
        check_min_height_center_y(view)?;

        Ok(())
    }
}

fn check_initial_layout() -> anyhow::Result<()> {
    check_colors(
        r"
             500    4 - #597c95
              24   24 - #00ff00
             176   24 - #00ff00
             356   24 - #00ff00
             264   72 - #00ff00
             104  108 - #00ff00
             592  112 - #597c95
             416  156 - #00ff00
             196  172 - #0000e7
             268  172 - #0000e7
             232  180 - #0000e7
              24  184 - #00ff00
             172  192 - #0000e7
             196  212 - #0000e7
             232  220 - #0000e7
             268  228 - #0000e7
             172  240 - #0000e7
             560  252 - #597c95
             212  268 - #0000e7
             264  268 - #0000e7
              72  284 - #00ff00
             424  300 - #597c95
              24  384 - #00ff00
             592  396 - #597c95
             328  404 - #00ff00
             188  416 - #00ff00
             444  480 - #597c95
             148  560 - #597c95
             296  564 - #597c95
               4  592 - #597c95
             412  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_min_width_center_x(view: Weak<MinWidth>) -> anyhow::Result<()> {
    from_main(move || {
        view.view.place().min_width(250).center_x();
    });

    check_colors(
        r"
             480    4 - #597c95
              24   24 - #00ff00
             172   24 - #00ff00
             320   24 - #00ff00
             592   76 - #597c95
             100   80 - #00ff00
             248   88 - #00ff00
             416  120 - #00ff00
              96  172 - #0000e7
             200  172 - #0000e7
             300  188 - #0000e7
             148  204 - #0000e7
             248  204 - #0000e7
             200  232 - #0000e7
              96  236 - #0000e7
             572  236 - #597c95
             152  268 - #0000e7
             248  268 - #0000e7
             340  268 - #0000e7
              24  276 - #00ff00
             424  300 - #597c95
             368  384 - #00ff00
             148  396 - #00ff00
             592  396 - #597c95
              24  404 - #00ff00
             272  416 - #00ff00
             448  452 - #597c95
              96  508 - #597c95
             196  588 - #597c95
               4  592 - #597c95
             384  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_min_height_center_y(view: Weak<MinWidth>) -> anyhow::Result<()> {
    from_main(move || {
        view.view.place().min_height(250).center_y();
    });

    check_colors(
        r"
             592    4 - #597c95
              24   24 - #00ff00
             128   24 - #00ff00
             228   24 - #00ff00
             416   24 - #00ff00
             308   40 - #00ff00
             200  100 - #0000e7
             528  108 - #597c95
              24  116 - #00ff00
             316  140 - #0000e7
             416  148 - #00ff00
              96  168 - #0000e7
             176  176 - #0000e7
             592  208 - #597c95
             248  220 - #0000e7
             340  228 - #0000e7
             148  252 - #0000e7
              24  256 - #00ff00
             424  300 - #597c95
             312  312 - #0000e7
             208  332 - #0000e7
             100  340 - #0000e7
             592  396 - #597c95
              24  416 - #00ff00
             276  416 - #00ff00
             396  416 - #00ff00
             148  464 - #597c95
             500  496 - #597c95
             196  588 - #597c95
               4  592 - #597c95
             384  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
