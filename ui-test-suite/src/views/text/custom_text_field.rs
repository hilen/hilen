use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Setup, TURQUOISE, TextField, ViewData, ViewFrame, ViewTest, YELLOW, view},
    ui_test::{check_colors, inject_touches},
};

#[view]
struct CustomTextField {
    #[init]
    field: TextField,
}

impl Setup for CustomTextField {
    fn setup(self: Weak<Self>) {
        self.field
            .set_text("1.eĘEŠ")
            .set_color(YELLOW)
            .set_border_color(TURQUOISE)
            .set_text_size(50)
            .set_corner_radius(28)
            .set_border_width(10);
        self.field.set_frame((50, 50, 200, 100));
    }
}

impl ViewTest for CustomTextField {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        unfocused_field_colors()?;
        tap_focuses_field()?;
        tap_outside_unfocuses_field()?;

        // hilen::ui_test::record_ui_test();

        Ok(())
    }
}

fn unfocused_field_colors() -> Result<()> {
    check_colors(
        r"
             112   52 - #00ffff
             236   56 - #00ffff
              60   60 - #00ffff
             176   60 - #ffff00
             212   76 - #050500
             144   84 - #baba00
              88   88 - #000000
             208   88 - #ffff00
             144   96 - #baba00
             156   96 - #d6d600
             184   96 - #d6d600
             188   96 - #d6d600
             124  100 - #ffff00
             152  100 - #555500
             160  100 - #555500
             180  100 - #555500
             184  100 - #555500
             188  100 - #555500
             216  100 - #010100
             216  108 - #ffff00
             128  112 - #ffff00
             144  112 - #baba00
              88  116 - #000000
             192  116 - #000000
             216  116 - #010100
             160  124 - #010100
              64  128 - #ffff00
             104  136 - #ffff00
             172  148 - #00ffff
             592  204 - #597c95
             124  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn tap_focuses_field() -> Result<()> {
    inject_touches(
        "
          193  123  b
          193  123  e
      ",
    );

    check_colors(
        r"
             112   52 - #00ffff
             236   56 - #00ffff
              60   60 - #00ffff
             176   60 - #ffff00
             212   76 - #050500
             144   84 - #baba00
             176   84 - #000000
              88   88 - #000000
             208   88 - #ffff00
             144   96 - #baba00
             156   96 - #d6d600
             184   96 - #d6d600
             188   96 - #d6d600
             124  100 - #ffff00
             152  100 - #555500
             160  100 - #555500
             180  100 - #555500
             184  100 - #555500
             188  100 - #555500
             208  104 - #ffff00
             144  112 - #baba00
             188  112 - #ffff00
              88  116 - #000000
             128  116 - #010100
             216  116 - #010100
             160  124 - #010100
              64  128 - #ffff00
             104  136 - #ffff00
             172  148 - #00ffff
             592  204 - #597c95
             124  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn tap_outside_unfocuses_field() -> Result<()> {
    inject_touches(
        "
        43   192  b
        43   192  e
    ",
    );

    check_colors(
        r"
             112   52 - #00ffff
             236   56 - #00ffff
              60   60 - #00ffff
             176   60 - #ffff00
             212   76 - #050500
             144   84 - #baba00
              88   88 - #000000
             208   88 - #ffff00
             144   96 - #baba00
             156   96 - #d6d600
             184   96 - #d6d600
             188   96 - #d6d600
             124  100 - #ffff00
             152  100 - #555500
             160  100 - #555500
             180  100 - #555500
             184  100 - #555500
             188  100 - #555500
             216  100 - #010100
             216  108 - #ffff00
             128  112 - #ffff00
             144  112 - #baba00
              88  116 - #000000
             192  116 - #000000
             216  116 - #010100
             160  124 - #010100
              64  128 - #ffff00
             104  136 - #ffff00
             172  148 - #00ffff
             592  204 - #597c95
             124  592 - #597c95
             592  592 - #597c95
        ",
    )
}
