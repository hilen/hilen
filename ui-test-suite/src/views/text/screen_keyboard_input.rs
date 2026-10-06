use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, TextAlignment, TextField, ViewData, ViewTest, view},
    ui_test::{
        check_colors, checkpoint, set_record_probe_count,
        system_input::{system_input, system_request, tap, type_text, wait_until},
    },
};

// Right of the text, so the caret of a desktop field lands at its end, where
// the system field of a phone puts it by itself.
const NAME: (f32, f32) = (540.0, 115.0);
const CITY: (f32, f32) = (540.0, 215.0);
const AGE: (f32, f32) = (540.0, 315.0);
const OUTSIDE: (f32, f32) = (300.0, 540.0);

/// Typing with the keyboard the platform really has. On a phone every tap
/// and every key goes through the system, the screen keyboard included, see
/// `system_input`. Each field must keep its own text through the system
/// text field that all of them share there.
#[view]
struct ScreenKeyboardInput {
    ended: Vec<String>,

    #[init]
    name_title:    Label,
    name:          TextField,
    city_title:    Label,
    city:          TextField,
    age_title:     Label,
    age:           TextField,
    log:           Label,
    outside_title: Label,
}

impl Setup for ScreenKeyboardInput {
    fn setup(self: Weak<Self>) {
        let title = |label: Weak<Label>, text: &str, y: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };
        let field = |field: Weak<TextField>, y: f32| {
            field.set_text_size(24);
            field.place().t(y).lr(20).h(50);
            field.editing_ended.val(move |text| self.note_ended(text));
        };

        title(self.name_title, "name, starts empty", 60.0);
        field(self.name, 90.0);

        title(self.city_title, "city, starts with Riga", 160.0);
        field(self.city, 190.0);
        self.city.set_text("Riga");

        title(self.age_title, "age, digits only", 260.0);
        field(self.age, 290.0);
        self.age.integer_only();

        self.log.set_text_size(18).set_multiline(true);
        self.log.set_alignment(TextAlignment::Left);
        self.log.place().t(360).lr(20).h(130);

        title(self.outside_title, "a tap down here ends the editing", 527.0);
    }
}

impl ScreenKeyboardInput {
    fn note_ended(mut self: Weak<Self>, text: String) {
        self.ended.push(text);
        let lines: Vec<String> = self.ended.iter().map(|text| format!("editing ended: {text}")).collect();
        self.log.set_text(lines.join("\n"));
    }
}

fn tap_at(point: (f32, f32)) -> Result<()> {
    tap(point.0, point.1)
}

impl ViewTest for ScreenKeyboardInput {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        tap_at(NAME)?;
        wait_until("name is edited", move || view.name.is_editing())?;
        type_text("Hello")?;
        tap_at(OUTSIDE)?;
        wait_until("name is not edited any more", move || !view.name.is_editing())?;
        checkpoint("name holds Hello, the log says it ended with Hello")?;
        from_main(move || {
            ensure!(view.name.text() == "Hello", "name holds '{}'", view.name.text());
            ensure!(view.ended == ["Hello"], "ended with {:?}", view.ended);
            Ok(())
        })?;

        // The field that was edited before must leave nothing behind.
        tap_at(CITY)?;
        wait_until("city is edited", move || view.city.is_editing())?;
        tap_at(OUTSIDE)?;
        wait_until("city is not edited any more", move || !view.city.is_editing())?;
        checkpoint("city still holds Riga, nothing was typed")?;
        from_main(move || {
            ensure!(view.city.text() == "Riga", "city holds '{}'", view.city.text());
            Ok(())
        })?;

        // Not the same point again, 2 taps on one point are a double click
        // on desktop and select the word.
        tap(CITY.0 - 20.0, CITY.1)?;
        wait_until("city is edited again", move || view.city.is_editing())?;
        type_text(" east")?;
        wait_until("city holds the typed text while it is edited", move || {
            view.city.text() == "Riga east"
        })?;
        checkpoint("city is edited and holds Riga east")?;

