use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Color, DynamicColor, Setup, TextField, Theme, ThemeMode, ViewData, ViewTest, view},
    ui_test::check_colors,
};

const FIELD_BG: DynamicColor = DynamicColor::new(Color::hex("#f0f2f5"), Color::hex("#31353c"));
const FIELD_TEXT: DynamicColor = DynamicColor::new(Color::hex("#17191d"), Color::hex("#f2f4f7"));

const LIGHT_PROBES: &str = r"
             540    4 - #597c95
              84   20 - #f0f2f5
             236   20 - #f0f2f5
             316   24 - #f0f2f5
             148   32 - #626468
             152   32 - #626468
             160   40 - #f0f2f5
             176   40 - #17191d
             192   40 - #17191d
              20   44 - #f0f2f5
             164   44 - #17191d
             176   44 - #17191d
             184   44 - #f0f2f5
             188   44 - #f0f2f5
             192   44 - #1e2024
             152   48 - #f0f2f5
             176   48 - #17191d
             188   48 - #f0f2f5
             192   48 - #17191d
             148   52 - #25272b
             156   52 - #25272b
             164   52 - #f0f2f5
             180   52 - #f0f2f5
              60   60 - #f0f2f5
             104   60 - #f0f2f5
             260   60 - #f0f2f5
             592  220 - #597c95
              32  320 - #597c95
             300  388 - #597c95
               4  592 - #597c95
             224  592 - #597c95
             592  592 - #597c95
";

const DARK_PROBES: &str = r"
             556    4 - #597c95
              80   20 - #31353c
             236   20 - #31353c
             316   24 - #31353c
             148   32 - #afb2b6
             152   32 - #afb2b6
             160   40 - #31353c
             176   40 - #f2f4f7
             192   40 - #f2f4f7
             164   44 - #f2f4f7
             184   44 - #31353c
             188   44 - #31353c
             192   44 - #eceef1
             152   48 - #31353c
             176   48 - #f2f4f7
             188   48 - #31353c
             192   48 - #f2f4f7
              20   52 - #31353c
             148   52 - #e6e8eb
             156   52 - #e6e8eb
             164   52 - #31353c
             180   52 - #31353c
             192   56 - #f1f3f6
              60   60 - #31353c
             104   60 - #31353c
             260   60 - #31353c
             592  240 - #597c95
               4  324 - #597c95
             292  372 - #597c95
              48  592 - #597c95
             360  592 - #597c95
             592  592 - #597c95
";

/// The `TextField` color setters accept theme pairs like every other
/// view, so a field follows a live theme switch. They used to take a
/// plain `Color`, which froze fields in the look of the launch theme.
#[view]
struct TextFieldTheme {
    #[init]
    field: TextField,
}

impl Setup for TextFieldTheme {
    fn setup(self: Weak<Self>) {
        self.field.set_color(FIELD_BG).set_corner_radius(10);
        self.field.set_text_color(FIELD_TEXT);
        self.field.set_text("Zug");
        self.field.place().tl(20).size(300, 44);
    }
}

impl ViewTest for TextFieldTheme {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // In human mode the OS theme may be dark. Start from a known state.
        from_main(|| {
            Theme::set_mode(ThemeMode::System);
            Theme::set_system(Theme::Light);
        });

        wait_for_next_frame();

        from_main(move || {
            assert_eq!(*view.field.color(), FIELD_BG.light);
        });

        check_colors(LIGHT_PROBES)?;

        from_main(|| Theme::set_system(Theme::Dark));

        wait_for_next_frame();

        from_main(move || {
            assert_eq!(*view.field.color(), FIELD_BG.dark);
        });

        check_colors(DARK_PROBES)?;

        // Leave the default state for the tests that follow.
        from_main(|| Theme::set_system(Theme::Light));

        Ok(())
    }
}
