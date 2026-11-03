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
              40   20 - #ffffff
              76   20 - #ffffff
             100   20 - #ffffff
             128   20 - #ffffff
             152   20 - #ffffff
             184   20 - #ffffff
             216   20 - #ffffff
             252   20 - #ffffff
             284   20 - #ffffff
             336   20 - #ffffff
             372   20 - #ffffff
             396   20 - #ffffff
              20   24 - #ffffff
             416   24 - #ffffff
              56   28 - #ffffff
             168   28 - #ffffff
             200   28 - #ffffff
             236   28 - #ffffff
             312   32 - #ffffff
             116   36 - #ffffff
             380   36 - #ffffff
              68   40 - #ffffff
             180   40 - #ffffff
             268   40 - #ffffff
             336   40 - #ffffff
             360   40 - #ffffff
              20   44 - #ffffff
              44   44 - #ffffff
             136   44 - #ffffff
             416   44 - #ffffff
              88   48 - #ffffff
             164   48 - #ffffff
             196   48 - #010101
             200   48 - #010101
             208   48 - #000000
             220   48 - #000000
             232   48 - #000000
             240   48 - #000000
             292   48 - #ffffff
             388   52 - #ffffff
             108   56 - #ffffff
             260   56 - #ffffff
             328   56 - #ffffff
             348   56 - #ffffff
             368   56 - #ffffff
             512   56 - #597c95
              28   60 - #ffffff
             128   60 - #ffffff
             180   60 - #ffffff
             308   60 - #ffffff
              48   64 - #ffffff
             152   64 - #ffffff
             280   64 - #ffffff
             248   68 - #ffffff
             400   68 - #ffffff
              76   72 - #ffffff
             360   72 - #ffffff
              20   76 - #ffffff
             100   76 - #ffffff
             136   76 - #ffffff
             172   76 - #ffffff
             200   76 - #ffffff
             228   76 - #ffffff
             264   76 - #ffffff
             300   76 - #ffffff
             336   76 - #ffffff
             384   76 - #ffffff
             416   76 - #ffffff
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
