use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::{
        Clock, Shape,
        color::{RED, WHITE},
    },
    level::{Banner, LevelCreation, LevelManager, LevelSetup, LevelTest, Sprite, SpriteTemplates, level},
    refs::Weak,
    ui::{Mouse, Point, UIManager},
    ui_test::{capture_screenshot, checkpoint, inject_right_click, inject_touches, step_frames},
    window::MouseButton,
};

const TARGET: Point = Point::new(14.0, 3.0);

/// A red box off the camera center at level scale 2, and a white marker
/// that follows `cursor_position` every step. The pointer moves onto the
/// box through its screen point, the marker lands on it, and the button
/// states follow the presses.
#[level]
#[derive(Default)]
struct LevelMouse {
    marker: Weak<Banner>,
    taps:   Vec<Point>,
}

impl LevelSetup for LevelMouse {
    fn setup(&mut self) {
        self.make_sprite::<Banner>(Shape::rect(2, 2), TARGET).set_color(RED);
        self.marker = self.make_sprite::<Banner>(Shape::rect(0.6, 0.6), (0, 0));
        self.marker.set_color(WHITE).to_foreground();
        self.on_tap
            .val(|pos| LevelManager::downcast_level::<LevelMouse>().taps.push(pos));
    }

    fn update(&mut self, _: f32) {
        let cursor = self.cursor_position;
        self.marker.set_position(cursor);
    }
}

fn held(button: MouseButton) -> bool {
    from_main(move || Mouse::held(button))
}

fn close(a: Point, b: Point) -> bool {
    (a - b).length() < 0.05
}

impl LevelTest for LevelMouse {
    fn perform_test(level: Weak<Self>) -> Result<()> {
        // The marker moves in the level step, the stepped clock makes sure
        // steps run between two checks however fast the frames are.
        from_main(|| {
            Clock::enter_stepped();
            *LevelManager::camera_pos() = Point::new(10.0, 5.0);
            LevelManager::set_scale(2.0);
        });
        step_frames(1);

        let screen = from_main(|| LevelManager::screen_point(TARGET));
        let (x, y) = (screen.x, screen.y);

        let shot = capture_screenshot()?;
        let scale = from_main(UIManager::scale);
        let pixel = |point: Point| shot.get_pixel(point * scale);
        ensure!(
            pixel(screen) == RED.into(),
            "the screen point of the box is not on the box"
        );
        let beside = from_main(|| LevelManager::screen_point(TARGET + Point::new(2.0, 0.0)));
        ensure!(pixel(beside) != RED.into(), "the box is drawn wider than it is");

        inject_touches(format!("{x} {y} m"));
        step_frames(2);
        let cursor = from_main(move || level.cursor_position);
        ensure!(
            close(cursor, TARGET),
            "the cursor is at {cursor:?}, the box at {TARGET:?}"
        );
        ensure!(!held(MouseButton::Left), "a move holds no button");
        checkpoint("pointer on the red box, the white marker on it")?;

        inject_touches(format!("{x} {y} b"));
        ensure!(held(MouseButton::Left), "the pressed button is not held");
        let tap = from_main(move || level.taps.last().copied());
        ensure!(
            tap.is_some_and(|tap| close(tap, TARGET)),
            "the tap landed at {tap:?}, the box is at {TARGET:?}"
        );

        inject_touches(format!("{x} {y} e"));
        ensure!(!held(MouseButton::Left), "the released button is still held");

        inject_touches(format!("{x} {y} b 2\n{x} {y} b 3\n{x} {y} e 2"));
        ensure!(
            held(MouseButton::Left),
            "one of two fingers lifted, the other still holds"
        );
        inject_touches(format!("{x} {y} e 3"));
        ensure!(!held(MouseButton::Left), "both fingers lifted");

        inject_right_click(x, y);
        ensure!(
            !held(MouseButton::Right) && !held(MouseButton::Left),
            "a finished right click holds nothing"
        );

        from_main(Clock::exit_stepped);
        Ok(())
    }
}