        // Straight from one field into the next, with no tap outside.
        tap(NAME.0 - 20.0, NAME.1)?;
        wait_until("name is edited, city is not", move || {
            view.name.is_editing() && !view.city.is_editing()
        })?;
        type_text(" you")?;
        wait_until("name holds Hello you", move || view.name.text() == "Hello you")?;
        tap_at(OUTSIDE)?;
        wait_until("name is not edited any more", move || !view.name.is_editing())?;
        checkpoint("name holds Hello you, city holds Riga east")?;
        from_main(move || {
            ensure!(
                view.city.text() == "Riga east",
                "city holds '{}'",
                view.city.text()
            );
            ensure!(
                view.name.text() == "Hello you",
                "name holds '{}'",
                view.name.text()
            );
            Ok(())
        })?;

        tap_at(AGE)?;
        wait_until("age is edited", move || view.age.is_editing())?;
        if system_input() {
            // A digits only field asks for the number pad, which has no
            // letters to tap at all.
            system_request("keyboard 1")?;
            let letter = system_request("has key a")?;
            ensure!(letter == "0", "the keyboard of a digits only field has letters");
        }
        type_text("42")?;
        tap_at(OUTSIDE)?;
        wait_until("age is not edited any more", move || !view.age.is_editing())?;
        if system_input() {
            system_request("keyboard 0")?;
        }
        from_main(move || {
            ensure!(view.age.text() == "42", "age holds '{}'", view.age.text());
            ensure!(
                view.ended == ["Hello", "Riga", "Riga east", "Hello you", "42"],
                "ended with {:?}",
                view.ended
            );
            Ok(())
        })?;

        check_colors(FINAL)
    }
}

const FINAL: &str = r"
             240    4 - #597c95
             392    4 - #597c95
             512    4 - #597c95
              76   72 - #597c95
              44   76 - #19232a
             116   76 - #0c1114
             152   76 - #1f2b33
             168   80 - #304350
             568   92 - #ffffff
             252  108 - #626262
             264  108 - #000000
             284  108 - #cbcbcb
             288  108 - #000000
             284  112 - #cbcbcb
             340  112 - #d3d3d3
             284  116 - #cbcbcb
             296  116 - #ffffff
             300  116 - #ffffff
             328  116 - #ffffff
             340  116 - #d3d3d3
             252  120 - #626262
             264  120 - #000000
             284  120 - #cbcbcb
             288  120 - #000000
             468  124 - #ffffff
             148  168 - #000000
              40  176 - #597c95
              88  176 - #000001
             180  180 - #597c95
             576  196 - #ffffff
             252  208 - #626262
             256  208 - #ffffff
             256  212 - #ffffff
             252  216 - #626262
             256  216 - #b7b7b7
             276  216 - #ffffff
             280  216 - #c4c4c4
             308  216 - #010101
             340  216 - #ffffff
             344  216 - #212121
             252  220 - #626262
             448  232 - #ffffff
             520  236 - #ffffff
              80  268 - #425b6e
             104  272 - #1d2830
              40  276 - #597c95
             144  276 - #000000
              64  280 - #597c95
             388  296 - #ffffff
             216  304 - #ffffff
             296  308 - #000000
             576  308 - #ffffff
             296  312 - #000000
             304  312 - #ffffff
             292  316 - #ffffff
             296  320 - #000000
             468  336 - #ffffff
             176  380 - #456073
              40  384 - #1b252c
              84  384 - #597c95
             128  384 - #1b252c
             156  400 - #49667a
              40  404 - #2b3b47
             100  404 - #2b3b47
             176  404 - #000000
              68  408 - #000000
             156  408 - #49667a
             316  408 - #597c95
             224  424 - #050708
             112  428 - #000001
             156  428 - #49667a
              52  440 - #425b6e
             132  440 - #597c95
             176  440 - #456073
             224  444 - #324654
             156  448 - #49667a
             176  448 - #456073
              76  452 - #597c95
             392  460 - #597c95
             592  460 - #597c95
             160  464 - #07090b
              52  468 - #425b6e
              96  468 - #000000
             128  468 - #000000
             492  492 - #597c95
             196  536 - #273641
              52  540 - #0b0f12
              96  540 - #597c95
             144  540 - #334756
             172  540 - #334756
             216  540 - #597c95
             264  540 - #476377
             272  544 - #456073
             356  592 - #597c95
             456  592 - #597c95
             588  592 - #597c95
";
