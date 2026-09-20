use std::sync::mpsc::{Receiver, channel};

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Container, Question, RED, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

// The red strip runs under the left half of the question, so a
// transparent question body would show it through.
#[view]
struct QuestionTestView {
    #[init]
    red: Container,
}

impl Setup for QuestionTestView {
    fn setup(self: Weak<Self>) {
        self.red.set_color(RED);
        self.red.place().tl(0).size(300, 600);
    }
}

fn ask(text: &'static str) -> Receiver<bool> {
    let (se, rc) = channel();
    from_main(move || {
        Question::ask(text).callback(move |answer| se.send(answer).unwrap());
    });
    wait_for_next_frame();
    rc
}

fn tap(x: u32, y: u32) {
    inject_touches(format!("{x} {y} b\n{x} {y} e"));
    wait_for_next_frame();
}

fn check_question_shown() -> Result<()> {
    check_colors(
        r"
           4    4 - #bf0000
         296    4 - #bf0000
         592    4 - #435d70
         176  248 - #f9f9f9
         424  248 - #f9f9f9
         264  272 - #1c1c1e
         260  276 - #1d1d1f
         336  276 - #f9f9f9
         260  280 - #1d1d1f
         276  280 - #f9f9f9
         280  280 - #676769
         284  280 - #ababac
         296  280 - #f9f9f9
         312  280 - #1f1f21
         316  280 - #f9f9f9
         384  280 - #f9f9f9
         168  308 - #c6c6c8
         200  308 - #c6c6c8
         328  308 - #c6c6c8
         432  308 - #c6c6c8
         232  328 - #b2d5fb
         368  328 - #057cff
         376  328 - #067dff
         224  332 - #66aefd
         232  332 - #b2d5fb
         368  332 - #3e9afd
         300  348 - #ececed
           4  400 - #bf0000
         592  404 - #435d70
           4  592 - #bf0000
         300  592 - #435d70
         592  592 - #435d70
        ",
    )
}

// The card grows with its text, so a long question wraps inside it.
fn check_long_question_shown() -> Result<()> {
    check_colors(
        r"
           4    4 - #bf0000
         256    4 - #bf0000
         592    4 - #435d70
         432  236 - #f9f9f9
         220  252 - #1c1c1e
         288  252 - #1f1f21
         332  256 - #909091
         340  256 - #69696a
         364  256 - #616163
         248  260 - #959596
         292  260 - #89898a
         316  260 - #868687
         208  272 - #1c1c1e
         352  272 - #e0e0e0
         256  276 - #737374
         288  276 - #b1b1b2
         352  276 - #e0e0e0
         272  280 - #f9f9f9
         316  280 - #f9f9f9
         352  280 - #e0e0e0
         380  280 - #bababa
         300  300 - #f9f9f9
         224  348 - #66aefd
         232  348 - #b2d5fb
         224  352 - #66aefd
         232  352 - #b2d5fb
         368  352 - #f9f9f9
         300  368 - #ececed
         424  368 - #f9f9f9
           4  592 - #bf0000
         300  592 - #435d70
         592  592 - #435d70
        ",
    )
}

fn check_question_gone() -> Result<()> {
    check_colors(
        r"
           4    4 - #ff0000
         152    4 - #ff0000
         440    4 - #597c95
         592    4 - #597c95
         296    8 - #ff0000
         516   72 - #597c95
         444  148 - #597c95
         148  152 - #ff0000
         592  152 - #597c95
           4  156 - #ff0000
         292  156 - #ff0000
         368  224 - #597c95
         512  224 - #597c95
         152  296 - #ff0000
         440  296 - #597c95
         588  296 - #597c95
           4  300 - #ff0000
         296  300 - #ff0000
         512  368 - #597c95
          84  372 - #ff0000
         220  372 - #ff0000
         368  372 - #597c95
           8  444 - #ff0000
         296  444 - #ff0000
         444  444 - #597c95
         592  444 - #597c95
         152  448 - #ff0000
         448  588 - #597c95
           4  592 - #ff0000
         156  592 - #ff0000
         300  592 - #597c95
         592  592 - #597c95
        ",
    )
}

impl ViewTest for QuestionTestView {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let answer = ask("Remove it?");
        check_question_shown()?;
        tap(367, 331);
        assert!(answer.try_recv()?);

        let answer = ask("Remove GemHunter1-FastTeleport and all of its files?");
        check_long_question_shown()?;
        tap(232, 340);
        assert!(!answer.try_recv()?);

        check_question_gone()
    }
}
