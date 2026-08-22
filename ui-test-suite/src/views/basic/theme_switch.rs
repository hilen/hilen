use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, BLUE, Color, Container, DynamicColor, Label, RED, Setup, Theme, ThemeMode, UIEvents, ViewData,
        ViewSubviews, ViewTest, WHITE, YELLOW, view,
    },
    ui_test::check_colors,
};

const BACKGROUND: DynamicColor = DynamicColor::new(WHITE, Color::hex("#7c7c7c"));
const BORDER: DynamicColor = DynamicColor::new(BLUE, YELLOW);
const TEXT: DynamicColor = DynamicColor::new(BLACK, WHITE);

#[view]
struct ThemeSwitch {
    switches: Vec<Theme>,
    added:    Weak<Container>,

    #[init]
    themed: Container,
    plain:  Container,
    label:  Label,
}

impl Setup for ThemeSwitch {
    fn setup(self: Weak<Self>) {
        self.themed.set_color(BACKGROUND).set_border_color(BORDER).set_border_width(10);
        self.themed.place().tl(20).size(200, 200);

        self.plain.set_color(RED);
        self.plain.place().t(20).l(240).size(100, 100);

        self.label.set_color(BACKGROUND);
        self.label.set_text("Theme").set_text_size(32).set_text_color(TEXT);
        self.label.place().t(240).l(20).size(200, 50);

        UIEvents::theme_changed().val(self, move |theme| {
            let mut this = self;
            this.switches.push(theme);
        });
    }
}

fn check_initial_light_theme(view: Weak<ThemeSwitch>) -> Result<()> {
    // In human mode the OS theme may be dark. Start from a known state.
    from_main(move || {
        Theme::set_mode(ThemeMode::System);
        Theme::set_system(Theme::Light);
        let mut this = view;
        this.switches.clear();
    });

    wait_for_next_frame();

    check_colors(
        r"
             144   20 - #0000e7
             336   20 - #ff0000
             240   24 - #ff0000
              32   32 - #ffffff
             264   80 - #ff0000
             212  112 - #0000e7
             312  116 - #ff0000
              92  136 - #ffffff
              20  188 - #0000e7
             204  192 - #ffffff
             132  196 - #ffffff
             592  248 - #597c95
              80  256 - #000000
             100  260 - #000000
             160  260 - #000000
             116  264 - #ffffff
             148  264 - #010101
             160  264 - #ffffff
             116  268 - #b4b4b4
             120  268 - #b4b4b4
             136  268 - #e8e8e8
             160  268 - #b4b4b4
             164  268 - #b4b4b4
              80  272 - #000000
              92  272 - #000000
             104  272 - #000000
             116  272 - #ffffff
             136  272 - #e8e8e8
             160  272 - #ffffff
               4  524 - #597c95
             256  592 - #597c95
             592  592 - #597c95
            ",
    )?;

    from_main(move || {
        assert_eq!(Theme::current(), Theme::Light);
        assert_eq!(*view.themed.color(), WHITE);
        assert_eq!(*view.themed.border_color(), BLUE);
        assert_eq!(*view.label.text_color(), BLACK);
        assert_eq!(*view.plain.color(), RED);
    });

    Ok(())
}

