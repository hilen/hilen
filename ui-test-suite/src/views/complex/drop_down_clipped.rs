use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, TextAlignment, TextDropDown, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

/// A card with round corners that clips its subviews, like a table card of
/// an admin page.
#[view]
struct ClippingCard {}

impl Setup for ClippingCard {
    fn setup(self: Weak<Self>) {}

    fn clips_to_bounds(&self) -> bool {
        true
    }
}

/// A drop down near the bottom of a card that clips its subviews. The open
/// list reaches past the card, and the card cut it off, its rows could be
/// neither seen nor tapped there. The list now sits at the end of the layer
/// of its box, so it shows whole over the card and the page, and a row out
/// of the card picks.
#[view]
struct DropDownClipped {
    drop: Weak<TextDropDown>,

    #[init]
    title: Label,
    card:  ClippingCard,
    state: Label,
}

impl Setup for DropDownClipped {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("clipping card with a drop down").set_text_size(18);
        self.title.set_alignment(TextAlignment::Left);
        self.title.set_frame((20, 16, 400, 26));

        self.card.set_color((40, 52, 72));
        self.card.set_corner_radius(12);
        self.card.set_frame((20, 60, 300, 110));

        let note = self.card.add_view::<Label>();
        note.set_text("card").set_text_size(16).set_alignment(TextAlignment::Left);
        note.set_text_color((220, 225, 235));
        note.set_color((40, 52, 72));
        note.set_frame((20, 12, 200, 24));

        self.drop = self.card.add_view::<TextDropDown>();
        self.drop.set_values(["Dog", "Cat", "Sheep"]);
        self.drop.set_frame((20, 50, 260, 40));

        self.state.set_text_size(18).set_multiline(true);
        self.state.set_alignment(TextAlignment::Left);
        self.state.set_frame((340, 60, 240, 110));
        self.show_state();
    }
}

impl DropDownClipped {
    fn show_state(self: Weak<Self>) {
        let open = if self.drop.is_opened() { "open" } else { "closed" };
        self.state.set_text(format!("list {open}, picked {}", self.drop.value()));
    }

    /// Taps a point, then gives whether the list is open and the pick.
    fn tap(self: Weak<Self>, x: u32, y: u32) -> (bool, String) {
        inject_touches(format!("{x} {y} b\n{x} {y} e"));
        from_main(move || {
            self.show_state();
            (self.drop.is_opened(), self.drop.value().to_string())
        })
    }
}

impl ViewTest for DropDownClipped {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        ensure!(view.tap(170, 130).0, "a tap on the box did not open the list");
        // the open list shows whole, its last 2 rows below the card
        check_colors(CHECK_1)?;

        // The box ends at 150, the list starts 6 under it with a padding of
        // 4, and a row is 36 tall, so the third row is from 232 to 268, all
        // of it below the card, which ends at 170.
        let (open, value) = view.tap(170, 250);
        ensure!(!open, "a tap on the third row left the list open");
        ensure!(
            value == "Sheep",
            "the tap on the third row, out of the card, picked {value:?}"
        );
        // Sheep is picked and the list is closed, nothing of it is left below
        // the card
        check_colors(CHECK_2)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
      136   24 - #000000
       52   28 - #11181d
      216   28 - #597c95
       52   32 - #11181d
      200   60 - #283448
      304   68 - #283448
       60   84 - #283448
      144  104 - #283448
      452  116 - #597c95
      516  116 - #597c95
      396  120 - #000000
       76  128 - #ffffff
       72  136 - #000000
      200  148 - #fefefe
       84  172 - #00daff
      116  172 - #01daff
       76  176 - #ebfcff
      284  176 - #00daff
      104  180 - #00daff
       72  184 - #00daff
      120  192 - #00daff
      220  216 - #ffffff
       40  236 - #ffffff
      300  236 - #445f72
      124  252 - #000000
       80  256 - #ffffff
      152  256 - #000000
      204  272 - #3c5465
      256  276 - #476377
      300  568 - #597c95
        4  592 - #597c95
      592  592 - #597c95
";

const CHECK_2: &str = r"
      168   24 - #040506
       40   28 - #597c95
       52   28 - #11181d
      216   28 - #597c95
      272   28 - #000000
       52   32 - #11181d
      120   32 - #000000
      176   60 - #283448
      256   60 - #283448
       60   84 - #283448
      316   88 - #283448
      264  112 - #ffffff
      396  112 - #11181d
      508  112 - #597c95
      376  116 - #000000
      396  116 - #11181d
      432  116 - #597c95
      468  116 - #597c95
      544  120 - #000000
       80  124 - #ffffff
      220  124 - #ffffff
      124  132 - #000000
      148  136 - #ffffff
       84  140 - #ffffff
      136  140 - #ffffff
       40  148 - #ffffff
      288  148 - #ffffff
      192  168 - #283448
      516  360 - #597c95
      300  468 - #597c95
        4  592 - #597c95
      592  592 - #597c95
";
