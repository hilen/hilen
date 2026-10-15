use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    system::Clipboard,
    ui::{
        CodeEditor, CodePos, Label, ModifiersState, NamedKey, Setup, UIManager, View, ViewData, ViewFrame,
        ViewTest, WHITE, view,
    },
    ui_test::{
        check_colors, inject_keys, inject_modifiers, inject_named_key, inject_right_click, inject_touches,
        set_record_probe_count,
    },
};

use super::common::{SCAD, click_at, color_all, quoted, screen_point, status_style};
use crate::text_points::{click, label_with};

/// Copy, cut and paste in a code editor, with the keys and with the right
/// click menu. The clipboard stays inside the test. The status label
/// shows the lines around the change.
#[view]
struct CodeEditorClipboard {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorClipboard {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(340);
        self.editor.set_text(SCAD);
        color_all(self.editor);
        self.editor.changed.val(move |_| color_all(self.editor));

        // Under the menu that opens at the last line.
        status_style(self.status);
        self.status.place().t(490).lr(10).b(10);
    }
}

impl CodeEditorClipboard {
    /// The last 3 lines, also written to the status label with the step.
    fn show(self: Weak<Self>, step: &'static str) -> Vec<String> {
        let lines = from_main(move || {
            let lines = self.editor.lines().to_vec();
            let tail = lines[lines.len().saturating_sub(3)..].to_vec();
            self.status.set_text(format!("{step}\nthe last lines:\n{}", quoted(&tail)));
            lines
        });
        wait_for_next_frame();
        lines
    }
}

fn command(ch: char) {
    inject_modifiers(ModifiersState::SUPER);
    inject_keys(ch);
    inject_modifiers(ModifiersState::empty());
}

impl ViewTest for CodeEditorClipboard {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        from_main(|| {
            Clipboard::set_in_process(true);
            UIManager::set_drag_scrolling(false);
        });

        // `cube(size);` of line 6.
        click_at(view.editor, CodePos::new(5, 8));
        inject_modifiers(ModifiersState::SHIFT);
        inject_named_key(NamedKey::End);
        inject_modifiers(ModifiersState::empty());
        command('c');
        let copied = from_main(Clipboard::get_text)?;
        ensure!(copied == "cube(size);", "the copy holds {copied:?}");

        click_at(view.editor, CodePos::new(11, 8));
        inject_named_key(NamedKey::Enter);
        command('v');
        let lines = view.show("a copy of `cube(size);` pasted on a new line after `plate();`");
        ensure!(lines[12] == "cube(size);", "the paste gave {lines:?}");
        ensure!(lines.len() == 14, "the paste gave {} lines", lines.len());
        check_colors(PASTED)?;

        // The last line cut whole, with the line break before it.
        click_at(view.editor, CodePos::new(12, 11));
        inject_modifiers(ModifiersState::SHIFT);
        inject_named_key(NamedKey::ArrowDown);
        inject_named_key(NamedKey::End);
        inject_modifiers(ModifiersState::empty());
        command('x');
        let lines = view.show("Shift with Down and End from the end of line 13, and a cut");
        ensure!(
            lines.last().map(String::as_str) == Some("cube(size);"),
            "the cut gave {lines:?}"
        );
        let cut = from_main(Clipboard::get_text)?;
        ensure!(cut == "\nundefined_part();", "the cut holds {cut:?}");

        // The right click menu pastes it back.
        let at = from_main(move || screen_point(view.editor, CodePos::new(12, 11)));
        inject_right_click(at.0, at.1);
        wait_for_next_frame();
        view.show("a right click at the end of the last line opens the menu");
        check_colors(MENU)?;

