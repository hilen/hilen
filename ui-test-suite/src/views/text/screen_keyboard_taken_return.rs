use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, NamedKey, Setup, TextAlignment, TextField, ViewData, ViewTest, view},
    ui_test::{
        check_colors, checkpoint, set_record_probe_count,
        system_input::{press_return, system_input, system_request, tap, type_text, wait_until},
    },
};

const NOTES: (f32, f32) = (540.0, 130.0);
const NAME: (f32, f32) = (540.0, 245.0);

/// The Return key of the keyboard the platform really has, taken by the
/// owner of the field with `take_keys`. A taken Return adds no line to a
/// text area and does not submit a single line field, both stay edited.
/// After `release_keys` the same key works as in every field. On a phone
/// the key goes through the system text field.
#[view]
struct ScreenKeyboardTakenReturn {
    taken:   usize,
    submits: Vec<String>,

    #[init]
    status:      Label,
    notes_title: Label,
    notes:       TextField,
    name_title:  Label,
    name:        TextField,
    log:         Label,
}

impl Setup for ScreenKeyboardTakenReturn {
    fn setup(self: Weak<Self>) {
        let title = |label: Weak<Label>, text: &str, y: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        title(self.status, "", 44.0);
        self.status.set_color("#DDE3E8");

        title(self.notes_title, "text area", 74.0);
        self.notes.set_text_size(24).set_multiline(true);
        self.notes.set_alignment(TextAlignment::Left);
        self.notes.place().t(100).lr(20).h(90);

        title(self.name_title, "single line field", 194.0);
        self.name.set_text_size(24);
        self.name.set_alignment(TextAlignment::Left);
        self.name.place().t(220).lr(20).h(50);

        self.log.set_text_size(18).set_multiline(true);
        self.log.set_alignment(TextAlignment::Left);
        self.log.place().t(280).lr(20).h(60);

        for field in [self.notes, self.name] {
            field.key_taken.val(move |_| self.note_taken());
            field.submitted.val(move |text| self.note_submit(text));
        }
        self.take(false);
    }
}

impl ScreenKeyboardTakenReturn {
    fn take(self: Weak<Self>, take: bool) {
        for field in [self.notes, self.name] {
            if take {
                field.take_keys([NamedKey::Enter]);
            } else {
                field.release_keys();
            }
        }
        self.status.set_text(if take {
            "the owner takes Return"
        } else {
            "the field has Return"
        });
        self.show_log();
    }

    fn note_taken(mut self: Weak<Self>) {
        self.taken += 1;
        self.show_log();
    }

    fn note_submit(mut self: Weak<Self>, text: String) {
        self.submits.push(text);
        self.show_log();
    }

    fn show_log(self: Weak<Self>) {
        self.log.set_text(format!(
            "times Return was taken: {}\nsubmitted: {}",
            self.taken,
            self.submits.join(", ")
        ));
    }
}

impl ViewTest for ScreenKeyboardTakenReturn {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        tap(NOTES.0, NOTES.1)?;
        wait_until("the text area is edited", move || view.notes.is_editing())?;
        type_text("One")?;
        wait_until("the text area holds One", move || view.notes.text() == "One")?;

        from_main(move || view.take(true));
        press_return()?;
        wait_until("the owner got Return", move || view.taken == 1)?;
        checkpoint("the owner takes Return, taken 1 time, the text area holds only One")?;
        from_main(move || {
            ensure!(
                view.notes.text() == "One",
                "a taken Return changed the text area: {:?}",
                view.notes.text()
            );
            ensure!(view.notes.is_editing(), "a taken Return ended the editing");
            Ok(())
        })?;

        from_main(move || view.take(false));
        press_return()?;
        type_text("Two")?;
        wait_until("the text area holds 2 lines", move || {
            view.notes.text() == "One\nTwo"
        })?;
        checkpoint("the field has Return, the text area shows One and Two on 2 lines")?;
        from_main(move || {
            ensure!(view.taken == 1, "Return was taken {} times", view.taken);
            Ok(())
        })?;

