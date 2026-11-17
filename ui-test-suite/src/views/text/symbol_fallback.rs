use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Label, Setup, ViewFrame, ViewTest, view},
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

/// Roboto has none of these chars. With no fallback of the app and no
/// system fonts, which the runner turns off as a browser has none, they
/// used to draw as empty boxes. The bundled symbol fonts draw them.
#[view]
struct SymbolFallback {
    #[init]
    stars:  Label,
    arrows: Label,
    shapes: Label,
}

impl Setup for SymbolFallback {
    fn setup(self: Weak<Self>) {
        self.stars.set_frame((20, 20, 560, 90));
        self.stars.set_text("Rated ★★★☆☆ ✓ ✗").set_text_size(44);

        self.arrows.set_frame((20, 130, 560, 90));
        self.arrows.set_text("Keys ← ↑ → ↓ ⌘ ⏎").set_text_size(44);

        self.shapes.set_frame((20, 240, 560, 90));
        self.shapes.set_text("Status ● ○ ▲ ⚠ ♥").set_text_size(44);
    }
}

impl ViewTest for SymbolFallback {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        check_colors(DRAWN)?;
        checkpoint("stars, checks, arrows, keys and shapes drawn, no empty boxes")?;
        Ok(())
    }
}

const DRAWN: &str = r"
             404   48 - #000000
              96   52 - #4f6e85
              88   56 - #415b6d
             508   56 - #000000
             276   60 - #000001
             364   60 - #0d1215
              88   64 - #415b6d
             144   64 - #222f39
             188   68 - #597c95
             140   72 - #3a5161
             164   72 - #597c95
             320   72 - #000001
              88   76 - #415b6d
             444   76 - #000000
             288  168 - #1d2830
             420  168 - #435d70
             476  172 - #597c95
             156  176 - #161f25
             160  176 - #161f25
             236  176 - #10171b
             260  176 - #10171b
             316  176 - #10171b
             328  176 - #10171b
             344  176 - #10171b
             420  176 - #344856
             288  180 - #1d2830
             288  184 - #1d2830
             196  188 - #000001
             288  188 - #1d2830
             288  192 - #1d2830
             480  272 - #000001
             124  276 - #597c95
             180  276 - #435e71
             188  276 - #435e71
             212  276 - #435e71
             368  276 - #000000
             144  284 - #283843
             336  284 - #010202
             144  288 - #283843
             304  288 - #000000
             148  292 - #344856
             220  292 - #4a677b
             432  296 - #597c95
             592  400 - #597c95
             144  472 - #597c95
             296  580 - #597c95
               4  592 - #597c95
             592  592 - #597c95
            ";
