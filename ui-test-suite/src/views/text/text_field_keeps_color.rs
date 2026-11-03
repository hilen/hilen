use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Color, DynamicColor, Label, Setup, TextAlignment, TextField, Theme, ThemeMode, ViewData, ViewTest,
        view,
    },
    ui_test::{
        check_colors,
        system_input::{tap, wait_until},
    },
};

const OWN: DynamicColor = DynamicColor::new(Color::hex("#e8f1ea"), Color::hex("#24342a"));
const CHOSEN: DynamicColor = DynamicColor::new(Color::hex("#fff2c4"), Color::hex("#4a3f1a"));

const PLAIN: (f32, f32) = (200.0, 115.0);
const CHOSEN_AT: (f32, f32) = (200.0, 235.0);
const OUTSIDE: (f32, f32) = (300.0, 450.0);

const PLAIN_EDITED: &str = r"
             172   68 - #000001
             136   72 - #597c95
              40   76 - #597c95
             100   76 - #10171b
             156   76 - #395060
             576   96 - #e8f1ea
             276  104 - #747975
             276  116 - #747975
             288  116 - #e8f1ea
             304  116 - #e8f1ea
             320  120 - #000000
             276  124 - #747975
             452  132 - #e8f1ea
              72  188 - #11181d
             152  188 - #334755
             108  192 - #2e414e
             144  192 - #23313a
             208  192 - #000000
              40  196 - #597c95
             144  196 - #23313a
             228  196 - #1e2a32
             240  200 - #597c95
             280  232 - #545754
             288  236 - #e8f1ea
             320  236 - #000000
             308  240 - #262827
             316  240 - #e8f1ea
             488  256 - #e8f1ea
               4  404 - #597c95
             340  504 - #597c95
              92  592 - #597c95
             592  592 - #597c95
";
const CHOSEN_EDITED: &str = r"
             172   68 - #000001
             136   72 - #597c95
              40   76 - #597c95
             100   76 - #10171b
             156   76 - #395060
             388   92 - #e8f1ea
             576   96 - #e8f1ea
             284  112 - #e8f1ea
             288  116 - #e8f1ea
             304  116 - #e8f1ea
             320  120 - #000000
             452  132 - #e8f1ea
             208  188 - #151d23
             108  192 - #2e414e
             144  192 - #23313a
              40  196 - #597c95
             120  196 - #000000
             144  196 - #23313a
             224  196 - #354a59
             576  216 - #fff2c4
             276  224 - #807962
             280  232 - #5c5747
             284  232 - #fff2c4
             304  236 - #fff2c4
             280  240 - #5c5747
             308  240 - #2a2820
             316  240 - #fff2c4
             276  244 - #807962
             488  256 - #fff2c4
             340  504 - #597c95
              92  592 - #597c95
             592  592 - #597c95
";

/// A field keeps the fill the app gave it while it is typed in. Only a
/// field with `set_selected_color` takes another fill then.
#[view]
struct TextFieldKeepsColor {
    #[init]
    plain_title:  Label,
    plain:        TextField,
    chosen_title: Label,
    chosen:       TextField,
}

impl Setup for TextFieldKeepsColor {
    fn setup(self: Weak<Self>) {
        let title = |label: Weak<Label>, y: f32, text: &str| {
            label.set_text_size(18).set_alignment(TextAlignment::Left);
            label.set_text(text);
            label.place().t(y).lr(20).h(26);
        };
        title(self.plain_title, 60.0, "a field in its own fill");
        title(self.chosen_title, 180.0, "a field with a fill for editing");

        for (field, y) in [(self.plain, 90.0), (self.chosen, 210.0)] {
            field.set_color(OWN).set_corner_radius(8);
            field.set_text_size(24).set_text("Riga");
            field.place().t(y).lr(20).h(50);
        }
        self.chosen.set_selected_color(CHOSEN);
    }
}

impl ViewTest for TextFieldKeepsColor {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(|| {
            Theme::set_mode(ThemeMode::System);
            Theme::set_system(Theme::Light);
        });

        tap(PLAIN.0, PLAIN.1)?;
        wait_until("the plain field is edited", move || view.plain.is_editing())?;
        wait_for_next_frame();
        from_main(move || {
            ensure!(
                *view.plain.color() == OWN.resolve(),
                "the field took another fill while edited: {:?}",
                view.plain.color()
            );
            Ok(())
        })?;
        check_colors(PLAIN_EDITED)?;

        tap(CHOSEN_AT.0, CHOSEN_AT.1)?;
        wait_until("the chosen field is edited", move || view.chosen.is_editing())?;
        wait_for_next_frame();
        from_main(move || {
            ensure!(
                *view.chosen.color() == CHOSEN.resolve(),
                "the field did not take its fill for editing: {:?}",
                view.chosen.color()
            );
            ensure!(
                *view.plain.color() == OWN.resolve(),
                "the plain field lost its fill after editing: {:?}",
                view.plain.color()
            );
            Ok(())
        })?;
        check_colors(CHOSEN_EDITED)?;

        // A tap on the empty ground ends the editing for the tests after this
        // one.
        tap(OUTSIDE.0, OUTSIDE.1)?;
        Ok(())
    }
}
