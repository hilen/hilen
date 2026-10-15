use anyhow::{Result, anyhow, ensure};
use futures_util::FutureExt;
use hilen::{
    camera::{Camera, CameraView, FakeCamera, PhotoFormat},
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Label, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, checkpoint},
};

/// The first picture, red, green, blue and yellow.
const FIRST: &str = r"
             168   36 - #2d3f4b
             220   36 - #000001
             100   40 - #597c95
             476   40 - #507086
             136   44 - #151d22
             168   48 - #2d3f4b
             340   48 - #1d2830
             412  116 - #28b43c
             300  128 - #707c34
             140  144 - #dc2828
             300  176 - #707c34
             212  212 - #dc2828
             420  216 - #28b43c
               4  220 - #597c95
             164  220 - #703a94
             296  220 - #703a94
             360  220 - #a0c630
             456  220 - #a0c630
             140  264 - #2846dc
             300  264 - #a09a70
             384  300 - #f0d228
             300  308 - #a09a70
             188  336 - #2846dc
             456  336 - #f0d228
             304  396 - #000001
             152  400 - #507086
             224  404 - #597c95
             396  404 - #000000
              84  408 - #2b3c48
             208  408 - #2d3f4c
             476  408 - #000000
             592  592 - #597c95
";

/// The second picture, the colors in the other corners.
const SECOND: &str = r"
             220   36 - #000001
             156   40 - #507086
             500   40 - #597c95
             412   44 - #597c95
             168   48 - #2d3f4b
             324   48 - #000000
             372  128 - #2846dc
             140  152 - #f0d228
             296  192 - #f0d228
               4  200 - #597c95
             456  216 - #2846dc
             180  220 - #78c034
             224  220 - #78c034
             328  220 - #943470
             372  220 - #943470
             152  256 - #28b43c
             300  284 - #946030
             456  296 - #dc2828
             216  312 - #28b43c
             380  324 - #dc2828
             300  336 - #946030
             308  388 - #3d5667
             184  392 - #000001
              92  396 - #1b262e
             272  396 - #000000
             416  400 - #507086
              92  404 - #1b262e
             448  404 - #24333d
             180  408 - #2f414e
             308  408 - #3d5667
             480  408 - #2d3f4c
             592  592 - #597c95
";

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;

const RED: [u8; 4] = [220, 40, 40, 255];
const GREEN: [u8; 4] = [40, 180, 60, 255];
const BLUE: [u8; 4] = [40, 70, 220, 255];
const YELLOW: [u8; 4] = [240, 210, 40, 255];

/// A picture of 4 colors, 1 in each quarter, so a turn or a flip shows.
fn quarters(top_left: [u8; 4], top_right: [u8; 4], bottom_left: [u8; 4], bottom_right: [u8; 4]) -> Vec<u8> {
    let mut rgba = Vec::new();
    for row in 0..HEIGHT {
        for column in 0..WIDTH {
            let color = match (row < HEIGHT / 2, column < WIDTH / 2) {
                (true, true) => top_left,
                (true, false) => top_right,
                (false, true) => bottom_left,
                (false, false) => bottom_right,
            };
            rgba.extend(color);
        }
    }
    rgba
}

/// The fake camera answers inside the call, so 1 poll of a future is its
/// answer.
fn now<T>(future: impl Future<Output = Result<T>>) -> Result<T> {
    future
        .now_or_never()
        .ok_or_else(|| anyhow!("the fake camera did not answer at once"))?
}

#[view]
struct CameraViewTest {
    #[init]
    title:   Label,
    preview: CameraView,
    status:  Label,
}

impl Setup for CameraViewTest {
    fn setup(self: Weak<Self>) {
        self.title.set_text("camera view of a fake camera");
        self.title.place().t(20).lr(20).h(40);
        self.preview.set_color((30, 30, 30));
        self.preview.place().t(100).center_x().size(320, 240);
        self.status.set_text("no camera");
        self.status.place().t(380).lr(20).h(40);
    }
}

impl ViewTest for CameraViewTest {
    fn before_start() {
        FakeCamera::install();
        FakeCamera::show(quarters(RED, GREEN, BLUE, YELLOW), WIDTH, HEIGHT);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let devices = now(Camera::devices())?;
        let device = devices.first().ok_or_else(|| anyhow!("the fake camera is not listed"))?;
        let camera = now(Camera::open(device))?;

        let shown = camera.clone();
        from_main(move || {
            view.preview.set_camera(Some(shown));
            view.status.set_text("frame 1: red, green, blue, yellow");
        });
        wait_for_next_frame();
        wait_for_next_frame();
        ensure!(from_main(move || view.preview.shown_frame()) == 1);
        check_colors(FIRST)?;

        FakeCamera::show(quarters(YELLOW, BLUE, GREEN, RED), WIDTH, HEIGHT);
        from_main(move || {
            view.status.set_text("frame 2: yellow, blue, green, red");
        });
        wait_for_next_frame();
        wait_for_next_frame();
        ensure!(from_main(move || view.preview.shown_frame()) == 2);
        check_colors(SECOND)?;

        let photo = now(camera.take_photo(PhotoFormat::Jpeg))?;
        ensure!(photo.format == PhotoFormat::Jpeg);
        ensure!((photo.width, photo.height) == (WIDTH, HEIGHT));
        let text = format!(
            "photo: {} by {}, {} bytes of JPEG",
            photo.width,
            photo.height,
            photo.bytes.len()
        );
        from_main(move || {
            view.status.set_text(text);
        });
        checkpoint("the photo is a JPEG of the second picture")?;

        from_main(move || {
            view.preview.set_camera(None);
        });
        drop(camera);
        FakeCamera::remove();
        Ok(())
    }
}
