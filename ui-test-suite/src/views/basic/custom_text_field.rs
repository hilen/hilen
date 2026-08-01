use anyhow::Result;
use test_engine::{
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

        // test_engine::ui_test::record_ui_test();

        Ok(())
    }
}

fn unfocused_field_colors() -> Result<()> {
    check_colors(
        r"
             460    4 - #597c95
             144   52 - #00ffff
             196   52 - #00ffff
              60   60 - #00ffff
              96   64 - #ffff00
             228   68 - #ffff00
             148   80 - #000000
             176   80 - #000000
             196   80 - #000000
             216   84 - #ffff00
             120   88 - #000000
             228   92 - #ffff00
              80   96 - #000000
             116   96 - #ffff00
             124   96 - #ffff00
             140  104 - #000000
             220  104 - #ffff00
             128  108 - #010100
             208  108 - #000000
             120  112 - #010100
             216  112 - #000000
             168  116 - #ffff00
              68  132 - #ffff00
             136  136 - #ffff00
             196  136 - #ffff00
             236  140 - #00ffff
             104  148 - #00ffff
             160  148 - #00ffff
             592  204 - #597c95
             300  456 - #597c95
               4  592 - #597c95
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
             460    4 - #597c95
             144   52 - #00ffff
             196   52 - #00ffff
              60   60 - #00ffff
              96   64 - #bcbcbc
             228   68 - #bcbcbc
             148   80 - #000000
             176   80 - #000000
             196   80 - #000000
             216   84 - #bcbcbc
             120   88 - #000000
             228   92 - #bcbcbc
              80   96 - #000000
             116   96 - #bcbcbc
             124   96 - #bcbcbc
             140  104 - #000000
             220  104 - #bcbcbc
             128  108 - #010101
             208  108 - #000000
             120  112 - #010101
             216  112 - #000000
             168  116 - #bcbcbc
              68  132 - #bcbcbc
             136  136 - #bcbcbc
             196  136 - #bcbcbc
             236  140 - #00ffff
             104  148 - #00ffff
             160  148 - #00ffff
             592  204 - #597c95
             300  456 - #597c95
               4  592 - #597c95
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
             460    4 - #597c95
             144   52 - #00ffff
             196   52 - #00ffff
              60   60 - #00ffff
              96   64 - #ffff00
             228   68 - #ffff00
             148   80 - #000000
             176   80 - #000000
             196   80 - #000000
             216   84 - #ffff00
             120   88 - #000000
             228   92 - #ffff00
              80   96 - #000000
             116   96 - #ffff00
             124   96 - #ffff00
             140  104 - #000000
             220  104 - #ffff00
             128  108 - #010100
             208  108 - #000000
             120  112 - #010100
             216  112 - #000000
             168  116 - #ffff00
              68  132 - #ffff00
             136  136 - #ffff00
             196  136 - #ffff00
             236  140 - #00ffff
             104  148 - #00ffff
             160  148 - #00ffff
             592  204 - #597c95
             300  456 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ",
    )
}
