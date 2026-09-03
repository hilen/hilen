use hilen::{
    refs::Weak,
    scene::SceneManager,
    ui::{
        Anchor, Button, Color, Container, Label, Point, Setup, Slider, UIManager, ViewData, ViewTouch, WHITE,
        view,
    },
};

use crate::{
    interface::{
        HomeView,
        palette::{ACCENT, BORDER, SURFACE, TEXT},
    },
    scenes::{DemoScene, SHADOW_DISTANCE},
};

/// Radians of look per point of drag.
const LOOK_SPEED: f32 = 0.004;
/// The shadow map slider moves over powers of two, 512 to 4096 texels,
/// its value the power.
const MAP_POWERS: (f32, f32) = (9.0, 12.0);
const SMALLEST_MAP: u32 = 512;

/// The 3D playground page, seen through the player's eyes. Drag to
/// look around, `w` `a` `s` `d` or the arrows walk, space jumps, drop
/// balls onto the pyramid, switch the fog off and on, and set how far
/// the shadows reach and how many texels their maps get. The page's
/// own touch sits below its controls, so a drag on a slider moves the
/// slider and not the view.
#[view]
pub struct Scene3D {
    scene:      Weak<DemoScene>,
    last_touch: Point,

    #[init]
    panel:            Container,
    back:             Button,
    drop:             Button,
    fog:              Button,
    distance:         Slider,
    distance_caption: Label,
    distance_value:   Label,
    map:              Slider,
    map_caption:      Label,
    map_value:        Label,
}

impl Setup for Scene3D {
    fn setup(mut self: Weak<Self>) {
        self.scene = SceneManager::set_scene(DemoScene::default());

        self.enable_touch_low_priority();
        self.touch().began.val(move |touch| {
            self.last_touch = touch.position;
        });
        self.touch().moved.val(move |touch| {
            let dx = touch.position.x - self.last_touch.x;
            let dy = touch.position.y - self.last_touch.y;
            self.last_touch = touch.position;
            if let Some(player) = self.scene.player.as_mut() {
                player.look(dx * LOOK_SPEED, -dy * LOOK_SPEED);
            }
        });

        self.back
            .set_color(ACCENT)
            .set_text_color(WHITE)
            .set_corner_radius(10)
            .set_text("Back");
        self.back.place().tl(20).size(90, 40);
        self.back.on_tap(|| {
            SceneManager::stop_scene();
            UIManager::set_view(HomeView::new());
        });

        self.drop
            .set_text("Drop ball")
            .set_color(SURFACE)
            .set_text_color(TEXT)
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(BORDER);
        self.drop.place().t(20).r(20).size(120, 40);
        self.drop.on_tap(move || self.scene.drop_ball());

        self.fog
            .set_text("Fog: on")
            .set_color(SURFACE)
            .set_text_color(TEXT)
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(BORDER);
        self.fog.place().t(70).r(20).size(120, 40);
        self.fog.on_tap(move || {
            self.scene.toggle_fog();
            let state = if self.scene.fog.is_some() { "on" } else { "off" };
            self.fog.set_text(format!("Fog: {state}"));
        });

        self.panel.set_color(Color::rgba(0.0, 0.0, 0.0, 0.45)).set_corner_radius(12);
        self.panel.place().b(10).l(10).size(360, 150);

        self.distance.set_horizontal().place().b(20).l(20).size(220, 32);
        self.distance.set_range(10, 240).set_value(SHADOW_DISTANCE);
        self.distance.on_change.val(move |distance| {
            self.scene.sun.shadow_distance = distance;
            self.distance_value.set_text(format!("{distance:.0} m"));
        });
        caption(self.distance_caption, "Shadow distance", self.distance);
        value(
            self.distance_value,
            format!("{SHADOW_DISTANCE:.0} m"),
            self.distance,
        );

        let default_size = self.scene.sun.shadow_map_size;
        self.map.set_horizontal().place().b(90).l(20).size(220, 32);
        self.map
            .set_range(MAP_POWERS.0, MAP_POWERS.1)
            .set_value(f64::from(default_size).log2());
        self.map.on_change.val(move |power| {
            let size = map_size(power);
            self.scene.sun.shadow_map_size = size;
            self.map_value.set_text(format!("{size} px"));
        });
        caption(self.map_caption, "Shadow map size", self.map);
        value(self.map_value, format!("{default_size} px"), self.map);
    }
}

/// The map doubles for every whole step of the slider up from its
/// first power.
fn map_size(power: f32) -> u32 {
    let mut size = SMALLEST_MAP;
    let mut step = MAP_POWERS.0;
    while step + 0.5 < power {
        size *= 2;
        step += 1.0;
    }
    size
}

/// The name of a slider, above it.
fn caption(label: Weak<Label>, text: &str, slider: Weak<Slider>) {
    label
        .set_text(text)
        .set_text_color(TEXT)
        .set_text_size(15)
        .place()
        .size(220, 22)
        .same_x(slider)
        .anchor(Anchor::Bot, slider, 2);
}

/// The value of a slider, to its right.
fn value(label: Weak<Label>, text: String, slider: Weak<Slider>) {
    label
        .set_text(text)
        .set_text_color(TEXT)
        .set_text_size(17)
        .place()
        .size(100, 32)
        .same_y(slider)
        .anchor(Anchor::Left, slider, 12);
}
