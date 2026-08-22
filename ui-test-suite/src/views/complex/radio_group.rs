use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{RadioGroup, Setup, Theme, ThemeMode, ViewData, ViewTest, view},
    ui_test::{
        check_colors, inject_touches,
        state::{append_state, get_state},
    },
};

#[view]
struct RadioGroupTestView {
    #[init]
    group: RadioGroup<&'static str>,
}

impl Setup for RadioGroupTestView {
    fn setup(mut self: Weak<Self>) {
        self.group.place().center_x().t(40).size(200, 120);
        self.group.on_changed(|val| {
            append_state(format!("{val}\n"));
        });
        self.group.set_values(vec!["Male", "Female", "Other"]);
    }
}

impl ViewTest for RadioGroupTestView {
    /// The ring and dot are theme pairs, so the run has to pin a theme
    /// instead of inheriting whatever the OS is set to. See the same hook
    /// on `ButtonDisabled`.
    fn before_start() {
        from_main(|| {
            Theme::set_mode(ThemeMode::Light);
        });
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        assert_eq!(view.group.value(), &"Male");
        assert_eq!(view.group.text(), "Male");

        selected_first()?;

        // Second option. Rows split the group height evenly, so each one
        // is 40 tall starting at y 40.
        inject_touches(
            r"
            300  100  b
            300  100  e
        ",
        );

        assert_eq!(view.group.value(), &"Female");
        assert_eq!(view.group.text(), "Female");
        assert_eq!(get_state::<String>(), "Female\n");

        selected_second()?;

        // Tapping the option that is already selected changes nothing and
        // must not fire `changed` a second time.
        inject_touches(
            r"
            300  100  b
            300  100  e
        ",
        );

        assert_eq!(view.group.value(), &"Female");
        assert_eq!(get_state::<String>(), "Female\n");

        // A programmatic pick moves the dot without firing `changed`.
        assert!(from_main(move || {
            let mut group = view.group;
            group.set_value(&"Other")
        }));
        assert_eq!(view.group.value(), &"Other");
        assert_eq!(view.group.text(), "Other");
        assert_eq!(get_state::<String>(), "Female\n");

        assert!(!from_main(move || {
            let mut group = view.group;
            group.set_value(&"Nonexistent")
        }));
        assert_eq!(view.group.value(), &"Other");

        // A rebuilt list falls back to the first entry, like DropDown.
        from_main(move || {
            let mut group = view.group;
            group.set_values(vec!["Male", "Female", "Other"]);
        });
        assert_eq!(view.group.value(), &"Male");
        assert_eq!(get_state::<String>(), "Female\n");

        selected_first()?;

        // The theme is global and nothing resets it between tests, so the
        // pin from `before_start` has to be handed back.
        from_main(|| {
            Theme::set_mode(ThemeMode::System);
        });

        Ok(())
    }
}

fn selected_first() -> Result<()> {
    check_colors(FIRST_SELECTED)
}

fn selected_second() -> Result<()> {
    check_colors(SECOND_SELECTED)
}

/// The dot sits in the first ring, at y 56 to 60.
const FIRST_SELECTED: &str = r"
             208   56 - #007aff
             248   56 - #000000
             208   60 - #007aff
             212   60 - #007aff
             300   60 - #54758c
             276   64 - #597c95
             248   88 - #11171c
             252   88 - #11171c
             324   88 - #010101
             276   96 - #2d3e4b
             200  100 - #8b8d93
             244  100 - #000000
             276  100 - #2d3e4b
             312  100 - #3d5667
             260  104 - #000001
             276  104 - #2d3e4b
             212  108 - #8e8e93
             276  108 - #2d3e4b
             300  108 - #466175
             332  108 - #000001
             276  132 - #000000
             200  140 - #8b8d93
             256  140 - #597c95
             244  144 - #000001
             300  144 - #597c95
             312  144 - #4a677c
             212  148 - #8e8e93
             312  148 - #4a677c
             592  280 - #597c95
             300  468 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ";

/// The same scene with the dot moved down one row, to y 96 to 100.
const SECOND_SELECTED: &str = r"
             248   56 - #000000
             200   60 - #8b8d93
             300   60 - #54758c
             276   64 - #597c95
             212   68 - #8e8e93
             248   88 - #11171c
             252   88 - #11171c
             324   88 - #010101
             208   96 - #007aff
             276   96 - #2d3e4b
             208  100 - #007aff
             212  100 - #007aff
             244  100 - #000000
             276  100 - #2d3e4b
             312  100 - #3d5667
             260  104 - #000001
             276  104 - #2d3e4b
             276  108 - #2d3e4b
             300  108 - #466175
             332  108 - #000001
             276  132 - #000000
             200  140 - #8b8d93
             256  140 - #597c95
             244  144 - #000001
             300  144 - #597c95
             312  144 - #4a677c
             212  148 - #8e8e93
             312  148 - #4a677c
             592  280 - #597c95
             300  468 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ";
