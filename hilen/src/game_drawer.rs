use crate::{
    deps::refs::main_lock::MainLock,
    game::{Game, Shape},
    gm::flat::Point,
    render::{BackgroundPipeline, SpriteView, TexturedSpriteBoxPipeline, data::TexturedSpriteInstance},
    ui::{UIManager, ui_drawer::set_viewport},
    window::RenderPass,
};

static OBJECT_DRAWER: MainLock<TexturedSpriteBoxPipeline> = MainLock::new();
static BACKGROUND: MainLock<BackgroundPipeline> = MainLock::new();

/// Drops what is built for a sample count. Each part is made again when
/// it is next used.
pub(crate) fn drop_pipelines() {
    OBJECT_DRAWER.clear();
    BACKGROUND.clear();
}

pub struct GameDrawer;

impl GameDrawer {
    pub fn draw(pass: &mut RenderPass, game: &mut Game) {
        game.update();

        let area = UIManager::render_area();
        set_viewport(pass, area);

        BACKGROUND.get_mut().draw(pass, &game.skybox, area, Point::default(), 0.0, 1.0);

        for object in &game.objects {
            if let Shape::Rect(size) = object.shape {
                OBJECT_DRAWER.get_mut().add_with_image(
                    TexturedSpriteInstance::new(object.position, size, object.rotation, 0.85),
                    object.texture,
                );
            }
        }

        OBJECT_DRAWER.get_mut().draw(
            pass,
            SpriteView {
                resolution: area,
                ..Default::default()
            },
        );

        set_viewport(pass, UIManager::window_resolution());
    }
}
