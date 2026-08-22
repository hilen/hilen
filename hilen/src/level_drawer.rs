use wgpu::RenderPass;

use crate::{
    deps::refs::{main_lock::MainLock, manage::ExistsManaged},
    level::LevelManager,
    render::{
        BackgroundPipeline, PolygonPipeline, SpriteBoxPipeline, SpriteView, TexturedSpriteBoxPipeline,
        data::{SpriteInstance, TexturedSpriteInstance},
    },
    ui::{UIManager, ui_drawer::set_viewport},
};

static SPRITE_DRAWER: MainLock<SpriteBoxPipeline> = MainLock::new();
static TEXTURED_SPRITE_DRAWER: MainLock<TexturedSpriteBoxPipeline> = MainLock::new();
static BACKGROUND: MainLock<BackgroundPipeline> = MainLock::new();
static POLYGON: MainLock<PolygonPipeline> = MainLock::new();

pub(crate) struct LevelDrawer;

impl LevelDrawer {
    pub(crate) fn update() {
        LevelManager::update();

        // A running level animates every frame, so keep the loop awake while
        // one is loaded. A plain UI screen has no level and stays idle.
        if !LevelManager::no_level() {
            crate::window::request_frame();
        }
    }

    pub fn draw(pass: &mut RenderPass) {
        if LevelManager::no_level() {
            return;
        }
        let resolution = UIManager::render_area();
        set_viewport(pass, resolution);

        let level = LevelManager::level();
        let camera_pos = *LevelManager::camera_pos();
        let scale = LevelManager::scale();

        if level.background.is_ok() {
            BACKGROUND.get_mut().draw(
                pass,
                &level.background,
                resolution,
                camera_pos.neg() / 10.0,
                0.0,
                scale,
            );
        }

        POLYGON.get_mut().clear();

        for sprite in level.sprites() {
            if sprite.image.exists_managed() {
                TEXTURED_SPRITE_DRAWER.get_mut().add_with_image(
                    TexturedSpriteInstance {
                        size:       sprite.render_size(),
                        scale:      sprite.image_scale,
                        position:   sprite.position(),
                        rotation:   sprite.rotation(),
                        z_position: sprite.z_position,
                    },
                    sprite.image,
                );
            } else if let Some(vertex_buffer) = &sprite.vertex_buffer {
                POLYGON.get_mut().add(
                    vertex_buffer,
                    sprite.position(),
                    *sprite.color(),
                    sprite.rotation(),
                );
            } else {
                SPRITE_DRAWER.get_mut().add(SpriteInstance {
                    size:       sprite.render_size(),
                    position:   sprite.position(),
                    color:      *sprite.color(),
                    rotation:   sprite.rotation(),
                    z_position: sprite.z_position,
                });
            }
        }

        SPRITE_DRAWER.get_mut().draw(
            pass,
            SpriteView {
                camera_pos,
                resolution,
                camera_rotation: 0.0,
                scale,
                _padding: 0,
            },
        );
        TEXTURED_SPRITE_DRAWER.get_mut().draw(
            pass,
            SpriteView {
                camera_pos,
                resolution,
                camera_rotation: 0.0,
                scale,
                _padding: 0,
            },
        );

        POLYGON.get_mut().draw(
            pass,
            SpriteView {
                camera_pos,
                resolution,
                camera_rotation: 0.0,
                scale,
                _padding: 0,
            },
        );

        set_viewport(pass, UIManager::window_resolution());
    }
}