        tap(NAME.0, NAME.1)?;
        wait_until("the single line field is edited", move || view.name.is_editing())?;
        type_text("Go")?;
        wait_until("the single line field holds Go", move || view.name.text() == "Go")?;

        from_main(move || view.take(true));
        press_return()?;
        wait_until("the owner got Return again", move || view.taken == 2)?;
        checkpoint("the owner takes Return, taken 2 times, nothing is submitted")?;
        from_main(move || {
            ensure!(view.name.is_editing(), "a taken Return ended the editing");
            ensure!(
                view.submits.is_empty(),
                "a taken Return submitted {:?}",
                view.submits
            );
            Ok(())
        })?;

        from_main(move || view.take(false));
        press_return()?;
        wait_until("Return ended the editing", move || !view.name.is_editing())?;
        if system_input() {
            system_request("keyboard 0")?;
        }
        checkpoint("the field has Return, the log says taken 2 times and submitted Go")?;
        from_main(move || {
            ensure!(view.submits == ["Go"], "submitted {:?}", view.submits);
            ensure!(view.taken == 2, "Return was taken {} times", view.taken);
            Ok(())
        })?;

        check_colors(FINAL)
    }
}

const FINAL: &str = r"
      212   44 - #dde3e8
      256   44 - #dde3e8
      320   44 - #dde3e8
      392   44 - #dde3e8
      508   44 - #dde3e8
       56   52 - #dde3e8
       76   52 - #dde3e8
       92   52 - #dde3e8
      148   52 - #dde3e8
      152   52 - #dde3e8
      156   52 - #dde3e8
      288   52 - #dde3e8
       40   56 - #dde3e8
       80   56 - #dde3e8
      128   56 - #dde3e8
      172   56 - #020202
       56   60 - #dde3e8
       68   60 - #000000
       80   60 - #dde3e8
      108   60 - #dde3e8
      128   60 - #dde3e8
      140   60 - #040405
      184   60 - #dde3e8
      236   68 - #dde3e8
      340   68 - #dde3e8
      372   68 - #dde3e8
      416   68 - #dde3e8
      448   68 - #dde3e8
      484   68 - #dde3e8
      536   68 - #dde3e8
      576   68 - #dde3e8
       40   88 - #597c95
       52   88 - #597c95
       60   92 - #597c95
       84   92 - #597c95
      100   92 - #000000
       40  116 - #ffffff
      268  116 - #ffffff
      392  116 - #ffffff
       36  120 - #ffffff
       40  120 - #ffffff
       68  120 - #ffffff
       40  124 - #ffffff
      148  136 - #ffffff
      576  140 - #ffffff
      216  144 - #ffffff
       68  152 - #ffffff
       72  152 - #ffffff
      452  152 - #ffffff
      300  164 - #ffffff
      516  168 - #ffffff
      380  188 - #ffffff
      136  200 - #597c95
      152  204 - #000000
       64  208 - #597c95
       80  208 - #090c0f
      112  208 - #090c0f
      128  208 - #000000
       40  212 - #010102
      100  212 - #597c95
      240  224 - #ffffff
      476  224 - #ffffff
       40  240 - #ffffff
       36  244 - #ffffff
       56  244 - #ffffff
       40  248 - #ffffff
       56  248 - #ffffff
      540  264 - #ffffff
      308  268 - #ffffff
      412  268 - #ffffff
       92  296 - #597c95
      108  296 - #000000
       40  300 - #597c95
       68  300 - #000000
      184  300 - #597c95
      208  300 - #000000
      132  316 - #597c95
       60  320 - #597c95
       88  320 - #050708
      112  320 - #597c95
       52  324 - #000000
      132  324 - #597c95
      480  360 - #597c95
      368  364 - #597c95
      592  380 - #597c95
      280  424 - #597c95
        4  452 - #597c95
      420  456 - #597c95
      536  484 - #597c95
      148  492 - #597c95
      336  504 - #597c95
        4  592 - #597c95
      112  592 - #597c95
      288  592 - #597c95
      452  592 - #597c95
      592  592 - #597c95
";
