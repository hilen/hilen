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
             396    4 - #597c95
             592    4 - #597c95
             172   60 - #000000
              56   68 - #000000
             116   96 - #000000
              36  124 - #000000
             252  140 - #2c3e4a
             188  156 - #000000
              64  176 - #000000
             128  188 - #000000
             216  216 - #000000
             140  252 - #2c3e4a
             112  284 - #000000
             104  288 - #000000
             112  288 - #597c95
             104  292 - #000000
             108  292 - #597c95
             112  292 - #597c95
             120  292 - #000000
              52  296 - #000000
             116  296 - #000000
             112  300 - #000000
             512  300 - #597c95
              44  304 - #597c95
              36  312 - #000000
              52  312 - #000000
             328  396 - #597c95
             144  468 - #597c95
             428  536 - #597c95
               4  568 - #597c95
             260  592 - #597c95
             592  592 - #597c95
            ",
        )?;
        Ok(())
    }
}
