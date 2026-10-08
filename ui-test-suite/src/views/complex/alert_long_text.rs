use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Alert, ModalView, UIManager, ViewFrame, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

/// A message longer than the alert can show. The alert stops growing and
/// the text scrolls inside it, so the last line can still be read.
#[view]
struct AlertLongText {}

const LINES: usize = 40;
/// The padding above and below the text, the text room and the OK row.
const FULL_HEIGHT: f32 = 20.0 + 400.0 + 20.0 + 44.0;

const TOP: &str = r"
     592    4 - #435d70
     284   84 - #5e5e5f
     288  104 - #b6b6b7
     284  124 - #5e5e5f
     432  132 - #f9f9f9
     296  140 - #505052
     284  160 - #5e5e5f
     412  188 - #a2a2a2
     288  192 - #b6b6b7
     304  212 - #f9f9f9
     288  228 - #b6b6b7
     316  244 - #f9f9f9
     412  244 - #a2a2a2
     280  248 - #a0a0a1
     284  280 - #e2e2e3
     412  296 - #a2a2a2
     292  300 - #939394
     320  312 - #cccccc
     284  336 - #e2e2e3
     316  364 - #f9f9f9
     280  368 - #a0a0a1
     284  404 - #e2e2e3
     320  420 - #1c1c1e
     284  440 - #e2e2e3
     308  460 - #1c1c1e
     316  460 - #1d1d1f
     280  476 - #a0a0a1
     296  520 - #f9f9f9
     168  532 - #f9f9f9
     432  532 - #f9f9f9
       4  592 - #435d70
     592  592 - #435d70
";

const BOTTOM: &str = r"
     592    4 - #435d70
     284   84 - #e2e2e3
     320  100 - #1c1c1e
     432  108 - #f9f9f9
     312  136 - #f9f9f9
     168  168 - #f9f9f9
     320  168 - #cccccc
       4  172 - #435d70
     308  192 - #1c1c1e
     308  228 - #b7b7b8
     284  260 - #e2e2e3
     320  276 - #1c1c1e
     412  276 - #a2a2a2
     292  296 - #939394
     168  300 - #f9f9f9
     320  344 - #cccccc
     280  348 - #a0a0a1
     316  348 - #434345
     412  384 - #a2a2a2
     316  396 - #f9f9f9
     280  400 - #a0a0a1
     284  436 - #e2e2e3
     412  452 - #a2a2a2
     348  464 - #343436
     252  468 - #888889
     312  468 - #646465
     340  468 - #3c3c3e
     348  468 - #636364
     252  472 - #888889
     312  472 - #646465
     296  520 - #f9f9f9
       4  592 - #435d70
";

const DISMISSED: &str = r"
       4    4 - #597c95
     444    4 - #597c95
     592    4 - #597c95
     296    8 - #597c95
     148   12 - #597c95
     228   84 - #597c95
      12  148 - #597c95
     444  152 - #597c95
     592  152 - #597c95
     156  156 - #597c95
     300  156 - #597c95
      84  228 - #597c95
     228  228 - #597c95
     372  228 - #597c95
       8  296 - #597c95
     448  296 - #597c95
     156  300 - #597c95
     300  300 - #597c95
     592  300 - #597c95
     228  372 - #597c95
     372  372 - #597c95
     516  372 - #597c95
       4  444 - #597c95
     152  444 - #597c95
     444  444 - #597c95
     296  448 - #597c95
     588  448 - #597c95
     448  588 - #597c95
       4  592 - #597c95
     152  592 - #597c95
     300  592 - #597c95
     592  592 - #597c95
";

impl ViewTest for AlertLongText {
    // A finger drag scrolls only with drag scrolling on, the touch
    // platform default.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let mut message: Vec<String> = (1..=LINES).map(|line| format!("line {line}")).collect();
        message.push("THE LAST LINE".to_string());
        let message = message.join("\n");

        let alert = from_main(|| Alert::prepare_modally_with_input(message));
        wait_for_next_frame();

        assert_eq!(from_main(move || alert.height()), FULL_HEIGHT);
        // The first lines, the rest is cut at the OK row.
        check_colors(TOP)?;

        // The finger stays down, a lifted finger would let the text coast.
        inject_touches(
            "
            300 460 b
            300 80  m
        ",
        );
        wait_for_next_frame();
        // The text is at its end, THE LAST LINE is the lowest one.
        check_colors(BOTTOM)?;
        inject_touches("300 80 e");

        // The OK row still takes a tap after a scroll.
        let frame = from_main(move || *alert.frame());
        let x = frame.center().x;
        let y = frame.max_y() - 22.0;
        inject_touches(format!("{x:.0} {y:.0} b\n{x:.0} {y:.0} e"));
        wait_for_next_frame();
        check_colors(DISMISSED)?;

        Ok(())
    }
}
