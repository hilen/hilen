use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::{Color, WHITE},
    ui::{Container, DynamicColor, RingSpinner, Setup, Theme, ThemeMode, ViewData, ViewTest, view},
    ui_test::check_colors,
    window::continuous_render_active,
};

const RING: DynamicColor = DynamicColor::new(Color::hex("#374151"), Color::hex("#d1d5db"));
const CARD: DynamicColor = DynamicColor::new(WHITE, Color::hex("#1f2329"));

/// A large ring held still so its gap position and stroke are pinned, and a
/// small one at the size the browser ring has, next to a card so the theme
/// pair of the stroke is visible against both looks.
#[view]
struct RingSpinnerTest {
    #[init]
    card:  Container,
    big:   RingSpinner,
    small: RingSpinner,
}

impl Setup for RingSpinnerTest {
    fn setup(mut self: Weak<Self>) {
        self.card.set_color(CARD).place().tl(20).size(300, 300);

        self.big.set_ring_color(RING).set_line_width(24).set_speed(0);
        self.big.place().t(50).l(50).size(240, 240);

        self.small.set_ring_color(RING).set_speed(0);
        self.small.place().t(300).l(164).size(12, 12);
    }
}

/// The gap on top, a quarter of the ring missing.
const GAP_TOP_PROBES: &str = r"
     556    4 - #597c95
      24   20 - #ffffff
     164   20 - #ffffff
     240   20 - #ffffff
     316   20 - #ffffff
      84   88 - #374151
     260   96 - #374151
     216  112 - #ffffff
     144  124 - #ffffff
     284  140 - #374151
     464  140 - #597c95
      56  144 - #374151
     288  180 - #374151
     164  192 - #ffffff
     260  204 - #374151
     316  204 - #ffffff
      60  208 - #374151
     592  240 - #597c95
      96  260 - #374151
     240  260 - #374151
     448  276 - #597c95
     200  284 - #374151
     152  288 - #9ba0a8
      20  316 - #ffffff
     316  316 - #ffffff
     468  416 - #597c95
       4  448 - #597c95
     136  468 - #597c95
     300  552 - #597c95
     444  588 - #597c95
       4  592 - #597c95
     592  592 - #597c95
";

/// The same ring turned half way, the gap at the bottom.
const GAP_BOTTOM_PROBES: &str = r"
     592    4 - #597c95
      24   20 - #ffffff
     316   20 - #ffffff
     176   52 - #374151
     124   60 - #374151
     208   80 - #374151
     252   84 - #374151
      76   96 - #374151
     120  116 - #ffffff
     192  120 - #ffffff
     468  140 - #597c95
      56  144 - #374151
     284  148 - #374151
      72  192 - #374151
     180  192 - #ffffff
      20  204 - #ffffff
     276  220 - #374151
      96  240 - #374151
     244  288 - #ffffff
     560  300 - #597c95
     168  308 - #ffffff
     172  308 - #ffffff
      20  316 - #ffffff
      96  316 - #ffffff
     316  316 - #ffffff
     592  440 - #597c95
     452  456 - #597c95
     144  484 - #597c95
     300  572 - #597c95
       4  592 - #597c95
     436  592 - #597c95
     592  592 - #597c95
";

/// The dark pair of both the card and the stroke after a theme switch.
const DARK_PROBES: &str = r"
     592    4 - #597c95
      24   20 - #1f2329
     316   20 - #1f2329
     176   52 - #d1d5db
     124   60 - #d1d5db
     208   80 - #d1d5db
     252   84 - #d1d5db
      76   96 - #d1d5db
     120  116 - #1f2329
     192  120 - #1f2329
     468  140 - #597c95
      56  144 - #d1d5db
     284  148 - #d1d5db
      72  192 - #d1d5db
     180  192 - #1f2329
      20  204 - #1f2329
     276  220 - #d1d5db
      96  240 - #d1d5db
     244  288 - #1f2329
     560  300 - #597c95
     168  308 - #1f2329
     172  308 - #1f2329
      20  316 - #1f2329
      96  316 - #1f2329
     316  316 - #1f2329
     592  440 - #597c95
     452  456 - #597c95
     144  484 - #597c95
     300  572 - #597c95
       4  592 - #597c95
     436  592 - #597c95
     592  592 - #597c95
";

impl ViewTest for RingSpinnerTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(|| {
            Theme::set_mode(ThemeMode::System);
            Theme::set_system(Theme::Light);
        });
        wait_for_next_frame();
        check_colors(GAP_TOP_PROBES)?;

        from_main(move || {
            let mut this = view;
            this.big.set_angle(0.5);
            this.small.set_angle(0.5);
        });
        wait_for_next_frame();
        check_colors(GAP_BOTTOM_PROBES)?;

        from_main(|| Theme::set_system(Theme::Dark));
        wait_for_next_frame();
        check_colors(DARK_PROBES)?;

        // A held ring must not keep the loop awake through its stroke alone.
        // The keep alive animation runs whenever the ring is visible, so
        // the loop draws while a ring shows, and sleeps once it hides.
        let continuous = from_main(continuous_render_active);
        ensure!(continuous, "a visible ring must keep the loop drawing");

        from_main(move || {
            let mut this = view;
            this.big.set_speed(1);
            this.big.set_angle(0.0);
        });
        wait_for_next_frame();
        wait_for_next_frame();
        let angle = from_main(move || view.big.angle());
        ensure!(
            angle > 0.0,
            "a spinning ring must turn between frames, angle stayed {angle}"
        );

        from_main(move || {
            view.big.set_hidden(true);
            view.small.set_hidden(true);
        });
        wait_for_next_frame();
        wait_for_next_frame();
        let continuous = from_main(continuous_render_active);
        ensure!(!continuous, "hidden rings must let the loop sleep");

        from_main(move || {
            view.big.set_hidden(false);
        });
        wait_for_next_frame();
        let continuous = from_main(continuous_render_active);
        ensure!(continuous, "a ring shown again must wake the loop");

        Ok(())
    }
}
