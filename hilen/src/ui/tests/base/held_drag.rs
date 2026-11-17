use std::{thread::sleep, time::Duration};

use anyhow::{Result, ensure};

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLUE, GREEN},
    ui::{Container, Label, LongPress, Setup, ViewData, ViewTest, ViewTouch, view},
    ui_test::{checkpoint, inject_touches},
};

/// A drag that rests for the time of a long press, a few points from
/// where it began, still gets its later moves and its end. Before the fix
/// the long press took the touch away from a view that has nothing on a
/// hold, and the view stayed in the middle of its drag for good.
#[view]
struct HeldDrag {
    moves: u32,
    ends:  u32,

    #[init]
    pad:    Container,
    name:   Label,
    counts: Label,
}

impl Setup for HeldDrag {
    fn setup(mut self: Weak<Self>) {
        self.pad.set_color(BLUE).place().tl(40).size(400, 200);
        self.pad.enable_touch();
        self.pad.touch().all.val(move |touch| {
            if touch.is_moved() {
                self.moves += 1;
            }
            if touch.is_ended() {
                self.ends += 1;
                self.pad.set_color(GREEN);
            }
            self.show();
        });

        self.name
            .set_text("drag pad, no hold action")
            .place()
            .l(40)
            .t(250)
            .size(400, 30);
        self.counts.place().l(40).t(290).size(400, 30);
        self.show();
    }
}

impl HeldDrag {
    fn show(self: Weak<Self>) {
        self.counts.set_text(format!("moves {}, ends {}", self.moves, self.ends));
    }
}

impl ViewTest for HeldDrag {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // A press and a move of 4 points, less than the slop of a long press.
        inject_touches(
            "
            100 100 b
            104 100 m
        ",
        );
        sleep(Duration::from_secs_f32(LongPress::DURATION + 0.2));
        inject_touches(
            "
            160 100 m
            160 100 e
        ",
        );

        let got = from_main(move || (view.moves, view.ends));
        ensure!(
            got == (2, 1),
            "expected 2 moves and 1 end after a rest in the drag, got {got:?}"
        );
        checkpoint("the pad is green, the drag that rested still got its end")
    }
}
