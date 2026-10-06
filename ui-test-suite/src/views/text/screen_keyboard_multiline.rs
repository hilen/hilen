use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, TextAlignment, TextField, ViewData, ViewTest, view},
    ui_test::{
        check_colors, checkpoint, set_record_probe_count,
        system_input::{press_return, system_input, system_request, tap, type_text, wait_until},
    },
};

const NOTES: (f32, f32) = (540.0, 110.0);
const OUTSIDE: (f32, f32) = (300.0, 540.0);

/// A text area typed with the keyboard the platform really has. Return is a
/// new line there, it neither ends the editing nor submits.
#[view]
struct ScreenKeyboardMultiline {
    submits: usize,
    ended:   Vec<String>,

    #[init]
    notes_title:   Label,
    notes:         TextField,
    log:           Label,
    outside_title: Label,
}

impl Setup for ScreenKeyboardMultiline {
    fn setup(mut self: Weak<Self>) {
        let title = |label: Weak<Label>, text: &str, y: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        title(self.notes_title, "notes, Return is a new line", 60.0);
        self.notes.set_text_size(24).set_multiline(true);
        self.notes.set_alignment(TextAlignment::Left);
        self.notes.place().t(90).lr(20).h(150);
        self.notes.submitted.val(move |_| self.note_submit());
        self.notes.editing_ended.val(move |text| self.note_ended(text));

        self.log.set_text_size(18).set_multiline(true);
        self.log.set_alignment(TextAlignment::Left);
        self.log.place().t(270).lr(20).h(80);
        self.show_log();

        title(self.outside_title, "a tap down here ends the editing", 527.0);
    }
}

impl ScreenKeyboardMultiline {
    fn note_submit(mut self: Weak<Self>) {
        self.submits += 1;
        self.show_log();
    }

    fn note_ended(mut self: Weak<Self>, text: String) {
        self.ended.push(text);
        self.show_log();
    }

    fn show_log(self: Weak<Self>) {
        self.log.set_text(format!(
            "times submitted: {}\ntimes editing ended: {}",
            self.submits,
            self.ended.len()
        ));
    }
}

impl ViewTest for ScreenKeyboardMultiline {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        tap(NOTES.0, NOTES.1)?;
        wait_until("notes is edited", move || view.notes.is_editing())?;

        type_text("One")?;
        press_return()?;
        type_text("Two")?;
        wait_until("notes holds 2 lines", move || view.notes.text() == "One\nTwo")?;
        checkpoint("notes is still edited and shows One and Two on 2 lines")?;
        from_main(move || {
            ensure!(view.notes.is_editing(), "Return ended the editing of a text area");
            ensure!(view.submits == 0, "a text area submitted {} times", view.submits);
            Ok(())
        })?;

        tap(OUTSIDE.0, OUTSIDE.1)?;
        wait_until("notes is not edited any more", move || !view.notes.is_editing())?;
        if system_input() {
            system_request("keyboard 0")?;
        }
        checkpoint("One and Two on 2 lines, times submitted 0, times editing ended 1")?;
        from_main(move || {
            ensure!(
                view.notes.text() == "One\nTwo",
                "notes holds {:?}",
                view.notes.text()
            );
            ensure!(view.ended == ["One\nTwo"], "ended with {:?}", view.ended);
            ensure!(view.submits == 0, "a text area submitted {} times", view.submits);
            Ok(())
        })?;

        check_colors(FINAL)
    }
}

const FINAL: &str = r"
             380    4 - #597c95
             492    4 - #597c95
             592    4 - #597c95
              76   72 - #597c95
             116   72 - #3f5869
             124   72 - #486478
             132   72 - #425c6e
             148   72 - #090c0f
             180   72 - #000000
              44   76 - #19232a
             104   76 - #597c95
             128   76 - #425b6e
             140   76 - #466175
             168   76 - #597c95
             232   76 - #000000
             308   92 - #ffffff
             380   92 - #ffffff
             452   92 - #ffffff
             536   92 - #ffffff
              44  108 - #ffffff
              60  108 - #000000
              40  112 - #ffffff
              44  112 - #ffffff
              72  112 - #cdcdcd
              44  116 - #ffffff
              56  116 - #aeaeae
              64  116 - #e0e0e0
              44  136 - #b7b7b7
              44  140 - #b7b7b7
              72  140 - #ffffff
              76  140 - #ffffff
              44  144 - #b7b7b7
             264  156 - #ffffff
             372  164 - #ffffff
             576  164 - #ffffff
             480  168 - #ffffff
             192  188 - #ffffff
             112  216 - #ffffff
              20  220 - #ffffff
             300  228 - #ffffff
             436  236 - #ffffff
             524  236 - #ffffff
             180  296 - #597c95
              40  300 - #597c95
              60  300 - #0c1114
              96  300 - #000000
             108  300 - #597c95
             116  300 - #000000
             136  300 - #374d5c
             152  300 - #000001
             164  300 - #000000
             172  316 - #273641
             192  316 - #415b6d
             208  316 - #11181d
              40  320 - #597c95
             100  320 - #597c95
             148  320 - #597c95
             168  320 - #597c95
             208  320 - #11181d
              48  324 - #000000
              60  324 - #0c1114
              76  324 - #597c95
             116  324 - #3c5465
             128  324 - #344857
             156  324 - #273641
             172  324 - #273641
             188  324 - #597c95
             208  324 - #11181d
             352  324 - #597c95
             492  340 - #597c95
             592  380 - #597c95
             412  416 - #597c95
             288  420 - #597c95
               4  432 - #597c95
             164  432 - #597c95
             520  484 - #597c95
             372  504 - #597c95
             196  536 - #273641
              52  540 - #0b0f12
              68  540 - #000000
              84  540 - #597c95
              96  540 - #597c95
             116  540 - #000000
             144  540 - #334756
             172  540 - #334756
             232  540 - #334756
             256  540 - #597c95
             272  540 - #456073
             136  544 - #000000
             180  544 - #1e2a32
             196  544 - #273641
             220  544 - #394f5f
             264  544 - #476377
             284  544 - #2b3c48
             436  580 - #597c95
             592  592 - #597c95
";
