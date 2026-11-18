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
    fn setup(self: Weak<Self>) {
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
                4    4 - #597c95
              300    4 - #597c95
              408    4 - #597c95
              500    4 - #597c95
              592    4 - #597c95
               76   72 - #597c95
              104   72 - #597c95
              112   72 - #000000
              168   72 - #597c95
              104   76 - #597c95
              148   76 - #090c0f
              168   76 - #597c95
              180   76 - #000001
              232   76 - #000001
              288   92 - #ffffff
              348   92 - #ffffff
              468   92 - #ffffff
              536   92 - #ffffff
               40  108 - #ffffff
               56  108 - #000000
               36  112 - #ffffff
               40  112 - #ffffff
               72  112 - #000000
              408  112 - #ffffff
               40  116 - #ffffff
               64  116 - #010101
               36  132 - #000000
              212  132 - #ffffff
               48  140 - #000000
               68  140 - #ffffff
               72  140 - #ffffff
              152  148 - #ffffff
              576  148 - #ffffff
              284  152 - #ffffff
              500  168 - #ffffff
              392  172 - #ffffff
              336  176 - #ffffff
              208  192 - #ffffff
              120  212 - #ffffff
               20  228 - #ffffff
              320  232 - #ffffff
              172  236 - #ffffff
              256  236 - #ffffff
              384  236 - #ffffff
              460  236 - #ffffff
              552  236 - #ffffff
              180  296 - #597c95
               40  300 - #597c95
               68  300 - #000000
               96  300 - #000000
              108  300 - #597c95
              128  300 - #000001
              148  300 - #000000
              160  300 - #597c95
              164  300 - #000000
              180  300 - #597c95
              208  316 - #11181d
               40  320 - #597c95
              100  320 - #597c95
              112  320 - #0e1317
              148  320 - #597c95
              168  320 - #597c95
              188  320 - #597c95
              208  320 - #11181d
              300  320 - #597c95
               44  324 - #090c0f
               60  324 - #0c1114
               76  324 - #597c95
              188  324 - #597c95
              208  324 - #11181d
              496  328 - #597c95
              400  336 - #597c95
              592  352 - #597c95
               88  408 - #597c95
              516  408 - #597c95
              312  412 - #597c95
                4  432 - #597c95
              168  432 - #597c95
              444  444 - #597c95
              244  460 - #597c95
              564  472 - #597c95
              352  504 - #597c95
               52  540 - #0b0f12
               72  540 - #597c95
               84  540 - #597c95
               92  540 - #000000
              140  540 - #000000
              192  540 - #597c95
              216  540 - #597c95
              256  540 - #597c95
              268  540 - #050708
              116  544 - #000001
              496  548 - #597c95
              324  592 - #597c95
              404  592 - #597c95
              592  592 - #597c95
";
