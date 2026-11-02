use std::sync::atomic::{AtomicU64, Ordering};

use super::mesh::{WORLD_HALF, surface_mesh};
use crate::{
    self as hilen,
    deps::refs::{Weak, manage::DataManager},
    gm::{
        color::{Color, WHITE},
        volume::{Shape3, Vec3},
    },
    scene::{Camera, Line3, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneView, scene},
    ui::{DynamicColor, Setup, UIEvents, ViewData, view},
};

const GRID: DynamicColor = DynamicColor::new(Color::hex("#B8BEC8"), Color::hex("#3A4250"));
const BACKGROUND: DynamicColor = DynamicColor::new(Color::hex("#F4F5F7"), Color::hex("#14171F"));

/// Each upload needs a name of its own, the store returns the old model
/// for a name it already holds.
static UPLOADS: AtomicU64 = AtomicU64::new(0);

/// The height at x and y, and a color of its own or none for the color of the
/// height.
type SurfaceFunction = Box<dyn Fn(f64, f64) -> (f64, Option<Color>)>;

#[scene]
#[derive(Default)]
struct SurfaceScene {}

impl SceneSetup for SurfaceScene {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(9.0, 8.0, 12.0),
            target: Vec3::ZERO,
            ..Camera::default()
        };
    }
}

/// A 3D graph of `z = f(x, y)` as a surface over a square, colored by
/// height, over a floor grid with the 3 axes. A drag turns the camera
/// around the middle, the wheel and a pinch zoom. Math z is up on the
/// screen. It runs the one scene of `SceneManager`, see `SceneView`.
#[view]
pub struct SurfacePlotView {
    half:     f64,
    steps:    u32,
    function: Option<SurfaceFunction>,
    scene:    Weak<SurfaceScene>,
    model:    Weak<Model>,

    #[init]
    scene_view: SceneView,
}

impl SurfacePlotView {
    /// Draws the surface of `z = function(x, y)`, in place of the one
    /// before. A value that is not finite leaves a hole.
    pub fn set_function(&mut self, function: impl Fn(f64, f64) -> f64 + 'static) -> &mut Self {
        self.function = Some(Box::new(move |x, y| (function(x, y), None)));
        self.rebuild();
        self
    }

    /// Like `set_function` with a color of its own at each point, the
    /// height and the color from 1 call. The modulus surface of a complex
    /// function is the case: the height is its size, the color its angle.
    pub fn set_colored_function(
        &mut self,
        function: impl Fn(f64, f64) -> (f64, Color) + 'static,
    ) -> &mut Self {
        self.function = Some(Box::new(move |x, y| {
            let (height, color) = function(x, y);
            (height, Some(color))
        }));
        self.rebuild();
        self
    }

    /// Removes the surface, the grid and the axes stay.
    pub fn remove_function(&mut self) -> &mut Self {
        self.function = None;
        self.rebuild();
        self
    }

    /// Shows x and y from `-half` to `half`, 10 at the start.
    pub fn set_range(&mut self, half: f64) -> &mut Self {
        self.half = half;
        self.rebuild();
        self
    }

    /// How many cells a side the surface has, 80 at the start.
    pub fn set_steps(&mut self, steps: u32) -> &mut Self {
        self.steps = steps.max(1);
        self.rebuild();
        self
    }

    /// The camera of the scene, to turn or zoom it from code.
    pub fn camera(&self) -> Camera {
        self.scene.camera
    }

    pub fn set_camera(&mut self, camera: Camera) -> &mut Self {
        self.scene.camera = camera;
        self
    }

    /// Whether the surface has triangles, false with no function or with
    /// no finite value in the range.
    pub fn has_surface(&self) -> bool {
        self.model.is_ok()
    }

    fn rebuild(&mut self) {
        if !self.scene.is_ok() {
            return;
        }
        self.scene.remove_all_nodes();
        if self.model.is_ok() {
            self.model.free();
            self.model = Weak::default();
        }
        let Some(function) = &self.function else {
            return;
        };
        let mesh = surface_mesh(function.as_ref(), self.half, self.steps);
        if mesh.indices.is_empty() {
            return;
        }
        let name = format!("surface-plot-{}", UPLOADS.fetch_add(1, Ordering::Relaxed));
        self.model = Model::from_smooth_mesh(&name, mesh);
        self.scene
            .make_node::<Prop>(Shape3::Model(self.model), Vec3::ZERO)
            .set_color(WHITE)
            .set_roughness(0.8);
    }

    fn theme(&mut self) {
        self.scene_view.set_background(BACKGROUND.resolve());
        if self.scene.is_ok() {
            let mut lines = Line3::grid(2.0 * WORLD_HALF, WORLD_HALF / 5.0, GRID.resolve());
            lines.extend(Line3::axes(WORLD_HALF * 1.3).into_iter().map(|line| line.width(3)));
            self.scene.lines = lines;
        }
    }
}

impl Setup for SurfacePlotView {
    fn setup(mut self: Weak<Self>) {
        self.half = 10.0;
        self.steps = 80;
        self.scene_view.place().back();
        self.scene = self.scene_view.set_scene(SurfaceScene::default());
        self.theme();
        UIEvents::theme_changed().sub(self, move || self.theme());
    }
}
