use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLUE, Container, GREEN, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct CenterField {
    field: Weak<Container>,

    #[init]
    container: Container,
}

impl Setup for CenterField {
    fn setup(mut self: Weak<Self>) {
        self.container.set_color(GREEN);
        self.container.place().all_sides(100);

        self.field = self.container.add_view();

        self.field.set_color(BLUE);
        self.field.place().lr(20).h(68);
    }
}

impl ViewTest for CenterField {
    fn perform_test(view: Weak<Self>) -> anyhow::Result<()> {
        check_initial_layout()?;
        check_all_sides_200(view)?;
        check_max_width(view)?;
        check_all_sides_100(view)?;
        check_center_x(view)?;
        check_center_y_offset(view)?;

        Ok(())
    }
}

fn check_initial_layout() -> anyhow::Result<()> {
    check_colors(
        r"
               4    4 - #597c95
             592    4 - #597c95
             300   96 - #597c95
             176  104 - #0000e7
             240  104 - #0000e7
             360  104 - #0000e7
             424  104 - #0000e7
             124  108 - #0000e7
             124  164 - #0000e7
             224  164 - #0000e7
             296  164 - #0000e7
             376  164 - #0000e7
             476  164 - #0000e7
               4  196 - #597c95
             592  220 - #597c95
             108  276 - #00ff00
             312  276 - #00ff00
             472  292 - #00ff00
               4  320 - #597c95
             200  336 - #00ff00
             592  348 - #597c95
             288  368 - #00ff00
             380  384 - #00ff00
             104  388 - #00ff00
             252  464 - #00ff00
             496  476 - #00ff00
             104  496 - #00ff00
             364  496 - #00ff00
               4  592 - #597c95
             304  592 - #597c95
             448  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_all_sides_200(view: Weak<CenterField>) -> anyhow::Result<()> {
    from_main(move || {
        view.container.place().clear().all_sides(200);
    });

    check_colors(
        r"
               4    4 - #597c95
             208    4 - #597c95
             592    4 - #597c95
             400   12 - #597c95
             300  196 - #597c95
             236  204 - #0000e7
             364  204 - #0000e7
             268  208 - #0000e7
             332  208 - #0000e7
             592  224 - #597c95
               4  228 - #597c95
             304  232 - #0000e7
             260  240 - #0000e7
             336  240 - #0000e7
             224  264 - #0000e7
             296  264 - #0000e7
             376  264 - #0000e7
             264  300 - #00ff00
             316  300 - #00ff00
             396  308 - #00ff00
             352  320 - #00ff00
             228  332 - #00ff00
             296  340 - #00ff00
             356  360 - #00ff00
             396  368 - #00ff00
             204  396 - #00ff00
             268  396 - #00ff00
             332  396 - #00ff00
             196  588 - #597c95
               4  592 - #597c95
             388  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_max_width(view: Weak<CenterField>) -> anyhow::Result<()> {
    from_main(move || {
        view.container.place().clear().all_sides(250);
        view.field.place().max_width(200);
    });

    check_colors(
        r"
               4    4 - #597c95
             252    4 - #597c95
             592    4 - #597c95
             420   88 - #597c95
             300  248 - #597c95
             252  252 - #00ff00
             328  252 - #0000e7
               4  256 - #597c95
             276  268 - #0000e7
             296  268 - #0000e7
             348  268 - #00ff00
             328  276 - #0000e7
             308  284 - #0000e7
             252  288 - #00ff00
             280  292 - #0000e7
             324  296 - #0000e7
             348  296 - #00ff00
             304  308 - #0000e7
             592  308 - #597c95
             284  316 - #0000e7
             328  316 - #0000e7
             256  320 - #00ff00
             348  324 - #00ff00
             276  340 - #00ff00
             324  340 - #00ff00
             252  348 - #00ff00
             304  348 - #00ff00
             348  348 - #00ff00
              84  424 - #597c95
               4  592 - #597c95
             280  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_all_sides_100(view: Weak<CenterField>) -> anyhow::Result<()> {
    from_main(move || {
        view.container.place().clear().all_sides(100);
    });

    check_colors(
        r"
               4    4 - #597c95
             432    4 - #597c95
             592    4 - #597c95
             300   96 - #597c95
             176  104 - #0000e7
             240  104 - #0000e7
             124  108 - #0000e7
             420  116 - #00ff00
             208  128 - #0000e7
             316  128 - #0000e7
             272  144 - #0000e7
             176  156 - #0000e7
             124  164 - #0000e7
             224  164 - #0000e7
             316  164 - #0000e7
               4  196 - #597c95
             496  200 - #00ff00
             392  264 - #00ff00
             284  268 - #00ff00
             108  276 - #00ff00
               4  324 - #597c95
             492  324 - #00ff00
             204  344 - #00ff00
             380  384 - #00ff00
             592  384 - #597c95
             264  464 - #00ff00
             104  496 - #00ff00
             476  496 - #00ff00
               4  592 - #597c95
             180  592 - #597c95
             304  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_center_x(view: Weak<CenterField>) -> anyhow::Result<()> {
    from_main(move || {
        view.field.place().center_x();
    });

    check_colors(
        r"
               4    4 - #597c95
             140    4 - #597c95
             592    4 - #597c95
             300   96 - #597c95
             232  104 - #0000e7
             368  104 - #0000e7
             496  104 - #00ff00
             268  116 - #0000e7
             204  128 - #0000e7
             304  136 - #0000e7
              44  144 - #597c95
             240  144 - #0000e7
             336  156 - #0000e7
             204  164 - #0000e7
             276  164 - #0000e7
             396  164 - #0000e7
             592  176 - #597c95
             128  264 - #00ff00
               4  284 - #597c95
             356  296 - #00ff00
             496  316 - #00ff00
             228  348 - #00ff00
             116  380 - #00ff00
             404  400 - #00ff00
             224  468 - #00ff00
             492  472 - #00ff00
             104  496 - #00ff00
             340  496 - #00ff00
               4  592 - #597c95
             224  592 - #597c95
             432  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_center_y_offset(view: Weak<CenterField>) -> anyhow::Result<()> {
    from_main(move || {
        view.field.place().center_y_offset(-50);
    });

    check_colors(
        r"
               4    4 - #597c95
             192    4 - #597c95
             428    4 - #597c95
             592    4 - #597c95
             300   96 - #597c95
             400  112 - #00ff00
             124  128 - #00ff00
             496  144 - #00ff00
             208  220 - #0000e7
             268  220 - #0000e7
             316  220 - #0000e7
             396  220 - #0000e7
             592  232 - #597c95
             104  244 - #00ff00
             356  244 - #0000e7
             252  268 - #0000e7
             204  280 - #0000e7
             300  280 - #0000e7
             340  280 - #0000e7
             396  280 - #0000e7
               4  316 - #597c95
             496  316 - #00ff00
             116  364 - #00ff00
             404  396 - #00ff00
             224  420 - #00ff00
             492  472 - #00ff00
             104  496 - #00ff00
             340  496 - #00ff00
               4  592 - #597c95
             224  592 - #597c95
             432  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
