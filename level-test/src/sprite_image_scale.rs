use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::{Shape, color::WHITE},
    level::{Banner, LevelCreation, LevelManager, LevelSetup, LevelTest, Sprite, SpriteTemplates, level},
    refs::Weak,
    ui::{Image, Point, UIManager},
    ui_test::{capture_screenshot, check_colors},
};

const BODY: Point = Point::new(4.0, 2.0);
const CAMERA: Point = Point::new(10.0, 5.0);

/// An orange picture drawn at 2 times its 4 by 4 body, with a white box of
/// the body size on the same point, and the camera away from both. The
/// image scale used to stretch the distance to the camera too, the picture
/// was on its body only while the camera sat on the sprite.
#[level]
#[derive(Default)]
struct SpriteImageScale {}

impl LevelSetup for SpriteImageScale {
    fn setup(&mut self) {
        const ORANGE: [u8; 4] = [230, 120, 20, 255];
        let image = Image::from_raw_data(
            [ORANGE; 4].concat(),
            "sprite_image_scale_orange",
            (2, 2).into(),
            4,
        );

        let mut picture = self.make_sprite::<Banner>(Shape::rect(4, 4), BODY);
        picture.set_image(image);
        picture.image_scale = 2.0;

        self.make_sprite::<Banner>(Shape::rect(4, 4), BODY)
            .set_color(WHITE)
            .to_foreground();

        *LevelManager::camera_pos() = CAMERA;
    }
}

impl LevelTest for SpriteImageScale {
    fn perform_test(_level: Weak<Self>) -> Result<()> {
        let shot = capture_screenshot()?;
        let scale = from_main(UIManager::scale);
        let pixel = |offset: Point| {
            let screen = from_main(move || LevelManager::screen_point(BODY + offset));
            shot.get_pixel(screen * scale)
        };
        let background = pixel(Point::new(-20.0, 0.0));
        let orange = pixel(Point::new(3.0, 0.0));

        ensure!(
            pixel(Point::default()) == WHITE.into(),
            "the body is not at its level point"
        );
        ensure!(orange != background, "the picture is not around its body");

        // The picture is 8 by 8, it ends 4 units from the body center.
        for (inside, outside) in [
            (Point::new(3.5, 0.0), Point::new(4.5, 0.0)),
            (Point::new(-3.5, 0.0), Point::new(-4.5, 0.0)),
            (Point::new(0.0, 3.5), Point::new(0.0, 4.5)),
            (Point::new(0.0, -3.5), Point::new(0.0, -4.5)),
        ] {
            ensure!(
                pixel(inside) == orange,
                "no picture {inside:?} from the body, {:?} is there",
                pixel(inside)
            );
            ensure!(
                pixel(outside) == background,
                "the picture reaches {outside:?} from the body"
            );
        }

        check_colors(SCALED)
    }
}

const SCALED: &str = r"
   4    4 - #597c95
 300    4 - #597c95
 592    4 - #597c95
 444  152 - #597c95
   4  216 - #597c95
 200  292 - #e67814
 224  292 - #e67814
 264  292 - #e67814
 300  300 - #597c95
 592  300 - #597c95
 236  312 - #ffffff
 256  312 - #ffffff
 220  316 - #ffffff
 200  320 - #e67814
 244  320 - #ffffff
 272  324 - #e67814
 252  328 - #ffffff
 236  332 - #ffffff
 220  336 - #ffffff
 256  340 - #ffffff
 200  344 - #e67814
 228  348 - #ffffff
 244  348 - #ffffff
 268  348 - #e67814
 200  368 - #e67814
 228  368 - #e67814
 252  368 - #e67814
 276  368 - #e67814
 456  432 - #597c95
   4  592 - #597c95
 352  592 - #597c95
 592  592 - #597c95
";