        let paste = from_main(|| {
            let item = label_with(UIManager::root_view().weak_view(), "Paste");
            let frame = item.absolute_frame();
            (frame.x() + frame.width() / 2.0, frame.y() + frame.height() / 2.0)
        });
        inject_touches(click(paste));
        let lines = view.show("Paste of the menu puts the cut line back");
        ensure!(
            lines[lines.len() - 2..] == ["cube(size);", "undefined_part();"],
            "the menu paste gave {lines:?}"
        );
        let editing = from_main(move || view.editor.is_editing());
        ensure!(editing, "the editor lost the keys to the menu");
        check_colors(MENU_PASTED)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const PASTED: &str = r"
             400    4 - #ffffff
             592    4 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             168   24 - #747c86
             184   24 - #6f7881
             200   24 - #747c86
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
             156   88 - #ffffff
             124  108 - #a17ce7
              88  112 - #8250df
             124  112 - #a17ce7
             140  112 - #c7b0f1
             124  132 - #c1a9ef
             140  132 - #8453e0
             180  132 - #282d33
              28  148 - #bbc1c8
             124  152 - #a17ce7
             124  156 - #a17ce7
             136  156 - #8251df
             180  176 - #c7b0f1
             204  176 - #a17ce7
             220  176 - #9c9ea1
             284  176 - #5b5f63
             316  176 - #ffffff
             260  180 - #9bb9df
              32  196 - #8d96a0
              52  264 - #c7b1f1
              84  264 - #8453e0
              64  288 - #8250df
             592  296 - #ffffff
              52  308 - #939598
              76  308 - #282d33
             104  308 - #272c32
             148  308 - #5b5f63
             416  324 - #ffffff
             388  500 - #d1d1d1
             104  504 - #ffffff
             256  504 - #d9d9d9
             284  504 - #000000
             312  504 - #525252
             328  504 - #d2d2d2
             360  504 - #d2d2d2
             420  504 - #ffffff
             188  508 - #000000
              28  520 - #000000
              84  524 - #000000
              80  560 - #525252
              96  560 - #9f9f9f
              56  572 - #515151
              32  576 - #2c2c2c
             144  576 - #ffffff
              68  580 - #010101
             100  580 - #9e9e9e
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const MENU: &str = r"
             400    4 - #ffffff
             592    4 - #ffffff
             160   20 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             196   24 - #bec2c6
              76   44 - #282d33
              32   84 - #d5d9dd
              72   84 - #cf222e
              56   88 - #eba1a6
              72   88 - #cf232f
              76   88 - #e79298
              80   88 - #cf222e
              92   88 - #d02632
             108   88 - #c7b1f1
             140   88 - #8453e0
             124  112 - #a17ce7
             140  132 - #8453e0
             180  132 - #282d33
              28  148 - #bbc1c8
             124  152 - #a17ce7
             124  156 - #a17ce7
             136  156 - #8251df
             180  176 - #c7b0f1
             204  176 - #a17ce7
             284  176 - #5b5f63
             316  176 - #ffffff
             260  180 - #9bb9df
              32  196 - #8d96a0
              52  264 - #c7b1f1
              72  264 - #8251df
              60  284 - #c1a9ef
             236  284 - #dadada
             112  288 - #ffffff
             300  292 - #d7d7d7
             172  300 - #d9d9d9
             592  300 - #ffffff
             196  332 - #8e8e93
             136  348 - #cbcbcb
             300  348 - #d3d3d3
             176  368 - #929297
             184  396 - #8e8e93
             300  404 - #d3d3d3
             136  408 - #cbcbcb
             192  424 - #b8b8b8
             136  456 - #cbcbcb
             184  480 - #b3b3b3
             240  480 - #b3b3b3
             288  480 - #b3b3b3
             340  500 - #000000
              64  504 - #404040
              88  504 - #858585
             148  504 - #d2d2d2
             180  504 - #000000
             216  504 - #7f7f7f
             276  504 - #000000
             304  504 - #ffffff
             380  504 - #000000
              32  516 - #4d4d4d
              32  524 - #4d4d4d
              92  524 - #282828
              32  564 - #181818
              80  580 - #525252
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const MENU_PASTED: &str = r"
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
              56   88 - #eba1a6
              72   88 - #cf232f
              76   88 - #e79298
              80   88 - #cf222e
              92   88 - #d02632
             108   88 - #c7b1f1
             140   88 - #8453e0
             124  108 - #a17ce7
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
             180  176 - #c7b0f1
             204  176 - #a17ce7
             284  176 - #5b5f63
             316  176 - #ffffff
             260  180 - #9bb9df
              32  196 - #8d96a0
             520  196 - #ffffff
              52  264 - #c7b1f1
              84  264 - #8453e0
              64  288 - #8250df
              52  308 - #939598
             104  308 - #272c32
             148  308 - #5b5f63
             592  384 - #ffffff
              32  500 - #ffffff
              96  500 - #000000
             304  500 - #676767
              64  504 - #d2d2d2
             116  504 - #e2e2e2
             144  504 - #999999
             176  504 - #5d5d5d
             208  504 - #c0c0c0
             240  504 - #848484
             268  504 - #d2d2d2
              28  520 - #000000
              92  524 - #282828
              64  536 - #ffffff
              32  540 - #000000
              80  560 - #525252
              96  560 - #9f9f9f
              56  572 - #515151
              32  576 - #2c2c2c
             128  576 - #000000
              68  580 - #010101
             100  580 - #9e9e9e
             508  592 - #ffffff
";
