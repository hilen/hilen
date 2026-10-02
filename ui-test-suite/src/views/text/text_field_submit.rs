use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Label, NamedKey, Setup, TextAlignment, TextField, ViewData, ViewTest, ViewTouch, view},
    ui_test::{check_colors, inject_keys, inject_named_key},
};

/// Enter in a single line field fires `submitted` with the text, after the
/// editing ended. Escape and a multiline field never fire it, and a secure
/// field hands over bullets, never the password.
#[view]
struct TextFieldSubmit {
    submits: Vec<String>,

    #[init]
    name_title:     Label,
    name:           TextField,
    password_title: Label,
    password:       TextField,
    notes_title:    Label,
    notes:          TextField,
    log:            Label,
}

impl Setup for TextFieldSubmit {
    fn setup(self: Weak<Self>) {
        let title = |label: Weak<Label>, text: &str, y: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        title(self.name_title, "single line field, Enter submits", 20.0);
        self.name.set_text_size(20);
        self.name.place().t(50).lr(20).h(44);

        title(self.password_title, "secure field, submits bullets", 120.0);
        self.password.set_text_size(20).set_secure(true);
        self.password.place().t(150).lr(20).h(44);

        title(self.notes_title, "multiline field, Enter is a new line", 220.0);
        self.notes.set_text_size(20).set_multiline(true);
        self.notes.place().t(250).lr(20).h(110);

        self.log.set_text_size(18).set_multiline(true);
        self.log.set_alignment(TextAlignment::Left);
        self.log.place().t(390).lr(20).h(180);

        for field in [self.name, self.password, self.notes] {
            field.submitted.val(move |text| self.submitted(text));
        }
        self.show_log();
    }
}

impl TextFieldSubmit {
    fn submitted(mut self: Weak<Self>, text: String) {
        self.submits.push(text);
        self.show_log();
    }

    fn show_log(self: Weak<Self>) {
        self.log.set_text(format!(
            "submitted {} times: {}",
            self.submits.len(),
            self.submits.join(", ")
        ));
    }

    fn submits(self: Weak<Self>) -> Vec<String> {
        from_main(move || self.submits.clone())
    }
}

impl ViewTest for TextFieldSubmit {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || view.name.focus());
        inject_keys("Ann");
        inject_named_key(NamedKey::Escape);
        wait_for_next_frame();
        ensure!(
            view.submits().is_empty(),
            "Escape submitted: {:?}",
            view.submits()
        );
        // Ann typed, Escape ended the editing, nothing submitted
        check_colors(CHECK_1)?;

        from_main(move || view.name.focus());
        inject_named_key(NamedKey::Enter);
        wait_for_next_frame();
        ensure!(
            view.submits() == ["Ann"],
            "Enter in the name field: {:?}",
            view.submits()
        );
        ensure!(
            from_main(move || !view.name.is_selected()),
            "Enter did not end the editing"
        );
        // Enter in the first field, the log shows 1 time: Ann
        check_colors(CHECK_2)?;

        from_main(move || view.password.focus());
        inject_keys("key");
        inject_named_key(NamedKey::Enter);
        wait_for_next_frame();
        let bullets = "\u{2022}".repeat(3);
        ensure!(
            view.submits() == ["Ann".to_string(), bullets],
            "Enter in the secure field: {:?}",
            view.submits()
        );
        ensure!(
            from_main(move || view.password.text() == "key"),
            "the secure field lost its text"
        );
        // Enter in the secure field, the log shows 2 times, the second is 3
        // bullets
        check_colors(CHECK_3)?;

        from_main(move || view.notes.focus());
        inject_keys("one");
        inject_named_key(NamedKey::Enter);
        inject_keys("two");
        wait_for_next_frame();
        ensure!(
            view.submits().len() == 2,
            "the multiline field submitted: {:?}",
            view.submits()
        );
        ensure!(
            from_main(move || view.notes.text() == "one\ntwo"),
            "Enter in the multiline field did not add a line"
        );
        // Enter in the multiline field made a second line, the log still shows
        // 2 times
        check_colors(CHECK_4)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
     592    4 - #597c95
      68   32 - #2f414e
     184   32 - #49667a
     260   32 - #1d2830
      52   36 - #273641
     124   36 - #476276
     456   52 - #ffffff
     288   72 - #ffffff
     296   72 - #e0e0e0
     296   76 - #e0e0e0
     316   76 - #d0d0d0
     232  128 - #1b252c
      96  132 - #000000
     148  132 - #3c5364
      40  136 - #597c95
     196  136 - #597c95
     432  192 - #ffffff
     152  228 - #344958
     268  228 - #2a3b47
      60  232 - #2d3f4c
     220  232 - #597c95
     116  236 - #020304
     188  236 - #000001
     268  236 - #2a3b47
     576  268 - #ffffff
     408  356 - #ffffff
     556  432 - #597c95
      60  480 - #597c95
     108  480 - #000001
     160  484 - #0c1114
     364  588 - #597c95
     592  592 - #597c95
