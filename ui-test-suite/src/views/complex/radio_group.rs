use anyhow::Result;
use test_engine::{
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
             264   48 - #395060
             288   48 - #000000
             204   56 - #597c95
             208   56 - #007aff
             264   56 - #395060
             208   60 - #007aff
             212   60 - #007aff
             244   60 - #000000
             264   60 - #395060
             300   60 - #000001
             264   64 - #395060
             212   68 - #8e8e93
             248   88 - #2d3e4b
             252   88 - #2d3e4b
             256   88 - #2d3e4b
             324   88 - #000000
             200  100 - #8b8d93
             212  104 - #597c95
             280  104 - #000001
             312  104 - #597c95
             336  104 - #597c95
             260  132 - #000001
             284  132 - #000001
             200  140 - #8b8d93
             244  144 - #000001
             272  144 - #597c95
             212  148 - #8e8e93
             300  148 - #000001
             592  280 - #597c95
             300  468 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ";

/// The same scene with the dot moved down one row, to y 96 to 100.
const SECOND_SELECTED: &str = r"
             264   48 - #395060
             208   56 - #597c95
             264   56 - #395060
             300   56 - #597c95
             200   60 - #8b8d93
             244   60 - #000000
             264   60 - #395060
             264   64 - #395060
             212   68 - #8e8e93
             248   88 - #2d3e4b
             252   88 - #2d3e4b
             256   88 - #2d3e4b
             324   88 - #000000
             296   92 - #000001
             208   96 - #007aff
             208  100 - #007aff
             212  100 - #007aff
             244  104 - #000000
             280  104 - #000001
             312  104 - #597c95
             336  104 - #597c95
             260  132 - #000001
             284  132 - #000001
             200  140 - #8b8d93
             244  144 - #000001
             272  144 - #597c95
             212  148 - #8e8e93
             300  148 - #000001
             592  280 - #597c95
             300  468 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ";
