use test_engine::{
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
             472    4 - #597c95
              44   24 - #ff0000
             140   24 - #ff0000
             236   24 - #ff0000
             288   24 - #ff0000
              92   44 - #ff0000
             216   72 - #ff0000
             104   96 - #ff0000
             164  108 - #ff0000
              24  116 - #00ff00
             280  116 - #00ff00
             592  144 - #597c95
             248  156 - #00ff00
             196  160 - #00ff00
              48  168 - #00ff00
             104  176 - #00ff00
             296  192 - #00ff00
             236  204 - #00ff00
              24  224 - #0000e7
             140  236 - #0000e7
              80  240 - #0000e7
             264  256 - #0000e7
             204  264 - #0000e7
              28  296 - #0000e7
             104  296 - #0000e7
             160  296 - #0000e7
             304  300 - #597c95
             528  368 - #597c95
             152  476 - #597c95
               4  592 - #597c95
             300  592 - #597c95
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
             592    4 - #597c95
             444    8 - #597c95
              24   24 - #000000
             228   24 - #000000
             296   28 - #000000
             160   32 - #000000
              92   36 - #000000
              32   92 - #000000
             228   96 - #000000
             100  104 - #000000
             480  152 - #597c95
              28  160 - #000000
             296  160 - #000000
             168  168 - #000000
              36  228 - #000000
             236  228 - #000000
             104  236 - #000000
              32  296 - #000000
             176  296 - #000000
             592  296 - #597c95
             304  300 - #597c95
             448  300 - #597c95
             592  440 - #597c95
               4  444 - #597c95
             296  444 - #597c95
             452  444 - #597c95
             152  456 - #597c95
             444  588 - #597c95
               4  592 - #597c95
             160  592 - #597c95
             300  592 - #597c95
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
              24   24 - #ffffff
             240   24 - #ffffff
              72   36 - #010101
             116   56 - #000000
             212   56 - #ffffff
             172   76 - #ffffff
             280   80 - #010101
              60   92 - #ffffff
             116  120 - #ffffff
             220  128 - #000000
              80  144 - #000000
              24  148 - #ffffff
             284  148 - #ffffff
             160  164 - #ffffff
             216  176 - #000000
              92  180 - #000000
             260  208 - #ffffff
             144  224 - #000000
             200  236 - #000000
              88  240 - #ffffff
              32  244 - #000000
             268  256 - #ffffff
             296  264 - #010101
             168  268 - #000000
             124  272 - #010101
             240  280 - #ffffff
             588  300 - #597c95
             428  416 - #597c95
             300  568 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ",
    )?;

    Ok(())
}
