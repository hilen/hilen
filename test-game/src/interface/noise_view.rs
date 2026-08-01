use test_engine::{
    Event,
    generate::noise::{TerrainData, TerrainParams, generate_terrain},
    gm::LossyConvert,
    level::LevelManager,
    refs::{Own, Weak},
    ui::{
        AddLabel, Anchor,
        Anchor::{Top, X},
        BLACK, Button, Container, DrawingView, FillRule, Image, ImageView, Label, NumberView, Point, Setup,
        Size, TURQUOISE, VectorPath, ViewData, ViewSubviews, ViewTouch, WHITE, view,
    },
};

use crate::{
    interface::{polygon_view::PolygonView, scenes::add_back_button},
    levels::NoiseLevel,
};

#[view]
pub struct NoiseView {
    seed: u32,

    on_back: Event,

    islands: Vec<Vec<Point>>,

    threshold_view: Weak<NumberView>,
    x_view:         Weak<NumberView>,
    y_view:         Weak<NumberView>,
    size_view:      Weak<NumberView>,
    skip_view:      Weak<NumberView>,

    #[init]
    drawing_view:  DrawingView,
    controls:      Container,
    image_view:    ImageView,
    counter_label: Label,
    update_level:  Button,
    polygon:       PolygonView,
}

impl NoiseView {
    fn update_image(mut self: Weak<Self>) {
        let resolution: Size<u32> = (100, 100).into();

        let (image, islands) = generate_image(TerrainParams {
            seed: self.seed,
            resolution,
            size: (self.size_view.value(), self.size_view.value()).into(),
            position: (self.x_view.value(), self.y_view.value()).into(),
            threshold: self.threshold_view.value().lossy_convert(),
            skip: self.skip_view.value().lossy_convert(),
        });

        self.counter_label.set_text(format!("{}", islands.len()));

        self.image_view.set_image(image);

        self.drawing_view.remove_all_paths();

        for island in &islands {
            self.drawing_view.add_path(island.iter().map(|a| *a * 20.0), BLACK);
        }

        self.islands = islands;

        self.drawing_view
            .add_fill(&VectorPath::circle((200, 100), 50), TURQUOISE, FillRule::NonZero);
    }

    fn update_level(self: Weak<Self>) {
        LevelManager::downcast_level::<NoiseLevel>().add_islands(
            self.islands
                .iter()
                .map(|p| p.iter().map(|p| (p.x, -p.y).into()).collect())
                .collect(),
        );

        let biggest_size = self.islands.iter().map(Vec::len).max().unwrap();

        if biggest_size < 5 {
            return;
        }

        let smallest_island = self.islands.iter().find(|i| i.len() == biggest_size).unwrap().clone();

        self.polygon.display_points(smallest_island);
    }

    pub fn on_back(self: Own<Self>, callback: impl FnMut() + Send + 'static) -> Own<Self> {
        self.on_back.sub(callback);
        self
    }
}

impl Setup for NoiseView {
    fn setup(mut self: Weak<Self>) {
        LevelManager::set_level(NoiseLevel::default());

        self.drawing_view.place().back();

        self.enable_touch_low_priority();
        self.touch().up_inside.sub(self, move || self.update_image());

        let update_image = move |_| self.update_image();

        // Steppers live in a wrapped row pinned to the bottom, so on a
        // narrow screen they fold into several rows instead of running
        // past the right edge.
        self.controls.place().b(10).lr(10).all(8).all_wrap();

        self.threshold_view = self.controls.add_view::<NumberView>();
        self.threshold_view
            .set_color(WHITE)
            .set_value(124.0)
            .set_step(2.0)
            .add_label("there")
            .on_change(update_image)
            .place()
            .size(90, 120);

        self.x_view = self.controls.add_view::<NumberView>();
        self.x_view
            .set_color(WHITE)
            .set_value(65.0)
            .set_step(0.5)
            .add_label("x")
            .on_change(update_image)
            .place()
            .size(90, 120);

        self.y_view = self.controls.add_view::<NumberView>();
        self.y_view
            .set_color(WHITE)
            .set_value(8.0)
            .set_step(0.5)
            .add_label("y")
            .on_change(update_image)
            .place()
            .size(90, 120);

        self.size_view = self.controls.add_view::<NumberView>();
        self.size_view
            .set_color(WHITE)
            .set_value(6.0)
            .set_step(2.0)
            .add_label("size")
            .on_change(update_image)
            .place()
            .size(90, 120);

        self.skip_view = self.controls.add_view::<NumberView>();
        self.skip_view
            .set_color(WHITE)
            .set_min(1.0)
            .set_step(1.0)
            .set_value(6.0)
            .add_label("size")
            .on_change(update_image)
            .place()
            .size(90, 120);

        self.update_level.set_text("Level");
        self.update_level
            .place()
            .anchor(Top, self.counter_label, 10)
            .same([Anchor::Width, Anchor::Height, X], self.counter_label);
        self.update_level.on_tap(move || {
            self.update_level();
        });

        self.image_view.place().size(200, 200).tr(0);

        add_back_button(self);

        self.counter_label.place().t(70).l(20).size(90, 40);

        self.polygon.place().size(800, 800).center_x();

        self.update_image();
    }
}

fn generate_image(
    TerrainParams {
        seed,
        resolution,
        size,
        position,
        threshold,
        skip,
    }: TerrainParams,
) -> (Weak<Image>, Vec<Vec<Point>>) {
    let TerrainData { pixels, islands } = generate_terrain(TerrainParams {
        seed,
        resolution,
        size,
        position,
        threshold,
        skip,
    });

    let image_name = format!("noise_image_{seed}_{resolution}_{size}_{position}_{threshold}");

    (
        Image::from_raw_data(
            pixels,
            image_name,
            (resolution.width, resolution.height).into(),
            1,
        ),
        islands,
    )
}
