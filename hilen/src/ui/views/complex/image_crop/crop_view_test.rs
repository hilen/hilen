use anyhow::{Result, ensure};

use super::{
    super::test_photo::{BOTTOM_LEFT, BOTTOM_RIGHT, TOP_LEFT, TOP_RIGHT, quarters_png},
    CropShape, ImageCropView,
};
use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::flat::Rect,
    ui::{Label, NamedKey, Setup, ViewData, ViewTest, view},
    ui_test::{
        check_colors, inject_key, inject_named_key, inject_pinch, inject_scroll, inject_touches,
        system_input::wait_until,
    },
};

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// A photo of 600 by 400 under the round mask of a crop view of 400 by
/// 400. The hole is 360 points across, so at zoom 1 a point is 1.11
/// pixels. A drag moves the photo, it stops at the edge, the wheel and a
/// pinch zoom, a press gives the view the arrow keys and `+` and `-`, and
/// Enter gives them back. The square mask, and the png of the cut.
#[view]
struct ImageCrop {
    #[init]
    status: Label,
    crop:   ImageCropView,
}

impl Setup for ImageCrop {
    fn setup(self: Weak<Self>) {
        self.set_color("#FFFFFF");
        self.status
            .set_text("reading the photo")
            .set_text_size(20)
            .set_text_color("#1B1F24");
        self.status.place().lrt(0).h(44);

        self.crop.set_color("#20242A");
        self.crop.place().t(60).l(100).size(400, 400);
        self.crop.on_change.sub(move || self.show_status());
        self.crop.set_photo(quarters_png());
    }
}

impl ImageCrop {
    fn show_status(self: Weak<Self>) {
        let Some(cut) = self.crop.cut() else {
            return;
        };
        self.status.set_text(format!(
            "zoom {:.2}   cut {:.0} {:.0} side {:.0}",
            self.crop.zoom(),
            cut.origin.x,
            cut.origin.y,
            cut.size.width
        ));
    }

    fn cut(self: Weak<Self>) -> Rect {
        from_main(move || self.crop.cut().unwrap())
    }

    fn zoom(self: Weak<Self>) -> f32 {
        from_main(move || self.crop.zoom())
    }
}

impl ViewTest for ImageCrop {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_until("the photo is in", move || view.crop.has_photo())?;
        // The middle of the photo, the 4 quarters meet in the hole.
        assert_eq!(view.cut(), Rect::new(100.0, 0.0, 400.0, 400.0));
        check_colors(CHECK_1)?;

        // 45 points to the right are 50 pixels.
        inject_touches("300 260 b\n345 260 m\n345 260 e");
        assert_eq!(view.cut(), Rect::new(50.0, 0.0, 400.0, 400.0));

        // Far to the left, the cut stops at the right edge of the photo.
        inject_touches("480 260 b\n110 260 m\n110 260 e");
        assert_eq!(view.cut(), Rect::new(200.0, 0.0, 400.0, 400.0));
        check_colors(CHECK_2)?;

        // The wheel zooms at the pointer, in the middle of the hole.
        inject_touches("300 260 m");
        inject_scroll(60);
        let wheel = 0.12_f32.exp();
        ensure!(near(view.zoom(), wheel), "zoom {} after the wheel", view.zoom());

        // A pinch on a trackpad doubles it.
        inject_pinch(300, 260, 2.0);
        ensure!(
            near(view.zoom(), wheel * 2.0),
            "zoom {} after the pinch",
            view.zoom()
        );
        check_colors(CHECK_3)?;

        // A press gives the view the keys, an arrow moves the photo by 10
        // points.
        inject_touches("300 260 b\n300 260 e");
        let before = view.cut();
        inject_named_key(NamedKey::ArrowRight);
        let scale = 360.0 / 400.0 * view.zoom();
        ensure!(
            near(view.cut().origin.x, before.origin.x - 10.0 / scale),
            "cut {:?} after the arrow, it was {before:?}",
            view.cut()
        );
        let zoom = view.zoom();
        inject_key('-');
        ensure!(near(view.zoom(), zoom / 1.15), "zoom {} after -", view.zoom());
        inject_key('+');
        ensure!(near(view.zoom(), zoom), "zoom {} after +", view.zoom());

        // Enter gives the keys back, the arrow moves the focus ring.
        inject_named_key(NamedKey::Enter);
        let before = view.cut();
        inject_named_key(NamedKey::ArrowRight);
        assert_eq!(view.cut(), before);

        // The square mask at zoom 1, the photo dragged to its right edge.
        from_main(move || {
            let crop = view.crop;
            crop.set_shape(CropShape::Square);
            crop.set_zoom(1.0);
        });
        inject_touches("480 260 b\n110 260 m\n110 260 e");
        assert_eq!(view.cut(), Rect::new(200.0, 0.0, 400.0, 400.0));
        check_colors(CHECK_4)?;

        // The cut as a png of 64: pixels 200 to 600 across and 0 to 400 down.
        let png = from_main(move || view.crop.cut_png(64))?;
        let cut = image::load_from_memory(&png)?.into_rgba8();
        assert_eq!(cut.dimensions(), (64, 64));
        assert_eq!(cut.get_pixel(4, 4).0, TOP_LEFT);
        assert_eq!(cut.get_pixel(48, 16).0, TOP_RIGHT);
        assert_eq!(cut.get_pixel(4, 60).0, BOTTOM_LEFT);
        assert_eq!(cut.get_pixel(48, 48).0, BOTTOM_RIGHT);

        Ok(())
    }
}

const CHECK_1: &str = r"
     312   16 - #45484c
     204   20 - #1b1f24
     420   20 - #ffffff
     248   24 - #1c2025
     168   28 - #494d51
     388  140 - #1c7ed6
     224  152 - #e03131
     392  232 - #1c7ed6
     212  240 - #e03131
     216  332 - #2f9e44
     356  340 - #f59f00
     284  364 - #2f9e44
     412  456 - #0e1013
     496  456 - #0e1013
       4  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_2: &str = r"
     384   16 - #1b1f24
     224   20 - #35383d
     284   20 - #7f8184
     204   24 - #414448
     248   24 - #1c2025
     168   28 - #494d51
     312   28 - #494d51
     284  168 - #1c7ed6
     384  184 - #1c7ed6
     200  316 - #2f9e44
     360  348 - #f59f00
     220  428 - #6e4800
     496  428 - #0e1013
     404  456 - #0e1013
       4  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_3: &str = r"
     340   16 - #cecfd0
     420   16 - #2e3136
     188   24 - #d8d8d9
     244   24 - #1c2025
     312   24 - #45484c
     340   24 - #cecfd0
     384   24 - #1b1f24
     156   28 - #494d51
     212   28 - #494d51
     412  104 - #0d3961
     496  116 - #0d3961
     260  164 - #1c7ed6
     384  184 - #1c7ed6
     304  264 - #f59f00
     400  344 - #f59f00
     496  424 - #6e4800
       4  592 - #ffffff
     412  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_4: &str = r"
     384   16 - #1b1f24
     224   20 - #35383d
     284   20 - #7f8184
     332   20 - #ffffff
     432   20 - #ffffff
     204   24 - #414448
     212   24 - #929496
     248   24 - #1c2025
     280   24 - #ffffff
     296   24 - #4e5155
     372   24 - #7c7e81
     384   24 - #1b1f24
     168   28 - #494d51
     308   28 - #494d51
     348  164 - #1c7ed6
     308  296 - #f59f00
     496  368 - #0e1013
     420  456 - #0e1013
       4  592 - #ffffff
     592  592 - #ffffff
";
