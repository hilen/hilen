use anyhow::{Result, anyhow};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    ui::{Anchor::Top, ImageView, Setup, Size, UIManager, ViewData, ViewTest, view},
    ui_test::helpers::check_colors,
    window::{
        Window,
        image::{Image, RASTER_KEEP_FRAMES, Svg},
    },
};

// One svg drawn at three sizes. Each size rasterizes at its own pixel
// size, so the big copy has clean edges instead of the old fixed
// bitmap's steps, and a size not drawn for a while is dropped.
#[view]
struct SvgScale {
    #[init]
    big:    ImageView,
    medium: ImageView,
    small:  ImageView,
}

impl Setup for SvgScale {
    fn setup(self: Weak<Self>) {
        self.big.place().tl(20).size(600, 600);
        self.big.set_image("bin.svg");

        self.medium.place().same_x(self.big).anchor(Top, self.big, 20).size(240, 240);
        self.medium.set_image("bin.svg");

        self.small.place().anchor(Top, self.big, 20).l(300).size(24, 24);
        self.small.set_image("bin.svg");
    }
}

impl ViewTest for SvgScale {
    fn canvas() -> (u32, u32) {
        (640, 1000)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
               4    4 - #597c95
             632    4 - #597c95
             276   72 - #007bff
             460  148 - #007bff
             492  236 - #2c7cca
             148  264 - #2c7cca
             348  276 - #007bff
             488  344 - #2c7cca
             204  424 - #2c7cca
             356  428 - #007bff
             484  452 - #2c7cca
             156  480 - #2c7cca
             468  532 - #007bff
               4  544 - #597c95
             200  564 - #007bff
             632  628 - #597c95
             312  644 - #597c95
             308  648 - #597c95
             312  648 - #597c95
             308  652 - #597c95
             312  652 - #597c95
             308  656 - #597c95
             312  656 - #597c95
             308  660 - #007bff
             312  660 - #007bff
             316  660 - #007bff
             188  740 - #2c7cca
              72  768 - #2c7cca
             488  812 - #597c95
              84  852 - #007bff
             340  992 - #597c95
             632  992 - #597c95
            ",
        )?;

        let scale = UIManager::scale();
        let pixels = |points: u32| {
            let side: u32 = (points.lossy_convert() * scale).round().lossy_convert();
            Size::new(side, side)
        };

        let sizes = raster_sizes("bin.svg");
        for points in [600, 240, 24] {
            if !sizes.contains(&pixels(points)) {
                return Err(anyhow!("no raster for {points} points, cached: {sizes:?}"));
            }
        }

        check_eviction(view, &pixels)
    }
}

fn check_eviction(view: Weak<SvgScale>, pixels: &dyn Fn(u32) -> Size<u32>) -> Result<()> {
    let resized_on = from_main(move || {
        view.big.place().size(300, 300);
        Window::render_frame()
    });
    wait_for_next_frame();

    check_colors(
        r"
         632    4 - #597c95
         172   48 - #007bff
          68   84 - #007bff
         256  120 - #2c7cca
          84  148 - #2c7cca
         132  148 - #2c7cca
         132  168 - #2c7cca
         132  184 - #2c7cca
         132  200 - #2c7cca
         132  216 - #2c7cca
         252  228 - #2c7cca
          88  256 - #2c7cca
         608  300 - #597c95
         312  344 - #597c95
         308  348 - #597c95
         308  352 - #597c95
         312  352 - #597c95
         308  356 - #597c95
         312  356 - #597c95
         152  360 - #007bff
         308  360 - #007bff
         312  360 - #007bff
         316  360 - #007bff
          56  392 - #007bff
         208  440 - #2c7cca
          92  468 - #2c7cca
         196  548 - #007bff
          84  552 - #007bff
         632  596 - #597c95
         320  864 - #597c95
           4  992 - #597c95
         632  992 - #597c95
        ",
    )?;

    // The headless loop pumps frames as fast as it can, so how many
    // passed since the resize is read together with the sizes and
    // only what that delta supports is asserted.
    let (frame, sizes) = frame_and_raster_sizes("bin.svg");
    if !sizes.contains(&pixels(300)) {
        return Err(anyhow!("no raster for the new 300 points, cached: {sizes:?}"));
    }
    if frame < resized_on + RASTER_KEEP_FRAMES && !sizes.contains(&pixels(600)) {
        return Err(anyhow!(
            "the old 600 points raster was dropped {} frames after its last draw: {sizes:?}",
            frame - resized_on
        ));
    }

    while from_main(Window::render_frame) <= resized_on + RASTER_KEEP_FRAMES {
        wait_for_next_frame();
    }

    let (_, sizes) = frame_and_raster_sizes("bin.svg");
    if sizes.contains(&pixels(600)) {
        return Err(anyhow!("the unused 600 points raster was not dropped: {sizes:?}"));
    }
    if !sizes.contains(&pixels(300)) {
        return Err(anyhow!("the drawn 300 points raster was dropped: {sizes:?}"));
    }

    Ok(())
}

fn raster_sizes(name: &'static str) -> Vec<Size<u32>> {
    frame_and_raster_sizes(name).1
}

fn frame_and_raster_sizes(name: &'static str) -> (u64, Vec<Size<u32>>) {
    from_main(move || {
        let image: Weak<Image> = Image::get(name);
        let sizes = image.svg.as_ref().map(Svg::raster_sizes).unwrap_or_default();
        (Window::render_frame(), sizes)
    })
}
