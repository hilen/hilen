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
    fn setup(mut self: Weak<Self>) {
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
     392   44 - #dde3e8
     508   44 - #dde3e8
      56   52 - #dde3e8
      88   52 - #9a9ea2
      96   52 - #cfd5d9
     140   52 - #040405
     148   52 - #dde3e8
     156   52 - #dde3e8
     288   52 - #dde3e8
      44   56 - #000000
      72   56 - #a1a6a9
      80   56 - #dde3e8
      88   56 - #9a9ea2
     172   56 - #020202
      56   60 - #dde3e8
      72   60 - #a1a6a9
      88   60 - #9a9ea2
     108   60 - #dde3e8
     128   60 - #dde3e8
     184   60 - #dde3e8
     236   68 - #dde3e8
     340   68 - #dde3e8
     448   68 - #dde3e8
     484   68 - #dde3e8
     536   68 - #dde3e8
     576   68 - #dde3e8
      40   88 - #597c95
     104   88 - #2f424f
      60   92 - #597c95
      84   92 - #597c95
      44  116 - #ffffff
      40  120 - #ffffff
      72  120 - #ffffff
      44  124 - #ffffff
      56  124 - #aeaeae
      72  124 - #6e6e6e
      56  128 - #aeaeae
      64  128 - #e0e0e0
     576  140 - #ffffff
     212  144 - #ffffff
      44  148 - #b7b7b7
      44  152 - #b7b7b7
      76  152 - #ffffff
     452  152 - #ffffff
      44  156 - #b7b7b7
     296  168 - #ffffff
     376  188 - #ffffff
      80  200 - #597c95
     136  200 - #597c95
     152  204 - #000000
      48  208 - #3e5768
      56  208 - #466175
      72  208 - #1b252c
     124  208 - #476276
     100  212 - #597c95
     112  212 - #010102
     136  212 - #010102
     236  220 - #ffffff
      44  240 - #ffffff
      40  244 - #ffffff
      48  244 - #b4b4b4
      60  244 - #ffffff
      44  248 - #ffffff
      48  248 - #575757
      60  248 - #ffffff
     540  264 - #ffffff
     308  268 - #ffffff
     412  268 - #ffffff
      88  292 - #486478
     108  296 - #000000
     196  296 - #1e2a33
      40  300 - #597c95
      64  300 - #000000
     120  300 - #24333d
     136  300 - #223039
     164  300 - #415a6d
     184  300 - #597c95
     208  300 - #000000
     220  300 - #344857
      56  316 - #263540
      52  320 - #202d36
      88  320 - #050708
     112  320 - #597c95
      56  324 - #263540
      72  324 - #222f38
     132  324 - #597c95
     592  380 - #597c95
     276  432 - #597c95
       4  452 - #597c95
     420  456 - #597c95
     536  484 - #597c95
     148  496 - #597c95
       4  592 - #597c95
     288  592 - #597c95
     452  592 - #597c95
     592  592 - #597c95
";
