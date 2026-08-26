use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Anchor::Top, ImageView, Setup, ViewData, ViewTest, view},
    ui_test::helpers::check_colors,
};

// A stroked svg drawn far below its bitmap size. The bitmap is eight
// times the svg, so the small copy samples the mip chain, and its strokes
// must stay as dark as the large copy's instead of thinning to gray.
#[view]
struct ImageDownscale {
    #[init]
    large: ImageView,
    small: ImageView,
    tiny:  ImageView,
}

impl Setup for ImageDownscale {
    fn setup(self: Weak<Self>) {
        self.large.place().tl(20).size(240, 240);
        self.large.set_image("settings.svg");

        self.small.place().same_x(self.large).anchor(Top, self.large, 20).size(48, 48);
        self.small.set_image("settings.svg");

        self.tiny.place().anchor(Top, self.large, 20).l(100).size(24, 24);
        self.tiny.set_image("settings.svg");
    }
}

impl ViewTest for ImageDownscale {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             592    4 - #597c95
             144   28 - #000000
              64   60 - #000000
             220   64 - #000000
             140   92 - #476377
             416  104 - #597c95
              60  108 - #000000
             248  128 - #000000
              28  140 - #000000
              92  140 - #476377
             188  140 - #141c21
             592  180 - #597c95
             140  188 - #141c21
              60  216 - #000000
             216  216 - #000000
             152  244 - #000000
             112  284 - #000000
             112  288 - #56778f
             104  292 - #000000
             108  292 - #56778f
             112  292 - #597c95
             120  292 - #010101
             112  300 - #010101
             464  300 - #597c95
              44  304 - #597c95
              52  312 - #000001
             292  396 - #597c95
             592  416 - #597c95
             428  516 - #597c95
               4  568 - #597c95
             260  592 - #597c95
             592  592 - #597c95
            ",
        )?;
        Ok(())
    }
}
