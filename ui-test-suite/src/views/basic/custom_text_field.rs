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
             236   56 - #00ffff
              60   60 - #00ffff
             136   60 - #ffff00
             176   60 - #ffff00
             212   76 - #040400
             144   84 - #c3c300
              88   88 - #010100
             208   88 - #ffff00
             144   96 - #c3c300
             160   96 - #d3d300
             184   96 - #d3d300
             188   96 - #d3d300
             248   96 - #00ffff
             124  100 - #ffff00
             152  100 - #575700
             160  100 - #575700
             180  100 - #575700
             184  100 - #575700
             188  100 - #575700
             216  108 - #ffff00
             224  108 - #010100
             128  112 - #ffff00
             144  112 - #c3c300
              88  116 - #010100
             192  116 - #000000
             212  116 - #010100
             160  124 - #010100
              64  128 - #ffff00
             108  148 - #00ffff
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
             236   56 - #00ffff
              60   60 - #00ffff
             136   60 - #bcbcbc
             176   60 - #bcbcbc
             212   76 - #030303
             144   84 - #909090
              88   88 - #010101
             208   88 - #bcbcbc
             144   96 - #909090
             160   96 - #9c9c9c
             184   96 - #9c9c9c
             188   96 - #9c9c9c
             248   96 - #00ffff
             124  100 - #bcbcbc
             152  100 - #404040
             160  100 - #404040
             180  100 - #404040
             184  100 - #404040
             188  100 - #404040
             216  108 - #bcbcbc
             224  108 - #010101
             128  112 - #bcbcbc
             144  112 - #909090
              88  116 - #010101
             192  116 - #000000
             212  116 - #010101
             160  124 - #010101
              64  128 - #bcbcbc
             108  148 - #00ffff
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
             236   56 - #00ffff
              60   60 - #00ffff
             136   60 - #ffff00
             176   60 - #ffff00
             212   76 - #040400
             144   84 - #c3c300
              88   88 - #010100
             208   88 - #ffff00
             144   96 - #c3c300
             160   96 - #d3d300
             184   96 - #d3d300
             188   96 - #d3d300
             248   96 - #00ffff
             124  100 - #ffff00
             152  100 - #575700
             160  100 - #575700
             180  100 - #575700
             184  100 - #575700
             188  100 - #575700
             216  108 - #ffff00
             224  108 - #010100
             128  112 - #ffff00
             144  112 - #c3c300
              88  116 - #010100
             192  116 - #000000
             212  116 - #010100
             160  124 - #010100
              64  128 - #ffff00
             108  148 - #00ffff
             592  204 - #597c95
             124  592 - #597c95
             592  592 - #597c95
        ",
    )
}
