use std::{
    fs::{OpenOptions, create_dir_all},
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use hilen::{
    Platform,
    audio::Sound,
    dispatch::{after, spawn},
    filesystem::Paths,
    level::LevelManager,
    net::{System, local_ip},
    refs::{Weak, manage::DataManager},
    ui::{
        ALL_VIEWS, AfterSetup, Alert, Anchor, Button, Container, Font, InfiniteScrollTest, Label, Point,
        ScrollView, Setup, Spinner, TextAlignment, UIManager, ViewData, ViewSubviews, all_views, view,
    },
    ui_test::run_all_tests,
};

#[cfg(feature = "bench")]
use crate::interface::dev::UIBenchmarkView;
use crate::{
    api::TEST_REST_REQUEST,
    interface::{
        game_view::GameView,
        home_view::HomeView,
        noise_view::NoiseView,
        palette::{BORDER, SURFACE, TEXT, TEXT_DIM},
        polygon_view::PolygonView,
        render_view::RenderView,
        root_layout_view::RootLayoutView,
        scenes::{FrostedHud, GameScene, HEADER_HEIGHT, Scene3D, add_title},
    },
    levels::BenchmarkLevel,
    no_physics::NoPhysicsView,
};

/// A flat dev launcher. A title over a scrollable set of labelled
/// sections, each a wrapped row of buttons that fire one dev action, so
/// every button stays reachable on small screens.
#[view]
pub struct MenuView {
    #[init]
    status: Label,
    scroll: ScrollView,
}

impl Setup for MenuView {
    fn setup(self: Weak<Self>) {
        add_title(
            self,
            "Dev",
            "The raw engine menu. Every button fires one dev action.",
        );

        let ip = local_ip().map_or_else(|_| "no ip".to_string(), |ip| ip.to_string());
        self.status
            .set_text(format!("{ip}   {}", UIManager::app_instance_id()))
            .set_text_color(TEXT_DIM)
            .set_text_size(10)
            .set_alignment(TextAlignment::Right);
        self.status.place().r(20).t(30).size(160, 20);

        self.scroll.place().t(HEADER_HEIGHT).lrb(0);

        let scenes = self.scenes();
        let ui = self.ui(scenes);
        let level = self.level(ui);
        self.system(level);
    }
}

impl MenuView {
    fn scenes(self: Weak<Self>) -> Weak<Container> {
        let scenes = self.section(None, "SCENES");
        Self::button(scenes, "Main level", || UIManager::set_view(GameScene::new()));
        Self::button(scenes, "Frosted HUD", || UIManager::set_view(FrostedHud::new()));
        Self::button(scenes, "3D scene", || {
            LevelManager::stop_level();
            UIManager::set_view(Scene3D::new());
        });
        Self::button(scenes, "Polygon", || UIManager::set_view(PolygonView::new()));
        Self::button(scenes, "Noise", || {
            LevelManager::stop_level();
            UIManager::set_view(NoiseView::new().on_back(|| {
                UIManager::set_view(Self::new());
            }));
        });
        Self::button(scenes, "Render", || {
            LevelManager::stop_level();
            UIManager::set_view(RenderView::new());
        });
        Self::button(scenes, "No physics", || UIManager::set_view(NoPhysicsView::new()));
        Self::button(scenes, "Root layout", || {
            LevelManager::stop_level();
            UIManager::set_view(RootLayoutView::new());
        });
        Self::button(scenes, "Empty game", || {
            LevelManager::stop_level();
            UIManager::set_view(GameView::new());
        });
        scenes
    }

    fn ui(self: Weak<Self>, anchor: Weak<Container>) -> Weak<Container> {
        let ui = self.section(Some(anchor), "UI");
        #[cfg(feature = "bench")]
        Self::button(ui, "UI bench", || {
            LevelManager::stop_level();
            UIManager::set_view(UIBenchmarkView::new());
        });
        Self::button(ui, "Run UI tests", || {
            // Never on the main thread. The tests drive the main thread through
            // `from_main`, so running them on it deadlocks on the first one.
            // `run_all_tests` puts the app's root view back when it is done.
            spawn(async {
                let report = run_all_tests();

                let text = if report.failures.is_empty() {
                    format!("{} tests, all passed", report.total)
                } else {
                    let names: Vec<&str> = report.failures.iter().map(|f| f.name.as_str()).collect();
                    format!(
                        "{} tests, {} failed:\n{}",
                        report.total,
                        report.failures.len(),
                        names.join("\n")
                    )
                };

                Alert::show(text);
            });
        });
        Self::button(ui, "Alert", || {
            Alert::show("Hello!");
        });
        Self::button(ui, "Sound", || Sound::get("retro.wav").play());
        Self::button(ui, "Spinner", || {
            let spin = Spinner::lock();
            after(2.0, move || {
                spin.animated_stop();
            });
        });
        Self::button(ui, "Pick folder", || {
            spawn(async {
                Alert::show(format!("{:?}", Paths::pick_folder().await));
            });
        });
        Self::button(ui, "Scroll test", || {
            let view = InfiniteScrollTest::new().after_setup(|mut v| {
                v.add_view::<Button>()
                    .set_text("Back")
                    .on_tap(|| UIManager::set_view(HomeView::new()))
                    .place()
                    .size(100, 20);
                v.table.place().clear().back();
                v.table.set_columns(4);
            });
            LevelManager::stop_level();
            UIManager::set_view(view);
        });
        Self::button(ui, "UI 1x", || UIManager::set_scale(1.0));
        Self::button(ui, "UI 2x", || UIManager::set_scale(2.0));
        ui
    }

    fn level(self: Weak<Self>, anchor: Weak<Container>) -> Weak<Container> {
        let level = self.section(Some(anchor), "LEVEL");
        Self::button(level, "Benchmark", || {
            *LevelManager::camera_pos() = Point::default();
            LevelManager::set_level(BenchmarkLevel::default());
        });
        Self::button(level, "Level 1x", || LevelManager::set_scale(1.0));
        Self::button(level, "Level 2x", || LevelManager::set_scale(2.0));
        level
    }

    fn system(self: Weak<Self>, anchor: Weak<Container>) {
        let system = self.section(Some(anchor), "SYSTEM");
        Self::button(system, "System info", || {
            Alert::with_label(|l| {
                l.set_text_size(15);
            })
            .show(System::get_info().dump());
        });
        if Platform::IOS {
            Self::button(system, "Cloud", write_cloud_data);
        }
        Self::button(system, "REST request", move || {
            spawn(async move {
                self.rest_pressed().await.unwrap();
            });
        });
        Self::button(system, "All views", || {
            dbg!(all_views!());
            dbg!(ALL_VIEWS);
            for (name, test) in hilen::UI_TESTS.lock().iter() {
                println!("{name}: {}", test.file);
            }
        });
        // A public web page should not offer a crash button.
        #[cfg(not_wasm)]
        Self::button(system, "Panic", || panic!("test panic"));
    }

    /// Adds a section header into the scroll and returns the wrapped
    /// button row below it, to feed the next section as its anchor. The
    /// first section passes no anchor and pins to the top.
    fn section(self: Weak<Self>, anchor: Option<Weak<Container>>, title: &str) -> Weak<Container> {
        let label = self.scroll.add_view::<Label>();
        label
            .set_text(title)
            .set_text_color(TEXT_DIM)
            .set_text_size(13)
            .set_font(Font::get("RussoOne-Regular.ttf"))
            .set_alignment(TextAlignment::Left);
        match anchor {
            Some(anchor) => label.place().anchor(Anchor::Top, anchor, 16),
            None => label.place().t(4),
        }
        .lr(28)
        .h(20);

        let grid = self.scroll.add_view::<Container>();
        // Anchor only the top to the header. below() would also copy the
        // header width, so lr could no longer inset the row.
        grid.place().anchor(Anchor::Top, label, 6).lr(22).all(6).all_wrap();
        grid
    }

    fn button<Ret>(grid: Weak<Container>, title: &str, mut action: impl FnMut() -> Ret + Send + 'static) {
        let button = grid.add_view::<Button>();
        button
            .set_text(title)
            .set_color(SURFACE)
            .set_text_color(TEXT)
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(BORDER);
        button.on_tap(move || {
            action();
        });
        button.place().size(132, 38);
    }

    async fn rest_pressed(self: Weak<Self>) -> Result<()> {
        let spin = Spinner::lock();

        let users = TEST_REST_REQUEST.await?;

        spin.stop();

        Alert::show(format!(
            "Got {} users. First name: {}",
            users.len(),
            users.first().unwrap().name
        ));

        Ok(())
    }
}

fn write_cloud_data() {
    let Some(path) = UIManager::cloud_storage_dir() else {
        Alert::show("No path!");
        return;
    };

    let path = path.to_string_lossy();
    let path = path.trim_start_matches("file://");

    let mut path = PathBuf::from(path);

    // iCloud only syncs files that live inside this Documents subfolder.
    path.push("Documents");

    if !path.exists() {
        create_dir_all(&path).unwrap();
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path.join("data.txt"))
        .unwrap();

    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    let mut number: i32 = content.parse().unwrap_or_default();
    number += 1;

    file.write_all(number.to_string().as_bytes()).unwrap();

    Alert::show(format!("{}", path.display()));
}
