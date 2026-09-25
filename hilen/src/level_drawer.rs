use wgpu::RenderPass;

use crate::{
    deps::refs::{main_lock::MainLock, manage::ExistsManaged},
    gm::flat::Size,
    level::{Level, LevelManager},
    render::{
        BackgroundPipeline, MAX_SPRITE_LIGHTS, PolygonPipeline, SpriteBoxPipeline, SpriteLight, SpriteView,
        TexturedSpriteBoxPipeline,
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

        Self::add_tile_maps(level);

        for sprite in level.sprites() {
            if sprite.image.exists_managed() {
                let mut instance = TexturedSpriteInstance::new(
                    sprite.position(),
                    sprite.render_size(),
                    sprite.rotation(),
                    sprite.z_position,
                )
                .flipped(sprite.flip.x, sprite.flip.y);
                instance.scale = sprite.image_scale;
                instance.tint = sprite.tint;
                TEXTURED_SPRITE_DRAWER.get_mut().add_with_image(instance, sprite.image);
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

        let view = Self::sprite_view(level, resolution);

        SPRITE_DRAWER.get_mut().draw(pass, view);
        let textured = TEXTURED_SPRITE_DRAWER.get_mut();
        textured.sort_back_to_front(|instance| instance.z_position);
        textured.draw(pass, view);
        POLYGON.get_mut().draw(pass, view);

        set_viewport(pass, UIManager::window_resolution());
    }

    /// Only the cells on screen are drawn, a long run of tiles costs what
    /// one screen of it does.
    fn add_tile_maps(level: &dyn Level) {
        let visible = LevelManager::visible_rect();
        for map in level.tile_maps() {
            for (cell, kind) in map.visible_cells(visible) {
                TEXTURED_SPRITE_DRAWER.get_mut().add_with_image(
                    TexturedSpriteInstance::new(cell.center(), cell.size / 2.0, 0.0, map.z_position),
                    kind.image,
                );
            }
        }
    }

    /// The camera and the light of the frame. With more lights than the
    /// shader takes, the ones nearest the screen win.
    fn sprite_view(level: &dyn Level, resolution: Size) -> SpriteView {
        let mut view = SpriteView {
            camera_pos: *LevelManager::camera_pos(),
            resolution,
            scale: LevelManager::scale(),
            ambient: level.ambient_light,
            ..Default::default()
        };

        let visible = LevelManager::visible_rect();
        let center = visible.center();
        let mut lights: Vec<_> = level
            .lights()
            .iter()
            .filter(|light| light.radius > 0.0 && light.intensity > 0.0)
            .map(|light| ((light.position - center).length() - light.radius, light))
            .collect();
        if lights.len() > MAX_SPRITE_LIGHTS {
            lights.sort_by(|a, b| a.0.total_cmp(&b.0));
            lights.truncate(MAX_SPRITE_LIGHTS);
        }

        for (slot, (_, light)) in view.lights.iter_mut().zip(&lights) {
            *slot = SpriteLight {
                color:    light.color.with_alpha(light.intensity),
                position: light.position,
                radius:   light.radius,
                falloff:  light.falloff,
            };
        }
        view.light_count = u32::try_from(lights.len()).expect("at most MAX_SPRITE_LIGHTS lights");

        view
    }
}
