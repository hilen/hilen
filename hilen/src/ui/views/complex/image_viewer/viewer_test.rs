use anyhow::{Result, ensure};
use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder};

use super::{ImageViewer, ImageViewerRequest};
use crate::{
    self as hilen,
    deps::{
        hreads::from_main,
        refs::{Weak, manage::DataManager},
    },
    image::Image,
    ui::{Label, ModalView, NamedKey, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, checkpoint, inject_named_key, inject_touches, system_input::wait_until},
};

/// A png in 4 colored quarters, so a zoom shows which part is in front.
fn quarters(wide: u32, high: u32) -> Vec<u8> {
    let picture = RgbaImage::from_fn(wide, high, |x, y| {
        Rgba(match (x < wide / 2, y < high / 2) {
            (true, true) => [224, 49, 49, 255],
            (false, true) => [28, 126, 214, 255],
            (true, false) => [47, 158, 68, 255],
            (false, false) => [245, 159, 0, 255],
        })
    });
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(picture.as_raw(), wide, high, ExtendedColorType::Rgba8)
        .unwrap();
    png
}

/// The viewer over a canvas of 600 with 2 pictures. The first is 600 by
/// 400, fitted to 552 by 368 at 24 116. The second is 100 square, it
/// has fewer pixels than the room, so it stays 100 at 250 250.
#[view]
struct ImageViewerTest {
    closed: usize,
    viewer: Weak<ImageViewer>,

    #[init]
    status: Label,
}

impl Setup for ImageViewerTest {
    fn setup(self: Weak<Self>) {
        self.status.set_text("the page under the viewer").set_text_size(20);
        self.status.place().lrb(0).h(44);
    }
}

impl ImageViewerTest {
    fn open(self: Weak<Self>, at: usize) {
        from_main(move || {
            let mut this = self;
            let images = vec![
                Image::load(&quarters(600, 400), "viewer test wide"),
                Image::load(&quarters(100, 100), "viewer test small"),
            ];
            let viewer = ImageViewer::prepare_modally_with_input(ImageViewerRequest::images(images, at));
            viewer.modal_event().val(move |()| {
                let mut this = self;
                this.closed += 1;
            });
            this.viewer = viewer;
        });
    }

    fn closed(self: Weak<Self>) -> usize {
        from_main(move || self.closed)
    }

    fn zoom(self: Weak<Self>) -> f32 {
        from_main(move || self.viewer.zoom())
    }
}

impl ViewTest for ImageViewerTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        view.open(0);
        check_colors(FITTED)?;

        // A double tap on the picture zooms in at the finger, the red
        // quarter fills the window.
        inject_touches("150 200 b\n150 200 e\n150 200 b\n150 200 e");
        let zoom = view.zoom();
        ensure!((zoom - 2.5).abs() < 0.01, "zoom {zoom} after a double tap");
        check_colors(ZOOMED)?;

        // A drag pans and does not close, also when it ends beside where
        // the picture was.
        inject_touches("300 300 b\n200 300 m\n100 300 m\n20 300 e");
        ensure!(view.closed() == 0, "a drag closed the viewer");
        checkpoint("panned, the viewer is still open")?;

        // A second double tap goes back to fitted.
        inject_touches("300 300 b\n300 300 e\n300 300 b\n300 300 e");
        let zoom = view.zoom();
        ensure!(
            (zoom - 1.0).abs() < 0.01,
            "zoom {zoom} after the second double tap"
        );

        // The right arrow steps to the small picture.
        inject_named_key(NamedKey::ArrowRight);
        wait_until("the right arrow went to the small picture", move || {
            view.viewer.page_index() == 1
        })?;
        check_colors(SMALL)?;

        // A tap on the picture keeps it, a tap beside it closes.
        inject_touches("300 300 b\n300 300 e");
        ensure!(view.closed() == 0, "a tap on the picture closed the viewer");
        inject_touches("100 500 b\n100 500 e");
        wait_until("a tap beside closed it", move || view.closed == 1)?;

        // The close button closes.
        view.open(1);
        inject_touches("566 34 b\n566 34 e");
        wait_until("the close button closed it", move || view.closed == 2)?;

        // Escape closes.
        view.open(0);
        inject_named_key(NamedKey::Escape);
        wait_until("Escape closed it", move || view.closed == 3)?;

        Ok(())
    }
}

const FITTED: &str = r"
     140    4 - #0b0f12
     300    4 - #0b0f12
     460    4 - #0b0f12
     160  136 - #e03131
     396  156 - #1c7ed6
     240  244 - #e03131
     344  244 - #1c7ed6
     452  248 - #1c7ed6
      84  260 - #e03131
     552  284 - #0d3960
     560  292 - #0d3960
     168  296 - #e03131
     560  304 - #6e4800
     556  316 - #6e4800
     384  336 - #f59f00
     276  344 - #2f9e44
     476  368 - #f59f00
     180  388 - #2f9e44
     444  460 - #f59f00
     208  476 - #2f9e44
     112  480 - #2f9e44
     484  592 - #0b0f12
";

const ZOOMED: &str = r"
      92    4 - #e03131
     268   28 - #651616
     332   28 - #651616
     300   36 - #651616
     580   48 - #0d3960
     592   88 - #1c7ed6
       4  156 - #e03131
     592  240 - #1c7ed6
     584  308 - #0d3960
     248  312 - #e03131
     588  380 - #1c7ed6
       4  448 - #e03131
     592  452 - #f59f00
     352  456 - #2f9e44
     576  500 - #f59f00
     592  552 - #f59f00
      76  592 - #2f9e44
     240  592 - #2f9e44
     568  592 - #f59f00
";

const SMALL: &str = r"
       4    4 - #0b0f12
     252    4 - #0b0f12
     592    4 - #0b0f12
     300  240 - #0b0f12
     268  252 - #e03131
     340  252 - #1c7ed6
     288  264 - #e03131
     320  276 - #1c7ed6
     312  296 - #1c7ed6
       4  304 - #0b0f12
     264  304 - #2f9e44
     320  312 - #f59f00
     280  324 - #2f9e44
     336  316 - #f59f00
     268  344 - #2f9e44
     348  328 - #f59f00
       4  592 - #0b0f12
     592  592 - #0b0f12
";
