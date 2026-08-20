use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::{Own, Weak},
    ui::{Container, NavigationView, RED, Setup, TouchStack, View, ViewController, ViewData, ViewTest, view},
    ui_test::helpers::check_colors,
};

#[view]
struct PresentTestView {}

impl ViewTest for PresentTestView {
    /// Presenting only works from inside a navigation stack, so the root is the
    /// stack and the view under test is its first view.
    fn make_root(view: Own<Self>) -> Own<dyn View> {
        NavigationView::with_view(view)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_before_present()?;
        check_present_animation(view)?;
        check_presented_stays()?;

        Ok(())
    }
}

fn check_before_present() -> Result<()> {
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

    assert_eq!(TouchStack::dump(), vec![vec!["Layer: Root view".to_string()]]);

    Ok(())
}

fn check_present_animation(view: Weak<PresentTestView>) -> Result<()> {
    let presented = from_main(move || {
        let presented = Container::new();
        presented.set_color(RED);

        view.present(presented)
    });

    presented.recv()?;

    check_colors(
        r"
               4    4 - #ff0000
             444    4 - #ff0000
             592    4 - #ff0000
             296    8 - #ff0000
             148   12 - #ff0000
             228   84 - #ff0000
              12  148 - #ff0000
             444  152 - #ff0000
             592  152 - #ff0000
             156  156 - #ff0000
             300  156 - #ff0000
              84  228 - #ff0000
             228  228 - #ff0000
             372  228 - #ff0000
               8  296 - #ff0000
             448  296 - #ff0000
             156  300 - #ff0000
             300  300 - #ff0000
             592  300 - #ff0000
             228  372 - #ff0000
             372  372 - #ff0000
             516  372 - #ff0000
               4  444 - #ff0000
             152  444 - #ff0000
             444  444 - #ff0000
             296  448 - #ff0000
             588  448 - #ff0000
             448  588 - #ff0000
               4  592 - #ff0000
             152  592 - #ff0000
             300  592 - #ff0000
             592  592 - #ff0000
        ",
    )?;

    assert_eq!(TouchStack::dump(), vec![vec!["Layer: Root view".to_string()]]);

    Ok(())
}

fn check_presented_stays() -> Result<()> {
    check_colors(
        r"
               4    4 - #ff0000
             444    4 - #ff0000
             592    4 - #ff0000
             296    8 - #ff0000
             148   12 - #ff0000
             228   84 - #ff0000
              12  148 - #ff0000
             444  152 - #ff0000
             592  152 - #ff0000
             156  156 - #ff0000
             300  156 - #ff0000
              84  228 - #ff0000
             228  228 - #ff0000
             372  228 - #ff0000
               8  296 - #ff0000
             448  296 - #ff0000
             156  300 - #ff0000
             300  300 - #ff0000
             592  300 - #ff0000
             228  372 - #ff0000
             372  372 - #ff0000
             516  372 - #ff0000
               4  444 - #ff0000
             152  444 - #ff0000
             444  444 - #ff0000
             296  448 - #ff0000
             588  448 - #ff0000
             448  588 - #ff0000
               4  592 - #ff0000
             152  592 - #ff0000
             300  592 - #ff0000
             592  592 - #ff0000
        ",
    )?;

    Ok(())
}
