use anyhow::Result;
use test_engine::{
    gm::Apply,
    refs::Weak,
    ui::{Anchor::Top, Button, ORANGE, Setup, Style, ViewData, ViewSubviews, ViewTest, view},
    ui_test::check_colors,
};

const MENU_BUTTON: Style = Style::new(|view| {
    view.set_color((75, 129, 244));
    view.set_corner_radius(20);
    view.place().size(280, 100).l(50);

    if let Some(view) = view.downcast_view::<Button>() {
        view.set_text_color(ORANGE);
        view.set_text_size(64);
    }
});

#[view]
struct Styles {
    #[init]
    button_1: Button,
    button_2: Button,
    button_3: Button,
}

impl Setup for Styles {
    fn setup(self: Weak<Self>) {
        [self.button_1, self.button_2, self.button_3].apply(|button| {
            button.apply_style(MENU_BUTTON);
        });

        self.button_1.set_text("Button 1").place().t(50);

        self.button_2.set_text("Button 2");
        self.button_2.place().anchor(Top, self.button_1, 40);

        self.button_3.set_text("Button 3");
        self.button_3.place().anchor(Top, self.button_2, 40);
    }
}

impl ViewTest for Styles {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             592    4 - #597c95
              92   80 - #4b81f4
             248   92 - #ffcb00
              92  104 - #4b81f4
             188  104 - #ffcb00
             292  112 - #ffcb00
             128  116 - #fecb13
             184  192 - #4b81f4
             324  200 - #4b81f4
              92  220 - #4b81f4
             156  224 - #ffcb00
             256  236 - #4b81f4
             296  236 - #fecb13
              92  244 - #4b81f4
             128  256 - #fecb13
             200  256 - #fecb13
             324  280 - #4b81f4
             588  300 - #597c95
             156  356 - #ffcb00
              92  360 - #4b81f4
             300  360 - #fecb13
              88  368 - #4b81f4
             248  372 - #ffcb00
              92  384 - #4b81f4
             128  396 - #fecb13
             204  396 - #fecb13
              52  412 - #4b81f4
             288  428 - #4b81f4
             468  444 - #597c95
             148  592 - #597c95
             352  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
