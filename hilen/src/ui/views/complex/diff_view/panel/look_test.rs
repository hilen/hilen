//! `Diff panel look`: 1 changed Rust file from a host in memory, in 1
//! column and in 2, with its syntax colors.

use std::sync::Arc;

use anyhow::{Result, ensure};
use filediff::{DiffHunk, DiffLine, DiffTarget, FileDiff, FileDiffBody};
use parking_lot::Mutex;

use crate::{
    self as hilen,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        DiffHost, DiffPanel, DiffShown, Font, Label, Setup, TextAlignment, ViewData, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

/// The syntax colors come from a thread. A test frame waits for them at
/// most this many frames.
const COLOR_FRAMES: usize = 600;

/// What the panel asked of its host, in order.
pub(super) type Calls = Arc<Mutex<Vec<String>>>;

/// A host with 1 diff in memory, it notes every call of the panel.
pub(super) struct MemoryHost {
    pub(super) calls: Calls,
}

impl DiffHost for MemoryHost {
    fn load(&self, shown: &DiffShown, done: Box<dyn FnOnce(Result<FileDiff, String>) + Send>) {
        self.calls.lock().push(format!("load {}", shown.file));
        done(Ok(sample()));
    }

    fn discard(&self, patch: String, done: Box<dyn FnOnce(Result<(), String>) + Send>) {
        self.calls.lock().push(format!("discard {patch}"));
        done(Ok(()));
    }

    fn confirm(&self, question: String, action: &'static str, yes: Box<dyn FnOnce() + Send>) {
        self.calls.lock().push(format!("confirm {question} {action}"));
        yes();
    }

    fn report(&self, result: Result<(), String>, done_text: &'static str) {
        self.calls.lock().push(format!("report {done_text} {result:?}"));
    }

    fn close(&self) {
        self.calls.lock().push("close".to_string());
    }

    fn mono(&self) -> Weak<Font> {
        Font::mono()
    }

    fn mode_changed(&self, split: bool) {
        self.calls.lock().push(format!("split {split}"));
    }
}

fn line(kind: &str, text: &str, old_no: Option<u32>, new_no: Option<u32>) -> DiffLine {
    DiffLine {
        kind: kind.to_string(),
        text: text.to_string(),
        old_no,
        new_no,
    }
}

/// 1 hunk of a Rust file: a changed line, an added line and the context
/// around them.
pub(super) fn sample() -> FileDiff {
    FileDiff {
        path:       "src/main.rs".to_string(),
        old_path:   None,
        is_new:     false,
        is_deleted: false,
        body:       FileDiffBody::Text {
            hunks: vec![DiffHunk {
                old_start: 1,
                old_lines: 4,
                new_start: 1,
                new_lines: 5,
                lines:     vec![
                    line("ctx", "fn main() {", Some(1), Some(1)),
                    line("rem", "    let n = 1;", Some(2), None),
                    line("add", "    let n = 2;", None, Some(2)),
                    line("add", "    let m = 3;", None, Some(3)),
                    line("ctx", "    run(n + m);", Some(3), Some(4)),
                    line("ctx", "}", Some(4), Some(5)),
                ],
            }],
        },
    }
}

/// A diff panel with 1 changed Rust file from a host in memory, first in
/// 1 column, then in 2.
#[view]
struct DiffPanelLook {
    calls: Calls,

    #[init]
    panel:  DiffPanel,
    status: Label,
}

impl Setup for DiffPanelLook {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.panel.place().t(10).lr(10).h(400);
        self.status.set_text_size(16).set_multiline(true);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().t(420).lr(10).b(10);

        self.panel.set_host(MemoryHost {
            calls: self.calls.clone(),
        });
        self.panel.show(
            Some(DiffShown {
                file:     "src/main.rs".to_string(),
                old_path: None,
                target:   DiffTarget::WorkingTree,
            }),
            1,
            true,
        );
    }
}

impl ViewTest for DiffPanelLook {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        from_main(move || {
            view.status.set_text(
                "unified: the file name on top, the old line red, the 2 new lines green, the numbers of \
                 both sides in the gutter",
            );
        });
        let mut frames = 0;
        while from_main(move || view.panel.runs.is_empty()) {
            ensure!(
                frames < COLOR_FRAMES,
                "the diff got no syntax colors in {COLOR_FRAMES} frames"
            );
            wait_for_next_frame();
            frames += 1;
        }
        wait_for_next_frame();
        let calls = from_main(move || view.calls.lock().clone());
        ensure!(calls == ["load src/main.rs"], "the panel asked {calls:?}");
        check_colors(UNIFIED)?;

