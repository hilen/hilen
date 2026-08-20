use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::Weak,
    ui::{GRAY_BLUE, GREEN, NoImage, Setup, UIImages, UIManager, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct Background {}

impl Setup for Background {
    fn setup(self: Weak<Self>) {
        UIManager::set_clear_color(GREEN);
    }
}

impl ViewTest for Background {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        green_clear_color()?;
        gray_blue_clear_color()?;
        root_view_image()?;
        image_removed()?;

        Ok(())
    }
}

fn green_clear_color() -> Result<()> {
    check_colors(
        r"
               4    4 - #00ff00
             444    4 - #00ff00
             592    4 - #00ff00
             296    8 - #00ff00
             148   12 - #00ff00
             228   84 - #00ff00
              12  148 - #00ff00
             444  152 - #00ff00
             592  152 - #00ff00
             156  156 - #00ff00
             300  156 - #00ff00
              84  228 - #00ff00
             228  228 - #00ff00
             372  228 - #00ff00
               8  296 - #00ff00
             448  296 - #00ff00
             156  300 - #00ff00
             300  300 - #00ff00
             592  300 - #00ff00
             228  372 - #00ff00
             372  372 - #00ff00
             516  372 - #00ff00
               4  444 - #00ff00
             152  444 - #00ff00
             444  444 - #00ff00
             296  448 - #00ff00
             588  448 - #00ff00
             448  588 - #00ff00
               4  592 - #00ff00
             152  592 - #00ff00
             300  592 - #00ff00
             592  592 - #00ff00
        ",
    )
}

fn gray_blue_clear_color() -> Result<()> {
    from_main(|| {
        UIManager::set_clear_color(GRAY_BLUE);
    });

    check_colors(
        r"
               4    4 - #597c95
             444    4 - #597c95
             592    4 - #597c95
             296    8 - #597c95
             148   12 - #597c95
             228   84 - #597c95
              12  148 - #597c95
             444  152 - #597c95
             592  152 - #597c95
             156  156 - #597c95
             300  156 - #597c95
              84  228 - #597c95
             228  228 - #597c95
             372  228 - #597c95
               8  296 - #597c95
             448  296 - #597c95
             156  300 - #597c95
             300  300 - #597c95
             592  300 - #597c95
             228  372 - #597c95
             372  372 - #597c95
             516  372 - #597c95
               4  444 - #597c95
             152  444 - #597c95
             444  444 - #597c95
             296  448 - #597c95
             588  448 - #597c95
             448  588 - #597c95
               4  592 - #597c95
             152  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn root_view_image() -> Result<()> {
    from_main(|| {
        UIManager::root_view().set_image(UIImages::up());
    });

    check_colors(
        r"
               4    4 - #597c95
              24    4 - #597c95
             196    4 - #0096e6
             400    4 - #0096e6
             576    4 - #597c95
             592    4 - #597c95
              20   16 - #0096e6
             592   20 - #597c95
               4   24 - #597c95
             120  148 - #0096e6
             284  156 - #ffffff
             592  184 - #0096e6
             244  228 - #ffffff
             372  256 - #ffffff
               4  260 - #0096e6
             476  296 - #0096e6
             272  300 - #ffffff
             188  328 - #ffffff
             388  340 - #ffffff
             272  388 - #ffffff
             452  396 - #ffffff
             188  416 - #ffffff
             356  416 - #ffffff
              20  424 - #0096e6
             592  432 - #0096e6
             432  556 - #0096e6
               4  576 - #597c95
             592  580 - #597c95
             588  588 - #597c95
               4  592 - #597c95
              20  592 - #597c95
             272  592 - #0096e6
        ",
    )
}

fn image_removed() -> Result<()> {
    from_main(|| {
        UIManager::root_view().set_image(NoImage);
    });

    check_colors(
        r"
               4    4 - #597c95
             444    4 - #597c95
             592    4 - #597c95
             296    8 - #597c95
             148   12 - #597c95
             228   84 - #597c95
              12  148 - #597c95
             444  152 - #597c95
             592  152 - #597c95
             156  156 - #597c95
             300  156 - #597c95
              84  228 - #597c95
             228  228 - #597c95
             372  228 - #597c95
               8  296 - #597c95
             448  296 - #597c95
             156  300 - #597c95
             300  300 - #597c95
             592  300 - #597c95
             228  372 - #597c95
             372  372 - #597c95
             516  372 - #597c95
               4  444 - #597c95
             152  444 - #597c95
             444  444 - #597c95
             296  448 - #597c95
             588  448 - #597c95
             448  588 - #597c95
               4  592 - #597c95
             152  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
    )
}
