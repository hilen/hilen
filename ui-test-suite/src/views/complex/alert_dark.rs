use std::sync::mpsc::channel;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Alert, Color, DynamicColor, Label, ModalView, Question, Setup, Theme, ThemeMode, ViewData, ViewFrame,
        ViewTest, view,
    },
    ui_test::{check_colors, inject_touches},
};

/// An alert and a question follow the theme. The dialogs had light only
/// colors, so a dark app showed a white alert on its dark page. Here the
/// alert opens dark on a dark page, both turn light while it is open,
/// then a question opens dark.
#[view]
struct AlertDark {
    #[init]
    caption: Label,
}

impl Setup for AlertDark {
    fn setup(self: Weak<Self>) {
        // The page follows the theme like a real app, so the switch in the
        // middle turns page and alert light together.
        self.set_color(DynamicColor::new(Color::hex("#f2f2f7"), Color::hex("#0a0a0a")));
        self.caption
            .set_text("app page")
            .set_text_size(20)
            .set_text_color(DynamicColor::new(Color::hex("#6c6c70"), Color::hex("#9a9a9a")));
        self.caption.place().t(20).lr(20).h(30);
    }
}

fn set_theme(mode: ThemeMode) {
    from_main(move || Theme::set_mode(mode));
    wait_for_next_frame();
}

fn tap(x: f32, y: f32) {
    inject_touches(format!("{x:.0} {y:.0} b\n{x:.0} {y:.0} e"));
    wait_for_next_frame();
}

impl ViewTest for AlertDark {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_theme(ThemeMode::Dark);
        let alert = from_main(|| {
            Alert::prepare_modally_with_input(
                "Follows the theme, dark now, light after the switch".to_string(),
            )
        });
        wait_for_next_frame();
        check_colors(DARK_ALERT)?;

        // The open alert re-resolves its colors on a theme switch.
        set_theme(ThemeMode::Light);
        check_colors(LIGHT_ALERT)?;

        let frame = from_main(move || *alert.frame());
        tap(frame.center().x, frame.max_y() - 22.0);

        set_theme(ThemeMode::Dark);
        let (se, rc) = channel();
        from_main(move || {
            Question::ask("Remove it?").callback(move |answer| se.send(answer).unwrap());
        });
        wait_for_next_frame();
        check_colors(DARK_QUESTION)?;

        tap(367.0, 331.0);
        assert!(rc.try_recv()?);

        set_theme(ThemeMode::System);
        Ok(())
    }
}

const DARK_ALERT: &str = r"
 268   36 - #272727
 288   36 - #080808
 304   36 - #080808
 316   36 - #747474
 324   36 - #080808
 336   36 - #737373
   4   72 - #080808
 192  264 - #dbdbdb
 188  268 - #4f4f50
 248  268 - #545455
 384  268 - #737374
 248  272 - #545455
 276  272 - #2c2c2e
 300  272 - #f9f9f9
 356  272 - #ebebeb
 388  272 - #bebebe
 400  272 - #c6c6c6
 332  284 - #ffffff
 344  284 - #8a8a8b
 264  288 - #8e8e8f
 284  288 - #5b5b5d
 300  288 - #727274
 328  288 - #c8c8c9
 332  288 - #c4c4c5
 344  288 - #8a8a8b
 292  336 - #2c2c2e
 296  336 - #2c2c2e
 228  356 - #2c2c2e
 356  356 - #2c2c2e
 424  356 - #2c2c2e
   4  592 - #080808
 592  592 - #080808
";

const LIGHT_ALERT: &str = r"
 276   36 - #b5b5b9
 284   36 - #a4a4a8
 300   36 - #919194
 316   36 - #515154
 336   36 - #525255
   4   68 - #b5b5b9
 220  240 - #d6d6d9
 328  240 - #d6d6d9
 420  240 - #d6d6d9
 188  268 - #d5d5d5
 192  268 - #858586
 248  268 - #cfcfd0
 188  272 - #d5d5d5
 248  272 - #cfcfd0
 276  272 - #f9f9f9
 300  272 - #222224
 356  272 - #313133
 384  272 - #aeaeaf
 400  272 - #585859
 332  284 - #1c1c1e
 264  288 - #939394
 284  288 - #c8c8c8
 300  288 - #afafb0
 328  288 - #555557
 340  288 - #f9f9f9
 292  336 - #f9f9f9
 296  336 - #f9f9f9
 432  352 - #f8f8f9
 208  356 - #f9f9f9
 360  356 - #f9f9f9
   4  592 - #b5b5b9
 592  592 - #b5b5b9
";

const DARK_QUESTION: &str = r"
   4    4 - #080808
 592    4 - #080808
 268   36 - #272727
 276   36 - #080808
 284   36 - #1a1a1a
 288   36 - #080808
 304   36 - #080808
 316   36 - #747474
 324   36 - #080808
 336   36 - #737373
 328   40 - #747474
 424  248 - #2c2c2e
 168  256 - #2c2c2e
 264  272 - #ffffff
 260  276 - #fefefe
 336  276 - #2c2c2e
 260  280 - #fefefe
 280  280 - #b7b7b8
 284  280 - #767678
 296  280 - #2c2c2e
 312  280 - #fcfcfc
 316  280 - #2c2c2e
 432  308 - #38383a
 232  328 - #22456a
 368  328 - #0b82fb
 376  328 - #0b82fa
 224  332 - #1860aa
 232  332 - #22456a
 368  332 - #136ecb
 300  352 - #2f2f31
   4  592 - #080808
 592  592 - #080808
";
