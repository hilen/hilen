use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, Label, ModifiersState, NamedKey, Setup, UIManager, ViewData, ViewTest, WHITE,
        view,
    },
    ui_test::{check_colors, inject_keys, inject_modifiers, inject_named_key, set_record_probe_count},
};

use super::common::{SCAD, click_at, color_all, drag_over, status_style};
use crate::text_points::shown;

/// Selecting code: a drag over lines, a double click on a name, a triple
/// click on a line, Shift with the arrows, Cmd or Ctrl with A, and Escape
/// that drops the selection. The status label shows what is selected,
/// line breaks written as `\n`.
#[view]
struct CodeEditorSelection {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorSelection {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(340);
        self.editor.set_text(SCAD);
        color_all(self.editor);

        status_style(self.status);
        self.status.place().t(360).lr(10).b(10);
    }
}

impl CodeEditorSelection {
    fn show(self: Weak<Self>, step: &'static str) -> String {
        let text = from_main(move || {
            let text = self.editor.selected_text();
            self.status.set_text(format!("{step}\nselected: {}", shown(&text)));
            text
        });
        wait_for_next_frame();
        text
    }
}

impl ViewTest for CodeEditorSelection {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        // A phone scrolls with a drag by default, this test is about the
        // drag that selects.
        from_main(|| UIManager::set_drag_scrolling(false));

        drag_over(view.editor, CodePos::new(3, 7), CodePos::new(5, 12));
        let text = view.show("a drag from `plate` on line 4 to after `cube` on line 6");
        ensure!(
            text == "plate() {\n    difference() {\n        cube",
            "the drag selected {text:?}"
        );
        check_colors(DRAG)?;

        click_at(view.editor, CodePos::new(7, 15));
        click_at(view.editor, CodePos::new(7, 15));
        let text = view.show("a double click on `cylinder`");
        ensure!(text == "cylinder", "the double click selected {text:?}");

        click_at(view.editor, CodePos::new(7, 15));
        let text = view.show("a third click takes the line");
        ensure!(
            text == "            cylinder(h = 12, r = 5);\n",
            "the triple click selected {text:?}"
        );
        check_colors(LINE)?;

        click_at(view.editor, CodePos::new(1, 0));
        inject_modifiers(ModifiersState::SHIFT);
        inject_named_key(NamedKey::ArrowDown);
        inject_named_key(NamedKey::ArrowDown);
        inject_named_key(NamedKey::End);
        inject_modifiers(ModifiersState::empty());
        let text = view.show("Shift with Down, Down and End from the start of line 2");
        ensure!(
            text == "size = [40, 30, 10];\n\nmodule plate() {",
            "Shift and the arrows selected {text:?}"
        );

        inject_modifiers(ModifiersState::SUPER);
        inject_keys('a');
        inject_modifiers(ModifiersState::empty());
        let text = view.show("Cmd or Ctrl with A takes all");
        ensure!(text == SCAD, "select all took {text:?}");
        check_colors(ALL)?;

        inject_named_key(NamedKey::Escape);
        let (text, editing) = from_main(move || (view.editor.selected_text(), view.editor.is_editing()));
        view.show("Escape drops the selection, the editor keeps the keys");
        ensure!(text.is_empty(), "Escape left {text:?}");
        ensure!(editing, "the first Escape ended the editing");