        from_main(move || {
            view.panel.set_split(true);
            view.status
                .set_text("split: the old line on the left, the new lines on the right");
        });
        wait_for_next_frame();
        check_colors(SPLIT)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const UNIFIED: &str = r"
             592    4 - #ffffff
              40   28 - #c0c0c0
              64   28 - #787878
              96   28 - #b4b4b4
             540   28 - #ededed
             148   52 - #aaaaaa
             156   52 - #a6a6a6
             188   52 - #b6b6b6
             128   72 - #ccbaee
             148   72 - #2b7dde
             160   72 - #2378dd
             324   80 - #f5e4e4
             200   88 - #986ce0
             216   88 - #b85d18
             492   88 - #f5e4e4
              12   92 - #f5e4e4
             172  108 - #9673e1
             184  108 - #959f9c
             200  108 - #bcb4e6
             200  124 - #946ee1
             424  128 - #dff0ea
             556  132 - #dff0ea
             156  144 - #b6d0ee
             184  144 - #a3a3a3
             216  144 - #858585
              32  160 - #898989
             592  280 - #ffffff
             328  300 - #f6f6f6
             148  324 - #f6f6f6
               4  348 - #ffffff
              72  492 - #383838
             236  492 - #1a1a1a
             332  492 - #050505
             416  492 - #545454
              28  496 - #7c7c7c
              84  496 - #242424
             100  496 - #1a1a1a
             140  496 - #000000
             232  496 - #3d3d3d
             248  496 - #1a1a1a
             272  496 - #ffffff
             288  496 - #3b3b3b
             312  496 - #676767
             344  496 - #bdbdbd
             416  496 - #545454
             456  496 - #ffffff
             484  496 - #000000
             500  496 - #1e1e1e
             516  496 - #1a1a1a
             200  500 - #ffffff
             372  500 - #000000
             112  512 - #989898
             212  512 - #808080
              32  516 - #4c4c4c
              36  516 - #000000
              56  516 - #5b5b5b
              76  516 - #4d4d4d
             112  516 - #989898
             132  516 - #575757
             164  516 - #ffffff
             192  516 - #d4d4d4
             208  516 - #000000
             224  516 - #ffffff
             252  516 - #1e1e1e
";

/// Recorded with `--record-colors`.
const SPLIT: &str = r"
             540   24 - #ededed
              40   28 - #c0c0c0
              60   28 - #9f9f9f
              64   28 - #787878
              96   28 - #b4b4b4
             548   28 - #ededed
             148   52 - #aaaaaa
             156   52 - #a6a6a6
             188   52 - #b6b6b6
              88   72 - #ccbaee
             108   72 - #2b7dde
             120   72 - #2378dd
             140   72 - #f6f6f6
             384   72 - #9b74e4
             400   72 - #adcaed
             404   72 - #b7d0ef
             416   72 - #91baea
             420   72 - #3d88e0
             436   72 - #f6f6f6
             560   80 - #dff0ea
             160   88 - #986ce0
             176   88 - #b85d18
             408   88 - #9068e1
             452   88 - #946ee1
             320   92 - #b2bcb8
              12   96 - #f5e4e4
             248   96 - #f5e4e4
             520  100 - #dff0ea
             408  108 - #9068e1
             416  108 - #916ae1
             436  108 - #a3aeaa
             440  108 - #abb7b3
             452  108 - #bcb4e6
             360  112 - #dff0ea
             588  112 - #dff0ea
             484  244 - #f6f6f6
             104  292 - #f6f6f6
             312  300 - #f6f6f6
             592  332 - #ffffff
             448  376 - #f6f6f6
              44  500 - #6b6b6b
             196  500 - #3b3b3b
             232  500 - #dadada
              36  504 - #000000
              44  504 - #6b6b6b
              48  504 - #525252
             196  504 - #3b3b3b
             252  504 - #000000
             332  504 - #000000
             376  504 - #9e9e9e
              44  508 - #6b6b6b
              48  508 - #525252
              76  508 - #000000
             132  508 - #d9d9d9
             156  508 - #000000
             180  508 - #737373
             196  508 - #3b3b3b
             208  508 - #505050
             236  508 - #010101
             264  508 - #ffffff
             332  508 - #010101
             356  508 - #8d8d8d
             372  508 - #747474
             592  592 - #ffffff
";
