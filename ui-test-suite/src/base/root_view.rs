use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{CLEAR, GREEN, ImageMode, ImageView, NoImage, Setup, UIManager, ViewData, ViewTest, view},
    ui_test::check_colors,
};

/// The image checks run against this view, not the root. The root is as big as
/// the window, so `AspectFill` would crop it differently on every screen, while
/// this canvas is the same everywhere.
#[view]
pub struct RootViewTest {
    #[init]
    image: ImageView,
}

impl Setup for RootViewTest {
    fn setup(mut self: Weak<Self>) {
        self.image.mode = ImageMode::AspectFill;
        self.image.place().back();
    }
}

impl ViewTest for RootViewTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_default_root()?;
        check_green_root()?;
        check_clear_root()?;
        check_image_root(view)?;
        check_no_image_root(view)?;

        Ok(())
    }
}

fn check_default_root() -> Result<()> {
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
    )?;

    Ok(())
}

fn check_green_root() -> Result<()> {
    from_main(|| {
        UIManager::root_view().set_color(GREEN);
    });

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
    )?;

    Ok(())
}

fn check_clear_root() -> Result<()> {
    from_main(|| {
        UIManager::root_view().set_color(CLEAR);
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
    )?;

    Ok(())
}

fn check_image_root(view: Weak<RootViewTest>) -> Result<()> {
    from_main(move || {
        view.image.set_image("cat.png");
    });

    check_colors(
        r"
               4    4 - #eec7cc
             360    4 - #e0b2b4
             592    4 - #d6a4a5
             156  108 - #edbdaf
             392  132 - #bda38a
             524  188 - #9b5c4b
             220  200 - #816f55
             248  208 - #362f21
             216  212 - #5b4b30
             236  216 - #0d0302
             484  216 - #844b33
             508  220 - #824d36
             240  228 - #3f3720
             244  228 - #413a1f
             380  256 - #0f0802
             344  268 - #120000
             360  272 - #41381b
             376  272 - #0b0103
             368  288 - #4e3a1e
             268  304 - #734433
             292  320 - #351205
              16  352 - #e2b0b1
             268  356 - #825340
             512  416 - #a88c76
             376  440 - #c6a68f
               4  480 - #e1a9aa
             208  508 - #d8b9a4
             588  508 - #937867
             468  544 - #886c57
              72  592 - #e4b3b2
             344  592 - #ceab97
             592  592 - #b4967e
        ",
    )?;

    Ok(())
}

fn check_no_image_root(view: Weak<RootViewTest>) -> Result<()> {
    from_main(move || {
        view.image.set_image(NoImage);
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
    )?;

    Ok(())
}
