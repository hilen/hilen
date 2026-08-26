use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Setup, TextField, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_keys, inject_named_key, set_record_probe_count},
    window::NamedKey,
};

/// Five bullets sit centered in the field, the pinned black pixels are
/// their ink, nothing of the real text is drawn.
const CHECK_1: &str = r"
             476    4 - #597c95
             592    4 - #597c95
              40   20 - #bcbcbc
              76   20 - #bcbcbc
             100   20 - #bcbcbc
             128   20 - #bcbcbc
             152   20 - #bcbcbc
             184   20 - #bcbcbc
             216   20 - #bcbcbc
             252   20 - #bcbcbc
             284   20 - #bcbcbc
             336   20 - #bcbcbc
             372   20 - #bcbcbc
             396   20 - #bcbcbc
              20   24 - #bcbcbc
             416   24 - #bcbcbc
              56   28 - #bcbcbc
             168   28 - #bcbcbc
             200   28 - #bcbcbc
             236   28 - #bcbcbc
             312   32 - #bcbcbc
             116   36 - #bcbcbc
             380   36 - #bcbcbc
              68   40 - #bcbcbc
             180   40 - #bcbcbc
             268   40 - #bcbcbc
             336   40 - #bcbcbc
             360   40 - #bcbcbc
              20   44 - #bcbcbc
              44   44 - #bcbcbc
             136   44 - #bcbcbc
             416   44 - #bcbcbc
              88   48 - #bcbcbc
             164   48 - #bcbcbc
             196   48 - #000000
             200   48 - #000000
             208   48 - #000000
             220   48 - #000000
             232   48 - #000000
             240   48 - #000000
             292   48 - #bcbcbc
             388   52 - #bcbcbc
             108   56 - #bcbcbc
             260   56 - #bcbcbc
             328   56 - #bcbcbc
             348   56 - #bcbcbc
             368   56 - #bcbcbc
             512   56 - #597c95
              28   60 - #bcbcbc
             128   60 - #bcbcbc
             180   60 - #bcbcbc
             308   60 - #bcbcbc
              48   64 - #bcbcbc
             152   64 - #bcbcbc
             280   64 - #bcbcbc
             248   68 - #bcbcbc
             400   68 - #bcbcbc
              76   72 - #bcbcbc
             360   72 - #bcbcbc
              20   76 - #bcbcbc
             100   76 - #bcbcbc
             136   76 - #bcbcbc
             172   76 - #bcbcbc
             200   76 - #bcbcbc
             228   76 - #bcbcbc
             264   76 - #bcbcbc
             300   76 - #bcbcbc
             336   76 - #bcbcbc
             384   76 - #bcbcbc
             416   76 - #bcbcbc
             592   84 - #597c95
             472  104 - #597c95
              64  136 - #597c95
             276  144 - #597c95
             548  148 - #597c95
               4  152 - #597c95
             124  160 - #597c95
             384  164 - #597c95
             208  168 - #597c95
             468  168 - #597c95
             300  208 - #597c95
              56  212 - #597c95
             592  212 - #597c95
             236  224 - #597c95
             360  232 - #597c95
             516  236 - #597c95
             424  248 - #597c95
               4  264 - #597c95
             184  264 - #597c95
             108  268 - #597c95
             260  284 - #597c95
             592  292 - #597c95
             340  296 - #597c95
              56  304 - #597c95
             412  320 - #597c95
             176  328 - #597c95
             492  332 - #597c95
               4  340 - #597c95
             292  340 - #597c95
             112  344 - #597c95
             232  360 - #597c95
             584  388 - #597c95
             332  392 - #597c95
             424  392 - #597c95
              60  412 - #597c95
             268  416 - #597c95
             160  440 - #597c95
             500  440 - #597c95
             388  448 - #597c95
             320  452 - #597c95
               4  476 - #597c95
             252  484 - #597c95
             592  488 - #597c95
             192  492 - #597c95
             436  496 - #597c95
              92  500 - #597c95
             352  504 - #597c95
             296  532 - #597c95
             512  532 - #597c95
             236  544 - #597c95
             176  552 - #597c95
               4  556 - #597c95
             356  584 - #597c95
             436  584 - #597c95
              72  592 - #597c95
             280  592 - #597c95
             528  592 - #597c95
             592  592 - #597c95
";

/// A secure field is a password box. `text` returns what was typed, the
/// label draws one bullet per character, and the caret keeps moving by
/// characters of the real text even though the bullets are wider bytes.
#[view]
struct SecureTextField {
    #[init]
    field: TextField,
}

impl Setup for SecureTextField {
    fn setup(self: Weak<Self>) {
        self.field.set_secure(true);
        self.field.set_text_size(32);
        self.field.place().tl(20).size(400, 60);
    }
}

impl ViewTest for SecureTextField {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);
        from_main(move || view.field.focus());
        inject_keys("abc");
        assert_eq!(from_main(move || view.field.text().to_string()), "abc");

        // Editing works in real text bytes. Two characters back from the
        // end lands between a and b, not somewhere inside a bullet.
        inject_named_key(NamedKey::ArrowLeft);
        inject_named_key(NamedKey::ArrowLeft);
        inject_keys("x\u{e9}");
        assert_eq!(from_main(move || view.field.text().to_string()), "ax\u{e9}bc");
        inject_keys("\u{8}");
        assert_eq!(from_main(move || view.field.text().to_string()), "axbc");
        inject_named_key(NamedKey::End);
        inject_keys("d");
        assert_eq!(from_main(move || view.field.text().to_string()), "axbcd");

        // Turning secure mode off shows the real text, on hides it again.
        from_main(move || {
            view.field.set_secure(false);
        });
        assert_eq!(from_main(move || view.field.text().to_string()), "axbcd");
        from_main(move || {
            view.field.set_secure(true);
        });
        assert_eq!(from_main(move || view.field.text().to_string()), "axbcd");
        assert!(from_main(move || view.field.is_secure()));

        check_colors(CHECK_1)?;

        Ok(())
    }
}
