use std::ops::Deref;

use educe::Educe;
use rapier2d::prelude::Vec2;

use crate::{
    deps::{
        refs::{Own, Weak},
        vents::Event,
    },
    gm::{
        color::{Color, WHITE},
        flat::Point,
    },
    level::{Level, LevelManager, Light, Sprite, TileMap, level::level_physics::LevelPhysics},
    window::image::Image,
};

#[derive(Educe)]
#[educe(Default)]
pub struct LevelBase {
    pub(crate) sprites: Vec<Own<dyn Sprite>>,

    pub(crate) lights:    Vec<Own<Light>>,
    pub(crate) tile_maps: Vec<Own<TileMap>>,

    pub background: Weak<Image>,

    /// The light every sprite gets with no point light near it. White
    /// draws the level unlit, a dark gray makes a cave that only the
    /// lights show.
    #[educe(Default = WHITE)]
    pub ambient_light: Color,

    pub cursor_position: Point,

    pub on_tap:             Event<Point>,
    pub on_sprite_selected: Event<Weak<dyn Sprite>>,

    #[educe(Default = LevelManager::default_z_position())]
    pub(crate) last_z_pos: f32,

    pub(crate) physics: Option<LevelPhysics>,
}

impl LevelBase {
    /// Physics steps per level step. Rapier's CCD covers a fast body
    /// hitting a wall, but not a fast kinematic wall sweeping into a slow
    /// body, that one only stays correct when a step moves the wall less
    /// than the body is wide. The level sets its kinematic poses once per
    /// substep, so the wall moves in hops of a 480th of a second at the
    /// default level step instead of one jump.
    pub const PHYSICS_SUBSTEPS: usize = 2;

    pub fn has_physics(&self) -> bool {
        self.physics.is_some()
    }

    pub fn init_physics(&mut self) {
        assert!(self.physics.is_none(), "Double init_physics");
        self.physics = LevelPhysics::default().into();
    }

    pub fn update_physics(&mut self, frame_time: f32) {
        if let Some(physics) = self.physics.as_mut() {
            physics.update_physics(&self.sprites, frame_time);
        }
    }

    pub fn remove(&mut self, sprite: Weak<dyn Sprite>) {
        let index = self.sprites.iter().position(|a| a.raw() == sprite.raw()).unwrap();

        let sprite = self.sprites[index].deref();

        if let Some(physics) = self.physics.as_mut() {
            physics.remove(sprite);
        }
        self.sprites.remove(index);
    }

    pub fn remove_all_sprites(&mut self) {
        if let Some(physics) = &mut self.physics {
            for sprite in self.sprites.drain(..) {
                physics.remove(sprite.deref());
            }
        } else {
            self.sprites.clear();
        }
    }
}

pub trait LevelTemplates {
    fn set_gravity(&mut self, g: impl Into<Point>);
}

impl<T: ?Sized + Level> LevelTemplates for T {
    fn set_gravity(&mut self, g: impl Into<Point>) {
        let g = g.into();
        if let Some(physics) = self.physics.as_mut() {
            physics.gravity = Vec2::new(g.x, g.y);
        }
    }
}
