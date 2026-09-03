use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::Apply,
    refs::Weak,
    ui::{DropDown, Setup, ViewData, ViewTest, view},
    ui_test::{
        inject_touches, inject_touches_delayed,
        state::{append_state, get_state},
    },
};

#[view]
struct DropDownTestView {
    #[init]
    top: DropDown<&'static str>,
    bot: DropDown<&'static str>,
}

impl Setup for DropDownTestView {
    fn setup(mut self: Weak<Self>) {
        [self.top, self.bot].apply(|v| {
            v.on_changed(|val| {
                append_state(format!("{val}\n"));
            });
            v.place().center_x().size(200, 40);
        });

        self.top.place().t(5);
        self.bot.place().b(5);

        self.top.set_values(vec!["Dog", "Cat", "Sheep"]);
        self.bot.set_values(vec!["Car", "Boat", "Plane"]);
    }
}

impl ViewTest for DropDownTestView {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        assert_eq!(view.top.value(), &"Dog");
        assert_eq!(view.bot.value(), &"Car");

        inject_touches_delayed(
            r"
            334  35   b
            334  35   e
            322  109  b
            321  109  e
            352  585  b
            352  585  e
            326  491  b
            326  491  e
        ",
        );

        assert_eq!(view.top.value(), &"Cat");
        assert_eq!(view.bot.value(), &"Boat");

        let picked = get_state::<String>();

        assert!(from_main(move || {
            let mut bot = view.bot;
            bot.set_value(&"Plane")
        }));
        assert_eq!(view.bot.value(), &"Plane");
        assert_eq!(view.bot.text(), "Plane");

        assert!(!from_main(move || {
            let mut bot = view.bot;
            bot.set_value(&"Train")
        }));
        assert_eq!(view.bot.value(), &"Plane");
        assert_eq!(view.bot.text(), "Plane");

        // A rebuilt list falls back to the first entry, and set_value is
        // what puts the old pick back.
        from_main(move || {
            let mut bot = view.bot;
            bot.set_values(vec!["Car", "Plane", "Rocket"]);
        });
        assert_eq!(view.bot.value(), &"Car");
        assert_eq!(view.bot.text(), "Car");

        assert!(from_main(move || {
            let mut bot = view.bot;
            bot.set_value(&"Plane")
        }));
        assert_eq!(view.bot.value(), &"Plane");
        assert_eq!(view.bot.text(), "Plane");

        assert_eq!(get_state::<String>(), picked);

        // The drop down is still collapsed, so one tap opens it and the
        // next one lands on a cell. A programmatic select that opened it
        // would swallow both taps.
        inject_touches_delayed(
            r"
            352  585  b
            352  585  e
            352  527  b
            352  527  e
        ",
        );

        assert_eq!(view.bot.value(), &"Rocket");
        assert_eq!(view.bot.text(), "Rocket");
        assert_eq!(get_state::<String>(), format!("{picked}Rocket\n"));

        inject_touches(
            "
            228  32   b
            228  32   e

        ",
        );

        Ok(())
    }
}
