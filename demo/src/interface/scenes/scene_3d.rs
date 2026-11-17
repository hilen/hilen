use hilen::{
    Window,
    refs::Weak,
    scene::SceneManager,
    ui::{
        Anchor, BLACK, Button, Color, Container, Cursor, Label, NamedKey, Point, Setup, Slider, UIManager,
        ViewData, ViewSubviews, ViewTouch, WHITE, view,
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

/// The 3D playground page, seen through the player's eyes. It takes the
/// mouse the moment it opens, the mouse then turns the head, `w` `a`
/// `s` `d` or the arrows walk, space jumps. Escape gives the mouse back
/// and shows the controls: drop balls onto the pyramid, switch the fog
/// off and on, show the colliders, set how far the shadows reach and
/// how many texels their maps get. Escape again takes the mouse back,
/// it never quits the app on this page. On a touch screen the mouse is
/// never taken and a drag turns the head. The page's own touch sits
/// below its controls, so a drag on a slider moves the slider and not
/// the view.
#[view]
pub struct Scene3D {
    scene:          Weak<DemoScene>,
    last_touch:     Point,
    /// The app's quit on Escape, off while the page is open and put
    /// back when it closes.
    quit_on_escape: bool,

    back:             Weak<Button>,
    drop:             Weak<Button>,
    fog:              Weak<Button>,
    colliders:        Weak<Button>,
    panel:            Weak<Container>,
    distance:         Weak<Slider>,
    distance_caption: Weak<Label>,
    distance_value:   Weak<Label>,
    map:              Weak<Slider>,
    map_caption:      Weak<Label>,
    map_value:        Weak<Label>,

    #[init]
    /// Everything the mouse can click, gone while the mouse is captured.
    hud:       Container,
    crosshair: Crosshair,
}

impl Setup for Scene3D {
    fn setup(mut self: Weak<Self>) {
        self.scene = SceneManager::set_scene(DemoScene::default());

        self.enable_touch_low_priority();
        self.touch().began.val(move |touch| {
            self.last_touch = touch.position;
        });
        self.touch().moved.val(move |touch| {
            // A captured mouse turns the player itself, a drag of the
            // hidden cursor must not turn it twice.
            if Cursor::captured() {
                return;
            }
            let dx = touch.position.x - self.last_touch.x;
            let dy = touch.position.y - self.last_touch.y;
            self.last_touch = touch.position;
            if let Some(player) = self.scene.player.as_mut() {
                player.look(dx * LOOK_SPEED, -dy * LOOK_SPEED);
            }
        });

        self.hud.place().back();
        self.setup_hud();

        self.crosshair.place().size(CROSSHAIR_SIZE, CROSSHAIR_SIZE).center();
        self.crosshair.set_hidden(true);

        Cursor::on_capture().val(self, move |captured| {
            self.hud.set_hidden(captured);
            self.crosshair.set_hidden(!captured);
        });

        // A captured mouse takes Escape for itself in the engine, so this
        // only sees the Escape of a free mouse and captures it again.
        self.quit_on_escape = Window::quit_on_escape();
        Window::set_quit_on_escape(false);
        UIManager::keymap().add(self, NamedKey::Escape, Cursor::capture);

        Cursor::capture();
    }
}

impl Scene3D {
    fn setup_hud(mut self: Weak<Self>) {
        self.back = self.hud.add_view();
        self.back
            .set_color(ACCENT)
            .set_text_color(WHITE)
            .set_corner_radius(10)
            .set_text("Back");
        self.back.place().tl(20).size(90, 40);
        self.back.on_tap(move || {
            Window::set_quit_on_escape(self.quit_on_escape);
            SceneManager::stop_scene();
            UIManager::set_view(HomeView::new());
        });

        self.drop = self.hud.add_view();
        self.drop
            .set_text("Drop ball")
            .set_color(SURFACE)
            .set_text_color(TEXT)
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(BORDER);
        self.drop.place().t(20).r(20).size(120, 40);
        self.drop.on_tap(move || self.scene.drop_ball());

        self.fog = self.hud.add_view();
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

        self.colliders = self.hud.add_view();
        self.colliders
            .set_text("Colliders: off")
            .set_color(SURFACE)
            .set_text_color(TEXT)
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(BORDER);
        self.colliders.place().t(120).r(20).size(120, 40);
        self.colliders.on_tap(move || {
            self.scene.show_colliders = !self.scene.show_colliders;
            let state = if self.scene.show_colliders { "on" } else { "off" };
            self.colliders.set_text(format!("Colliders: {state}"));
        });

        self.panel = self.hud.add_view();
        self.panel.set_color(Color::rgba(0.0, 0.0, 0.0, 0.45)).set_corner_radius(12);
        self.panel.place().b(10).l(10).size(360, 150);

        self.distance = self.hud.add_view();
        self.distance.set_horizontal().place().b(20).l(20).size(220, 32);
        self.distance.set_range(10, 240).set_value(SHADOW_DISTANCE);
        self.distance.on_change.val(move |distance| {
            self.scene.sun.shadow_distance = distance;
            self.distance_value.set_text(format!("{distance:.0} m"));
        });
        self.distance_caption = caption(self.hud, "Shadow distance", self.distance);
        self.distance_value = value(self.hud, format!("{SHADOW_DISTANCE:.0} m"), self.distance);

        let default_size = self.scene.sun.shadow_map_size;
        self.map = self.hud.add_view();
        self.map.set_horizontal().place().b(90).l(20).size(220, 32);
        self.map
            .set_range(MAP_POWERS.0, MAP_POWERS.1)
            .set_value(f64::from(default_size).log2());
        self.map.on_change.val(move |power| {
            let size = map_size(power);
            self.scene.sun.shadow_map_size = size;
            self.map_value.set_text(format!("{size} px"));
        });
        self.map_caption = caption(self.hud, "Shadow map size", self.map);
        self.map_value = value(self.hud, format!("{default_size} px"), self.map);
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
fn caption(hud: Weak<Container>, text: &str, slider: Weak<Slider>) -> Weak<Label> {
    let label = hud.add_view::<Label>();
    label
        .set_text(text)
        .set_text_color(TEXT)
        .set_text_size(15)
        .place()
        .size(220, 22)
        .same_x(slider)
        .anchor(Anchor::Bot, slider, 2);
    label
}

/// The value of a slider, to its right.
fn value(hud: Weak<Container>, text: String, slider: Weak<Slider>) -> Weak<Label> {
    let label = hud.add_view::<Label>();
    label
        .set_text(text)
        .set_text_color(TEXT)
        .set_text_size(17)
        .place()
        .size(100, 32)
        .same_y(slider)
        .anchor(Anchor::Left, slider, 12);
    label
}

const CROSSHAIR_SIZE: f32 = 24.0;
const CROSSHAIR_LINE: f32 = 9.0;
/// A white line with a black outline, so it reads over a light sky and
/// a dark crate alike.
const CROSSHAIR_THICKNESS: f32 = 4.0;
const CROSSHAIR_OUTLINE: f32 = 1.0;

/// The Counter-Strike crosshair: four short lines around an empty
/// center, shown while the mouse is captured.
#[view]
struct Crosshair {
    #[init]
    up:    Container,
    down:  Container,
    left:  Container,
    right: Container,
}

impl Setup for Crosshair {
    fn setup(self: Weak<Self>) {
        for line in [self.up, self.down, self.left, self.right] {
            line.set_color(WHITE)
                .set_border_color(BLACK)
                .set_border_width(CROSSHAIR_OUTLINE);
        }
        self.up.place().size(CROSSHAIR_THICKNESS, CROSSHAIR_LINE).center_x().t(0);
        self.down.place().size(CROSSHAIR_THICKNESS, CROSSHAIR_LINE).center_x().b(0);
        self.left.place().size(CROSSHAIR_LINE, CROSSHAIR_THICKNESS).center_y().l(0);
        self.right.place().size(CROSSHAIR_LINE, CROSSHAIR_THICKNESS).center_y().r(0);
    }
}