";

const CHECK_2: &str = r"
      68   32 - #2f414e
     104   32 - #000001
     168   32 - #000000
     260   32 - #1d2830
      52   36 - #273641
     444   52 - #ffffff
     576   52 - #ffffff
     288   72 - #ffffff
     296   72 - #e0e0e0
     296   76 - #e0e0e0
     316   76 - #d0d0d0
     108  132 - #597c95
     216  132 - #364b5b
      40  136 - #597c95
     168  136 - #3a5060
     460  192 - #ffffff
     152  228 - #344958
     268  228 - #2a3b47
      60  232 - #2d3f4c
     112  236 - #2c3d49
     188  236 - #000001
     232  236 - #273641
     284  236 - #223039
      20  356 - #ffffff
     592  364 - #597c95
     396  400 - #597c95
      60  480 - #597c95
      96  480 - #597c95
     144  480 - #090c0f
     212  484 - #395060
     592  520 - #597c95
     456  592 - #597c95
";

const CHECK_3: &str = r"
      68   32 - #2f414e
     104   32 - #000001
     168   32 - #000000
     260   32 - #1d2830
      52   36 - #273641
     444   52 - #ffffff
     576   52 - #ffffff
     288   72 - #ffffff
     296   72 - #e0e0e0
     296   76 - #e0e0e0
     316   76 - #d0d0d0
     108  132 - #597c95
     216  132 - #364b5b
      40  136 - #597c95
     168  136 - #3a5060
     460  192 - #ffffff
     152  228 - #344958
     268  228 - #2a3b47
      60  232 - #2d3f4c
     112  236 - #2c3d49
     188  236 - #000001
     232  236 - #273641
     284  236 - #223039
      20  356 - #ffffff
     592  364 - #597c95
     396  400 - #597c95
      60  480 - #597c95
      96  480 - #597c95
     144  480 - #090c0f
     212  484 - #395060
     592  520 - #597c95
     456  592 - #597c95
";

const CHECK_4: &str = r"
     168   32 - #000000
     260   32 - #1d2830
      52   36 - #273641
     124   36 - #476276
     524   52 - #ffffff
     420   64 - #ffffff
     288   72 - #ffffff
     296   72 - #e0e0e0
     296   76 - #e0e0e0
     316   76 - #d0d0d0
     108  132 - #597c95
      40  136 - #597c95
     168  136 - #3a5060
     232  136 - #1b252c
     432  184 - #ffffff
     268  228 - #2a3b47
      60  232 - #2d3f4c
     112  236 - #2c3d49
     188  236 - #000001
     576  252 - #bcbcbc
     288  272 - #bcbcbc
     316  284 - #5e5e5e
     288  292 - #bcbcbc
     316  300 - #5e5e5e
     460  340 - #bcbcbc
      20  356 - #bcbcbc
     592  432 - #597c95
      60  480 - #597c95
     144  480 - #090c0f
     420  480 - #597c95
     212  484 - #395060
     592  592 - #597c95
";
