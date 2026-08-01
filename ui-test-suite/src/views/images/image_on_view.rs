use anyhow::Result;
use test_engine::{
    refs::Weak,
    ui::{Container, GREEN, ImageView, Setup, UIImages, ViewData, ViewSubviews, ViewTest, view},
    ui_test::helpers::check_colors,
};

#[view]
struct ImageOnView {
    image: Weak<ImageView>,

    #[init]
    container: Container,
}

impl Setup for ImageOnView {
    fn setup(mut self: Weak<Self>) {
        self.container.set_color(GREEN).place().size(200, 200).tl(100);

        self.image = self.container.add_view();

        self.image.set_image(UIImages::rb()).place().size(100, 100).center();
    }
}

impl ViewTest for ImageOnView {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             416    4 - #597c95
             592    4 - #597c95
             104  104 - #00ff00
             296  108 - #00ff00
             200  116 - #00ff00
             152  152 - #00ff00
             476  152 - #597c95
             248  156 - #444444
             224  180 - #444444
             296  180 - #00ff00
             104  188 - #00ff00
             220  200 - #444444
             248  200 - #444444
             200  204 - #444444
             224  220 - #444444
             188  236 - #444444
             248  236 - #444444
             296  240 - #00ff00
             108  244 - #00ff00
             160  248 - #444444
             216  248 - #444444
             104  296 - #00ff00
             188  296 - #00ff00
             248  296 - #00ff00
             592  296 - #597c95
             304  300 - #597c95
               4  420 - #597c95
             452  444 - #597c95
             152  476 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
