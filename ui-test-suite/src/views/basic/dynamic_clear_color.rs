use anyhow::Result;
use test_engine::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLUE, Container, DynamicColor, GREEN, RED, Setup, Theme, ThemeMode, UIManager, ViewData, ViewTest,
        view,
    },
    ui_test::check_colors,
};

const BACKGROUND: DynamicColor = DynamicColor::new(GREEN, RED);

const LIGHT: &str = r"
       4    4 - #00ff00
     252    4 - #00ff00
     592    4 - #00ff00
     424  100 - #00ff00
     132  132 - #00ff00
     564  156 - #00ff00
     300  248 - #00ff00
     260  252 - #0000e7
     332  252 - #0000e7
       4  256 - #00ff00
     284  276 - #0000e7
     316  276 - #0000e7
     348  276 - #0000e7
     252  288 - #0000e7
     280  300 - #0000e7
     304  304 - #0000e7
     336  304 - #0000e7
     592  304 - #00ff00
     272  320 - #0000e7
     320  328 - #0000e7
     348  332 - #0000e7
     276  344 - #0000e7
     252  348 - #0000e7
     300  348 - #0000e7
     332  348 - #0000e7
      84  424 - #00ff00
     484  448 - #00ff00
     264  480 - #00ff00
     168  568 - #00ff00
       4  592 - #00ff00
     332  592 - #00ff00
     592  592 - #00ff00
";

const DARK: &str = r"
       4    4 - #ff0000
     252    4 - #ff0000
     592    4 - #ff0000
     424  100 - #ff0000
     132  132 - #ff0000
     564  156 - #ff0000
     300  248 - #ff0000
     260  252 - #0000e7
     332  252 - #0000e7
       4  256 - #ff0000
     284  276 - #0000e7
     316  276 - #0000e7
     348  276 - #0000e7
     252  288 - #0000e7
     280  300 - #0000e7
     304  304 - #0000e7
     336  304 - #0000e7
     592  304 - #ff0000
     272  320 - #0000e7
     320  328 - #0000e7
     348  332 - #0000e7
     276  344 - #0000e7
     252  348 - #0000e7
     300  348 - #0000e7
     332  348 - #0000e7
      84  424 - #ff0000
     484  448 - #ff0000
     264  480 - #ff0000
     168  568 - #ff0000
       4  592 - #ff0000
     332  592 - #ff0000
     592  592 - #ff0000
";

const PLAIN: &str = r"
       4    4 - #0000e7
     444    4 - #0000e7
     592    4 - #0000e7
     296    8 - #0000e7
     148   12 - #0000e7
     228   84 - #0000e7
      12  148 - #0000e7
     444  152 - #0000e7
     592  152 - #0000e7
     156  156 - #0000e7
     300  156 - #0000e7
      84  228 - #0000e7
     228  228 - #0000e7
     372  228 - #0000e7
       8  296 - #0000e7
     448  296 - #0000e7
     156  300 - #0000e7
     300  300 - #0000e7
     592  300 - #0000e7
     228  372 - #0000e7
     372  372 - #0000e7
     516  372 - #0000e7
       4  444 - #0000e7
     152  444 - #0000e7
     444  444 - #0000e7
     296  448 - #0000e7
     588  448 - #0000e7
     448  588 - #0000e7
       4  592 - #0000e7
     152  592 - #0000e7
     300  592 - #0000e7
     592  592 - #0000e7
";

/// The screen background is the clear color, not a color on the root view. This
/// fixture paints nothing itself, so every probe outside the marker reads the
/// clear color straight from the surface.
#[view]
struct DynamicClearColor {
    #[init]
    marker: Container,
}

impl Setup for DynamicClearColor {
    fn setup(self: Weak<Self>) {
        UIManager::set_clear_color(BACKGROUND);
        self.marker.set_color(BLUE);
        self.marker.place().center().size(100, 100);
    }
}

impl ViewTest for DynamicClearColor {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_light()?;
        check_dark()?;
        check_plain_stops_following()?;

        Ok(())
    }
}

fn check_light() -> Result<()> {
    // In human mode the OS theme may be dark. Start from a known state.
    from_main(|| {
        Theme::set_mode(ThemeMode::System);
        Theme::set_system(Theme::Light);
    });
    wait_for_next_frame();

    check_colors(LIGHT)?;

    Ok(())
}

fn check_dark() -> Result<()> {
    from_main(|| Theme::set_system(Theme::Dark));
    wait_for_next_frame();

    check_colors(DARK)?;

    Ok(())
}

/// A plain color replaces the pair, so the next theme switch must not bring
/// the pair back.
fn check_plain_stops_following() -> Result<()> {
    from_main(|| UIManager::set_clear_color(BLUE));
    wait_for_next_frame();

    from_main(|| Theme::set_system(Theme::Light));
    wait_for_next_frame();

    check_colors(PLAIN)?;

    Ok(())
}
