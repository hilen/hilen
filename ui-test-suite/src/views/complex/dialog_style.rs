use std::sync::mpsc::{Receiver, channel};

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Alert, Color, Container, DialogStyle, Label, NamedKey, Question, Setup, UIColor, ViewData, ViewTest,
        view,
    },
    ui_test::{check_colors, inject_named_key},
};

const QUESTION: &str = r"
            184   40 - #0a0806
            288   40 - #884a22
            320   40 - #351e0f
            352   40 - #552f17
            376   40 - #0a0806
            416   40 - #472814
            224   44 - #aa5c29
            284   44 - #bf682e
            320   44 - #351e0f
            260  272 - #443c35
            360  272 - #3d342e
            268  276 - #d8cbbe
            332  276 - #7e746b
            380  276 - #f3e6d8
            224  280 - #1f1712
            260  280 - #443c35
            296  280 - #cec2b6
            316  280 - #f3e6d8
            332  280 - #7e746b
            348  280 - #c2b6aa
            352  280 - #f3e6d8
            240  284 - #d3c7ba
            168  308 - #4a3426
            432  308 - #4a3426
            300  312 - #2a1f17
            372  328 - #fa873c
            232  332 - #5f381e
            360  332 - #ff8a3d
            364  332 - #c76d32
            300  348 - #2a1f17
            4  592 - #0a0806
            592  592 - #0a0806
            ";

const DESTRUCTIVE: &str = r"
            176   40 - #bf682e
            228   40 - #713e1d
            268   40 - #0a0806
            320   40 - #351e0f
            352   40 - #552f17
            376   40 - #0a0806
            416   40 - #472814
            224   44 - #aa5c29
            288   44 - #884a22
            320   44 - #351e0f
            192   48 - #bf682e
            240  272 - #f3e6d8
            296  276 - #f2e5d7
            280  280 - #1f1712
            328  280 - #f1e4d6
            168  308 - #4a3426
            336  308 - #4a3426
            432  308 - #4a3426
            300  312 - #2a1f17
            232  328 - #fe8a3d
            256  328 - #ff8a3d
            300  328 - #2a1f17
            372  328 - #e1474c
            208  332 - #ff8a3d
            356  332 - #e4484d
            360  332 - #b33c3e
            372  332 - #b33c3e
            388  332 - #b33c3e
            300  348 - #2a1f17
            4  592 - #0a0806
            252  592 - #0a0806
            592  592 - #0a0806
            ";

const ALERT: &str = r"
            296   36 - #ac5d29
            188   40 - #bf682e
            228   40 - #713e1d
            268   40 - #0a0806
            288   40 - #884a22
            320   40 - #351e0f
            352   40 - #552f17
            376   40 - #0a0806
            416   40 - #472814
            180   44 - #6b3b1b
            284   44 - #bf682e
            288   44 - #884a22
            320   44 - #351e0f
            360   44 - #a35928
            192   48 - #bf682e
            372   48 - #bf682e
            592  164 - #0a0806
            4  168 - #0a0806
            340  276 - #8c8178
            260  280 - #f2e5d7
            268  280 - #3c332d
            296  280 - #1f1712
            316  280 - #1f1712
            168  308 - #4a3426
            220  308 - #4a3426
            332  308 - #4a3426
            380  308 - #4a3426
            432  308 - #4a3426
            296  332 - #1f1712
            4  592 - #0a0806
            328  592 - #0a0806
            592  592 - #0a0806
            ";

/// A dark ember palette like the one of an app with its own theme.
const EMBER: DialogStyle = DialogStyle {
    background:  UIColor::Plain(Color::hex("#1f1712")),
    text:        UIColor::Plain(Color::hex("#f3e6d8")),
    separator:   UIColor::Plain(Color::hex("#4a3426")),
    button:      UIColor::Plain(Color::hex("#ff8a3d")),
    destructive: UIColor::Plain(Color::hex("#e5484d")),
};

/// An app sets the look of every dialog once. The plain question shows
/// ember buttons, the destructive one a red yes, and the alert follows
/// the same palette.
#[view]
struct DialogStyleTest {
    #[init]
    backdrop: Container,
    caption:  Label,
}

impl Setup for DialogStyleTest {
    fn setup(self: Weak<Self>) {
        self.backdrop.set_color(Color::hex("#0d0a08"));
        self.backdrop.place().back();
        self.caption.set_text("app with a dark ember palette").set_text_size(20);
        self.caption.set_text_color(Color::hex("#ff8a3d"));
        self.caption.place().lrt(20).h(40);
    }
}

fn ask(question: Question) -> Receiver<bool> {
    let (se, rc) = channel();
    from_main(move || question.callback(move |answer| se.send(answer).unwrap()));
    wait_for_next_frame();
    rc
}

fn escape() {
    inject_named_key(NamedKey::Escape);
    wait_for_next_frame();
}

impl ViewTest for DialogStyleTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        from_main(|| EMBER.apply_globally());

        let answer = ask(Question::ask("Keep this mod enabled?").options("No", "Keep"));
        // Ember box, light text, both buttons orange.
        check_colors(QUESTION)?;
        escape();
        assert!(!answer.try_recv()?);

        let answer = ask(Question::ask("Delete this mod?").options("Cancel", "Delete").destructive());
        // The same box, the Delete button red.
        check_colors(DESTRUCTIVE)?;
        escape();
        assert!(!answer.try_recv()?);

        from_main(|| Alert::show("Mod deleted"));
        wait_for_next_frame();
        // The alert in the same palette, an orange OK.
        check_colors(ALERT)?;
        escape();

        Ok(())
    }
}
