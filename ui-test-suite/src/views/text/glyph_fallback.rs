use std::ops::Range;

use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::{Weak, manage::DataManager},
    ui::{Font, Label, RunStyle, Setup, ViewFrame, ViewTest, view},
    ui_test::{check_colors, checkpoint},
};

/// Thai chars neither Roboto nor Droid Mono cover, so without a fallback
/// they render as the notdef boxes.
const TEXT: &str = "Sa กข de";

const PLAIN_FRAME: (u32, u32, u32, u32) = (20, 20, 400, 80);
const RUN_FRAME: (u32, u32, u32, u32) = (20, 120, 400, 80);

/// The Thai span of `TEXT` in bytes.
const THAI: Range<usize> = 3..10;

#[view]
struct GlyphFallback {
    #[init]
    plain: Label,
    /// The Thai span sits inside an explicit font run whose font also
    /// misses it, so the fallback has to split the run.
    run:   Label,
}

impl Setup for GlyphFallback {
    fn setup(self: Weak<Self>) {
        self.plain.set_frame(PLAIN_FRAME);
        self.plain.set_text(TEXT).set_text_size(50);

        self.run.set_frame(RUN_FRAME);
        self.run.set_text(TEXT).set_text_size(50);
        self.run
            .set_font_runs([(0..TEXT.len(), RunStyle::font(Font::get("DroidSansMono.ttf")))]);
    }
}

impl ViewTest for GlyphFallback {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        assert_eq!(&TEXT[THAI.start..THAI.end], "กข ");

        // No fallback registered, the Thai span is notdef boxes.
        check_colors(BOXES)?;
        checkpoint("no fallback registered, the Thai span is boxes")?;

        from_main(|| {
            Font::set_fallbacks([Font::get("NotoSansThai.ttf")]);
        });

        // The Thai chars now shape and draw with the fallback font, in
        // the plain label and inside the split font run alike.
        check_colors(THAI_DRAWN)?;
        checkpoint("NotoSansThai registered as fallback, the Thai span is drawn")?;

        from_main(Font::reset_fallbacks);

        // Clearing the fallbacks brings the boxes back, the same probes
        // as the first check.
        check_colors(BOXES)?;
        checkpoint("fallbacks cleared, the boxes are back")?;

        Ok(())
    }
}

const BOXES: &str = r"
             592    4 - #597c95
             212   44 - #597c95
             240   44 - #11171c
             280   44 - #3f586a
             140   48 - #597c95
             240   52 - #11171c
             300   56 - #597c95
             240   60 - #11171c
             176   64 - #435d70
             148   68 - #597c95
             280   68 - #3f586a
             208   72 - #597c95
             240   72 - #11171c
             300   76 - #000000
             108  144 - #000001
             244  144 - #000000
             212  148 - #3f5769
             296  152 - #000001
             320  164 - #293945
             324  164 - #293945
             328  164 - #293945
             332  164 - #293945
             212  168 - #3f5769
             152  172 - #000000
             116  176 - #000000
             196  176 - #000000
             236  176 - #000000
             292  176 - #000001
             592  300 - #597c95
             300  488 - #597c95
               4  592 - #597c95
             592  592 - #597c95
            ";

const THAI_DRAWN: &str = r"
             284   40 - #000000
             144   56 - #597c95
             176   60 - #3a5162
             272   60 - #597c95
             176   64 - #3a5162
             308   64 - #000001
             176   68 - #3a5162
             196   68 - #131a20
             244   68 - #000000
             200   72 - #1e2a33
             300  144 - #223039
             108  148 - #000001
             240  152 - #10161a
             304  152 - #10161a
             156  160 - #394f5f
             212  160 - #4a677b
             244  160 - #222f38
             288  160 - #597c95
             304  164 - #10161a
             320  164 - #293945
             324  164 - #293945
             328  164 - #293945
             332  164 - #293945
             156  168 - #394f5f
             212  168 - #4a677b
             244  168 - #222f38
             212  172 - #4a677b
             116  176 - #000000
             292  176 - #000000
             300  488 - #597c95
               4  592 - #597c95
             592  592 - #597c95
            ";
