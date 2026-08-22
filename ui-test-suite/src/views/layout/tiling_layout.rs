use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLACK, BLUE, Button, Container, GREEN, RED, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct TilingLayout {
    #[init]
    menu: Container,
}

impl Setup for TilingLayout {
    fn setup(self: Weak<Self>) {
        self.menu.set_color(BLACK).place().tl(20).size(280, 280).all_ver();

        self.menu.add_view::<Container>().set_color(RED);
        self.menu.add_view::<Container>().set_color(GREEN);
        self.menu.add_view::<Container>().set_color(BLUE);
    }
}

impl ViewTest for TilingLayout {
    fn perform_test(view: Weak<Self>) -> anyhow::Result<()> {
        check_initial_tiles()?;
        check_empty_menu(view)?;
        check_button_tiles(view)?;

        Ok(())
    }
}

fn check_initial_tiles() -> anyhow::Result<()> {
    check_colors(
        r"
             484    4 - #597c95
              24   20 - #ff0000
             140   20 - #ff0000
             296   20 - #ff0000
              80   24 - #ff0000
             220   36 - #ff0000
              84   80 - #ff0000
             148   92 - #ff0000
             292  104 - #ff0000
              20  116 - #00ff00
             208  116 - #00ff00
             108  128 - #00ff00
             252  152 - #00ff00
             592  156 - #597c95
              68  160 - #00ff00
             148  168 - #00ff00
             296  188 - #00ff00
              20  204 - #00ff00
             216  208 - #0000e7
             136  224 - #0000e7
              84  240 - #0000e7
             268  244 - #0000e7
             160  272 - #0000e7
              20  296 - #0000e7
             104  296 - #0000e7
             228  296 - #0000e7
             300  300 - #597c95
             532  372 - #597c95
               4  476 - #597c95
             368  524 - #597c95
             144  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_empty_menu(view: Weak<TilingLayout>) -> anyhow::Result<()> {
    from_main(move || {
        view.menu.remove_all_subviews();
    });

    check_colors(
        r"
             444    4 - #597c95
             592    4 - #597c95
              24   20 - #000000
             228   20 - #000000
             160   24 - #000000
             296   28 - #000000
              20   92 - #000000
              92   92 - #000000
             228   96 - #000000
             296   96 - #000000
             456  152 - #597c95
              24  160 - #000000
             592  160 - #597c95
             160  164 - #000000
             228  164 - #000000
             296  164 - #000000
              92  232 - #000000
             236  232 - #000000
              20  296 - #000000
             168  296 - #000000
             300  300 - #597c95
             592  300 - #597c95
             448  372 - #597c95
             108  408 - #597c95
             228  448 - #597c95
             572  448 - #597c95
               4  484 - #597c95
             376  516 - #597c95
             472  588 - #597c95
             156  592 - #597c95
             280  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}

fn check_button_tiles(view: Weak<TilingLayout>) -> anyhow::Result<()> {
    from_main(move || {
        view.menu
            .add_view::<Button>()
            .add_transition::<TilingLayout, TilingLayout>()
            .set_text("Classic")
            .set_text_size(80);

        view.menu
            .add_view::<Button>()
            .add_transition::<TilingLayout, TilingLayout>()
            .set_text("Custom Game")
            .set_text_size(80);

        view.menu
            .add_view::<Button>()
            .add_transition::<TilingLayout, TilingLayout>()
            .set_text("Settings")
            .set_text_size(80);
    });

    check_colors(
        r"
             592    4 - #597c95
              88   36 - #b1b1b1
             168   64 - #ffffff
             180   64 - #d5d5d5
             220   64 - #d5d5d5
             128   68 - #636363
              20   76 - #ffffff
             132   76 - #7b7b7b
             284   80 - #a7a7a7
              88   92 - #b1b1b1
              20  156 - #ffffff
             268  156 - #343434
             296  160 - #e8e8e8
             152  164 - #626262
             212  172 - #000000
             252  176 - #6c6c6c
             296  176 - #e8e8e8
             124  180 - #010101
              64  184 - #000000
             116  228 - #242424
             140  228 - #242424
              52  240 - #cacaca
              96  252 - #010101
             188  256 - #9a9a9a
             188  276 - #9a9a9a
             164  280 - #535353
             212  280 - #535353
             288  280 - #010101
             244  288 - #ffffff
              24  296 - #ffffff
             128  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