        inject_named_key(NamedKey::Escape);
        let editing = from_main(move || view.editor.is_editing());
        view.show("a second Escape ends the editing, the caret is gone");
        ensure!(!editing, "the second Escape kept the editing");
        check_colors(ENDED)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const DRAG: &str = r"
             400    4 - #ffffff
             592    4 - #ffffff
             160   20 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             200   24 - #747c86
              76   44 - #282d33
              28   48 - #ccd0d6
              32   84 - #d5d9dd
              72   84 - #cf222e
             128   84 - #b8d2ff
             156   84 - #b8d2ff
              56   88 - #eba1a6
              72   88 - #cf232f
              76   88 - #e79298
              80   88 - #cf222e
              92   88 - #d02632
             108   88 - #a098f1
             172   96 - #b8d2ff
              92  104 - #b8d2ff
             124  108 - #9071e7
             200  108 - #b8d2ff
             144  112 - #8352e0
             172  112 - #b8d2ff
              52  120 - #b8d2ff
             128  132 - #8250df
             180  132 - #282d33
             100  136 - #b8d2ff
              72  140 - #b8d2ff
              28  148 - #bbc1c8
             124  156 - #a17ce7
             140  156 - #c7b0f1
             168  156 - #8251df
             180  176 - #c7b0f1
             204  176 - #a17ce7
             220  176 - #9c9ea1
             284  176 - #5b5f63
             316  176 - #ffffff
             260  180 - #9bb9df
              32  196 - #8d96a0
              52  264 - #c7b1f1
              72  264 - #8251df
              60  288 - #9c9ea1
             104  288 - #272c32
             148  288 - #5b5f63
             592  300 - #ffffff
              48  464 - #cdcdcd
             128  464 - #080808
             172  464 - #000000
             296  464 - #161616
              76  468 - #505050
             100  468 - #b0b0b0
             188  468 - #9e9e9e
             216  468 - #8a8a8a
             260  468 - #000000
             352  468 - #b8b8b8
              44  484 - #000000
              84  484 - #000000
             104  484 - #525252
             120  484 - #ffffff
             148  484 - #ffffff
             192  484 - #3b3b3b
             324  484 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const LINE: &str = r"
             404    4 - #ffffff
             592    4 - #ffffff
             160   20 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             200   24 - #747c86
             120   40 - #b1c8e6
              28   48 - #ccd0d6
              32   84 - #d5d9dd
              72   84 - #cf222e
              56   88 - #eba1a6
              72   88 - #cf232f
              76   88 - #e79298
              80   88 - #cf222e
              92   88 - #d02632
             128   88 - #8251df
             124  108 - #a17ce7
              88  112 - #8250df
             140  112 - #c7b0f1
             124  132 - #c1a9ef
             140  132 - #8453e0
             180  132 - #282d33
              28  148 - #bbc1c8
             124  152 - #a17ce7
             204  152 - #ffffff
             124  156 - #a17ce7
             140  156 - #c7b0f1
             168  156 - #8251df
              60  164 - #b8d2ff
             236  164 - #b8d2ff
             324  172 - #b8d2ff
             152  176 - #b8d2ff
             164  176 - #b8d2ff
             180  176 - #a097f1
             284  176 - #495363
             208  180 - #b8d2ff
             260  180 - #709edf
              88  184 - #b8d2ff
             120  184 - #b8d2ff
              32  196 - #8d96a0
              52  264 - #c7b1f1
              84  264 - #8453e0
              52  284 - #939598
              56  284 - #24292f
             120  284 - #24292f
             148  284 - #5b5f63
              60  288 - #9c9ea1
             100  288 - #9c9ea1
             144  288 - #252a30
             148  288 - #5b5f63
             172  288 - #ffffff
             588  300 - #ffffff
              88  464 - #525252
             156  464 - #000000
             116  468 - #ffffff
             196  468 - #d9d9d9
              44  480 - #000000
             160  480 - #bababa
              84  484 - #000000
             172  484 - #000000
             192  484 - #e6e6e6
             232  484 - #737373
             292  484 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const ALL: &str = r"
             592    4 - #ffffff
             232   12 - #b8d2ff
              52   16 - #b8d2ff
             152   16 - #b8d2ff
             196   24 - #97a9c6
             120   40 - #80a9e6
              76   44 - #272c33
             412   44 - #ffffff
              28   48 - #ccd0d6
              72   84 - #cf222e
              56   88 - #c287a6
              72   88 - #cf232f
              76   88 - #c37b98
              80   88 - #cf222e
              92   88 - #cf2532
             128   88 - #8251df
             192  108 - #b8d2ff
             140  112 - #a097f1
             124  132 - #9d92ef
              28  148 - #bbc1c8
              80  156 - #b8d2ff
             144  156 - #8352e0
             344  164 - #b8d2ff
             180  176 - #a097f1
             204  176 - #9071e7
             284  176 - #495363
             260  180 - #709edf
              32  196 - #8d96a0
              96  204 - #b8d2ff
              52  220 - #b8d2ff
             520  240 - #ffffff
              52  264 - #a098f1
              84  264 - #8353e0
             148  284 - #495363
             184  284 - #b8d2ff
              60  288 - #7586a1
             120  288 - #24292f
             376  316 - #ffffff
             104  444 - #383838
             160  448 - #d1d1d1
              64  452 - #ffffff
             300  460 - #0d0d0d
             228  464 - #545454
             452  464 - #050505
             132  468 - #6b6b6b
             188  468 - #e5e5e5
             264  468 - #545454
             412  468 - #a9a9a9
             416  468 - #171717
             484  468 - #ffffff
              32  480 - #6e6e6e
             360  480 - #686868
             176  484 - #afafaf
             536  484 - #ffffff
             104  488 - #ffffff
              64  504 - #3a3a3a
             136  504 - #000000
             204  504 - #d6d6d6
             236  504 - #e6e6e6
             260  504 - #000000
             296  504 - #8d8d8d
             324  504 - #000000
             180  508 - #989898
             316  508 - #9a9a9a
";

/// Recorded with `--record-colors`.
const ENDED: &str = r"
             400    4 - #ffffff
             592    4 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             168   24 - #747c86
             196   24 - #bec2c6
             200   24 - #747c86
             120   40 - #b1c8e6
              76   44 - #282d33
              28   48 - #ccd0d6
              32   84 - #d5d9dd
              72   84 - #cf222e
              56   88 - #eba1a6
              72   88 - #cf232f
              76   88 - #e79298
              80   88 - #cf222e
              92   88 - #d02632
             108   88 - #c7b1f1
             140   88 - #8453e0
             172  108 - #ffffff
              88  112 - #8250df
             124  112 - #a17ce7
             140  112 - #c7b0f1
             132  132 - #c6aff0
             180  132 - #282d33
              28  148 - #bbc1c8
             124  152 - #a17ce7
             124  156 - #a17ce7
             136  156 - #8251df
             168  156 - #8251df
             152  176 - #ffffff
             180  176 - #c7b0f1
             204  176 - #a17ce7
             220  176 - #9c9ea1
             284  176 - #5b5f63
             316  176 - #ffffff
             260  180 - #9bb9df
              32  196 - #8d96a0
              52  264 - #c7b1f1
              72  264 - #8251df
              52  284 - #939598
             148  284 - #5b5f63
              72  288 - #24292f
             104  288 - #272c32
             148  288 - #5b5f63
             592  296 - #ffffff
             232  460 - #eaeaea
             100  464 - #9d9d9d
             196  464 - #000000
             232  464 - #eaeaea
             316  464 - #000000
             340  464 - #a1a1a1
              52  468 - #ffffff
              80  468 - #5a5a5a
             160  468 - #343434
             200  468 - #b2b2b2
             232  468 - #eaeaea
             252  468 - #7f7f7f
             284  468 - #d9d9d9
             360  468 - #000000
              32  484 - #ffffff
              48  484 - #000000
              84  484 - #000000
             592  592 - #ffffff
";
