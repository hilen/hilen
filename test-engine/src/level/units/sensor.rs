use std::ops::{Deref, DerefMut};

use rapier2d::{geometry::ColliderHandle, prelude::Vec2};
use refs::{Own, Weak, weak_from_ref};

use crate::{
    gm::flat::{Point, Shape},
    level::{LevelManager, Sprite, SpriteData, ToCollider},
};

/// A static collider that fires `on_collision` without blocking movement.
/// Other sprites pass through it, so it works as a trigger zone.
pub struct Sensor {
    collider_handle: ColliderHandle,
    sprite:          SpriteData,
}

impl Sprite for Sensor {
    fn make(shape: Shape, position: Point) -> Own<Self> {
        let collider = shape
            .make_collider()
            .sensor(true)
            .translation(Vec2::new(position.x, position.y))
            .build();

        let sprite = SpriteData::make(shape, position);
        let collider_handle = LevelManager::physics().sets.colliders.insert(collider);

        let mut new = Own::new(Self {
            collider_handle,
            sprite,
        });

        new.enable_collision_detection();

        new
    }

    fn collider_handle(&self) -> Option<ColliderHandle> {
        self.collider_handle.into()
    }

    fn weak_sprite(&self) -> Weak<dyn Sprite> {
        weak_from_ref(self)
    }
}

impl Deref for Sensor {
    type Target = SpriteData;

    fn deref(&self) -> &Self::Target {
        &self.sprite
    }
}

impl DerefMut for Sensor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sprite
    }
}

#[cfg(test)]
mod test {
    use hreads::set_current_thread_as_main;
    use serial_test::serial;

    use crate::{
        gm::flat::Shape,
        level::{Body, LevelCreation, LevelManager, LevelSetup, Sensor, level},
    };

    #[level]
    #[derive(Default)]
    struct SensorLevel {
        triggered: bool,
    }

    impl LevelSetup for SensorLevel {
        fn needs_physics(&self) -> bool {
            true
        }

        fn setup(&mut self) {
            let sensor = self.make_sprite::<Sensor>(Shape::rect(10, 1), (0, -5));

            sensor.on_collision.sub(|| {
                LevelManager::downcast_level::<SensorLevel>().triggered = true;
            });

            self.make_sprite::<Body>(Shape::rect(1, 1), (0, 5));
        }
    }

    #[test]
    #[serial]
    fn sensor_triggers_on_collision() {
        set_current_thread_as_main();

        let level = LevelManager::set_level(SensorLevel::default());

        // 10 simulated seconds at the default 1/60 step. The body falls
        // onto the sensor in about 1.5 of them under default gravity.
        for _ in 0..600 {
            LevelManager::update();
            if level.triggered {
                break;
            }
        }

        let triggered = level.triggered;

        LevelManager::stop_level();

        assert!(triggered, "Body never triggered the sensor");
    }
}
