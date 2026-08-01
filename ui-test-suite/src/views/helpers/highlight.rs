use anyhow::Result;
use test_engine::{
    refs::Weak,
    ui::{BLUE, GREEN, HighlightView, Setup, ViewTest, view},
    ui_test::check_colors,
};

#[view]
struct HighLightTestView {
    #[init]
    highlight: HighlightView,
}

impl Setup for HighLightTestView {
    fn setup(mut self: Weak<Self>) {
        self.highlight.set((200, 200), GREEN, BLUE);
    }
}

impl ViewTest for HighLightTestView {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             392    4 - #597c95
             592    4 - #597c95
             128  128 - #00ff00
             168  128 - #00ff00
             204  128 - #00ff00
             236  128 - #0000e7
             268  128 - #0000e7
             196  156 - #00ff00
             236  160 - #0000e7
             268  160 - #0000e7
             156  164 - #00ff00
             452  180 - #597c95
             236  192 - #0000e7
             136  200 - #00ff00
             268  212 - #0000e7
             160  224 - #00ff00
             192  236 - #00ff00
             128  240 - #00ff00
             240  240 - #0000e7
             268  240 - #0000e7
             160  268 - #00ff00
             212  268 - #00ff00
             240  268 - #0000e7
             268  268 - #0000e7
             300  300 - #597c95
             592  300 - #597c95
               4  392 - #597c95
             444  444 - #597c95
             180  452 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