fn check_dark_theme(view: Weak<ThemeSwitch>) -> Result<()> {
    from_main(|| Theme::set_system(Theme::Dark));
    wait_for_next_frame();

    check_colors(
        r"
              24   20 - #ffff00
             152   20 - #ffff00
             336   20 - #ff0000
             244   24 - #ff0000
             240  116 - #ff0000
             336  116 - #ff0000
              20  144 - #ffff00
             216  196 - #ffff00
             592  248 - #597c95
              80  256 - #ffffff
              92  256 - #ffffff
             100  260 - #ffffff
             132  260 - #ffffff
             160  260 - #ffffff
             108  264 - #7f7f7f
             116  264 - #7c7c7c
             148  264 - #fefefe
             160  264 - #7c7c7c
             116  268 - #a3a3a3
             120  268 - #a3a3a3
             136  268 - #888888
             160  268 - #a3a3a3
             164  268 - #a3a3a3
              80  272 - #ffffff
              92  272 - #ffffff
             104  272 - #ffffff
             116  272 - #7c7c7c
             136  272 - #888888
             160  272 - #7c7c7c
               4  524 - #597c95
             256  592 - #597c95
             592  592 - #597c95
            ",
    )?;

    from_main(move || {
        assert_eq!(Theme::current(), Theme::Dark);
        assert_eq!(*view.themed.color(), BACKGROUND.dark);
        assert_eq!(*view.themed.border_color(), YELLOW);
        assert_eq!(*view.label.text_color(), WHITE);
        assert_eq!(*view.plain.color(), RED);
        assert_eq!(view.switches, vec![Theme::Dark]);
    });

    Ok(())
}

// A view created while dark resolves against the dark theme.
fn check_view_added_in_dark(view: Weak<ThemeSwitch>) {
    from_main(move || {
        let mut this = view;
        let added = this.add_view::<Container>();
        added.set_color(BACKGROUND);
        added.place().t(140).l(240).size(100, 100);
        this.added = added;
    });

    wait_for_next_frame();

    from_main(move || {
        assert_eq!(*view.added.color(), BACKGROUND.dark);
    });
}

// Forced light wins over the dark system theme.
fn check_forced_light(view: Weak<ThemeSwitch>) -> Result<()> {
    from_main(|| Theme::set_mode(ThemeMode::Light));
    wait_for_next_frame();

    check_colors(
        r"
             336   20 - #ff0000
             240   24 - #ff0000
              32   32 - #ffffff
             140   32 - #ffffff
             264   80 - #ff0000
             212  112 - #0000e7
             312  116 - #ff0000
              20  152 - #0000e7
             120  160 - #ffffff
             216  196 - #0000e7
             336  236 - #ffffff
             592  248 - #597c95
              80  256 - #000000
             100  260 - #000000
             160  260 - #000000
             116  264 - #ffffff
             148  264 - #010101
             160  264 - #ffffff
             116  268 - #b4b4b4
             120  268 - #b4b4b4
             136  268 - #e8e8e8
             160  268 - #b4b4b4
             164  268 - #b4b4b4
              80  272 - #000000
              92  272 - #000000
             104  272 - #000000
             116  272 - #ffffff
             136  272 - #e8e8e8
             160  272 - #ffffff
               4  524 - #597c95
             256  592 - #597c95
             592  592 - #597c95
            ",
    )?;

    from_main(move || {
        assert_eq!(Theme::current(), Theme::Light);
        assert_eq!(*view.themed.color(), WHITE);
        assert_eq!(*view.themed.border_color(), BLUE);
        assert_eq!(*view.label.text_color(), BLACK);
        assert_eq!(*view.added.color(), WHITE);
        assert_eq!(view.switches, vec![Theme::Dark, Theme::Light]);
    });

    Ok(())
}

// Back to following the system, which is still dark.
fn check_back_to_system(view: Weak<ThemeSwitch>) {
    from_main(|| Theme::set_mode(ThemeMode::System));
    wait_for_next_frame();

    from_main(move || {
        assert_eq!(Theme::current(), Theme::Dark);
        assert_eq!(*view.themed.color(), BACKGROUND.dark);
        assert_eq!(view.switches, vec![Theme::Dark, Theme::Light, Theme::Dark]);
    });

    // Leave the default state for the tests that follow.
    from_main(|| {
        Theme::set_system(Theme::Light);
        assert_eq!(Theme::current(), Theme::Light);
    });
}

impl ViewTest for ThemeSwitch {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_initial_light_theme(view)?;
        check_dark_theme(view)?;
        check_view_added_in_dark(view);
        check_forced_light(view)?;
        check_back_to_system(view);

        Ok(())
    }
}
