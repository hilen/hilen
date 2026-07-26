use anyhow::Result;
use test_engine::{
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
             592    4 - #597c95
              88   24 - #0000e7
             148   24 - #0000e7
             288   24 - #ff0000
             336   24 - #ff0000
              32   32 - #ffffff
             244   32 - #ff0000
             304   64 - #ff0000
             160   84 - #ffffff
             260   88 - #ff0000
             336   88 - #ff0000
              96   92 - #ffffff
              24  100 - #0000e7
             308  116 - #ff0000
             212  120 - #0000e7
              84  144 - #ffffff
              32  164 - #ffffff
             128  172 - #ffffff
             216  204 - #0000e7
             164  212 - #0000e7
              76  216 - #0000e7
             116  268 - #ffffff
             160  268 - #ffffff
             592  268 - #597c95
              24  288 - #ffffff
             216  288 - #ffffff
             388  336 - #597c95
             532  428 - #597c95
             132  452 - #597c95
             300  532 - #597c95
               4  592 - #597c95
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
             580    4 - #597c95
              24   24 - #ffff00
             152   24 - #ffff00
             244   24 - #ff0000
             336   24 - #ff0000
              88   28 - #ffff00
             288   28 - #ff0000
             248   68 - #ff0000
             292   72 - #ff0000
             336   72 - #ff0000
              28   96 - #ffff00
             268   96 - #ff0000
             244  116 - #ff0000
             292  116 - #ff0000
             336  116 - #ff0000
             492  132 - #597c95
             212  152 - #ffff00
              24  164 - #ffff00
             216  200 - #ffff00
              76  212 - #ffff00
             140  212 - #ffff00
              24  216 - #ffff00
             592  248 - #597c95
             116  268 - #7c7c7c
             160  268 - #7c7c7c
             392  332 - #597c95
             548  420 - #597c95
             132  452 - #597c95
             300  532 - #597c95
               4  592 - #597c95
             440  592 - #597c95
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
             496    4 - #597c95
              88   24 - #0000e7
             288   24 - #ff0000
             336   24 - #ff0000
              32   32 - #ffffff
             144   32 - #ffffff
             244   32 - #ff0000
             304   64 - #ff0000
             260   88 - #ff0000
             336   88 - #ff0000
             104   96 - #ffffff
              28  100 - #0000e7
             308  116 - #ff0000
             212  120 - #0000e7
             592  132 - #597c95
              24  168 - #0000e7
             124  168 - #ffffff
             284  184 - #ffffff
             216  204 - #0000e7
             160  212 - #0000e7
              72  216 - #0000e7
             336  236 - #ffffff
             116  268 - #ffffff
             160  268 - #ffffff
              24  288 - #ffffff
             216  288 - #ffffff
             536  360 - #597c95
             376  392 - #597c95
             132  452 - #597c95
             300  532 - #597c95
               4  592 - #597c95
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
