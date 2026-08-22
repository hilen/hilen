use anyhow::Result;
use hilen::{
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
             316   52 - #4b81f4
             296   80 - #e6c122
              88   84 - #4b81f4
             172   96 - #a2a57e
             252   96 - #fecb01
             296  100 - #e6c122
              92  112 - #4b81f4
             216  112 - #ffcb00
             140  116 - #cfb741
             296  116 - #e6c122
             316  192 - #4b81f4
              80  216 - #bbaf5c
             284  220 - #ffcb00
             172  224 - #a2a57e
             212  232 - #fecb01
             100  252 - #ffcb00
             172  252 - #a2a57e
             140  256 - #cfb741
             284  256 - #fecb01
             576  300 - #597c95
              80  356 - #bbaf5c
             172  364 - #a2a57e
             280  368 - #7d96b0
             148  372 - #a5a67a
             228  376 - #ffcb00
             288  376 - #ccb645
              88  392 - #4b81f4
             172  392 - #a2a57e
             140  396 - #cfb741
             216  428 - #4b81f4
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
