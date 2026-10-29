use rapier3d::prelude::Pose3;

use crate::{
    gm::volume::{Quat, Vec3},
    scene::{LightKind, Node, NodeTemplates, SceneBase, follow_parent},
};

/// A rigid move of a whole scene: a point `p` lands on `turn * p + shift`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldMove {
    pub turn:  Quat,
    pub shift: Vec3,
}

impl WorldMove {
    pub fn point(&self, point: Vec3) -> Vec3 {
        self.turn * point + self.shift
    }

    /// A direction or a speed, which only turns.
    pub fn direction(&self, direction: Vec3) -> Vec3 {
        self.turn * direction
    }

    pub fn rotation(&self, rotation: Quat) -> Quat {
        (self.turn * rotation).normalize()
    }

    /// The move that undoes this one.
    pub fn inverse(&self) -> Self {
        let turn = self.turn.inverse();
        Self {
            turn,
            shift: -(turn * self.shift),
        }
    }
}

impl SceneBase {
    /// Moves and turns everything in the scene at once by `world`: every
    /// node with its speed and spin, the player, the camera, gravity and
    /// its wells, the sun, the lights, the sky's sun and moon, the lines,
    /// the wind and the handle.
    /// A world bigger than 32 bit floats hold keeps its own 64 bit places
    /// and moves the scene back near the origin when the player went far,
    /// the floating origin of a space game. Nothing wakes and no contact
    /// breaks, a body keeps rolling where it rolled.
    ///
    /// An attached node keeps its place on its parent. A field of grass
    /// moves its own axes along, its patches and its `ground` stay. The cube
    /// `sky` stays, it has no place, so a turn about any axis but up turns
    /// the scene against it.
    pub fn move_world(&mut self, world: WorldMove) {
        for node in &mut self.nodes {
            move_node(&mut **node, world);
        }
        for node in &mut self.nodes {
            follow_parent(&mut **node);
        }
        if let (Some(player), Some(physics)) = (self.player.as_mut(), self.physics.as_mut()) {
            player.move_world(physics, world);
        }
        if let Some(physics) = self.physics.as_mut() {
            physics.gravity = world.direction(physics.gravity);
            for well in &mut physics.wells {
                well.center = world.point(well.center);
            }
        }

        let camera = &mut self.camera;
        camera.position = world.point(camera.position);
        camera.target = world.point(camera.target);
        camera.up = world.direction(camera.up);

        self.sun.direction = world.direction(self.sun.direction);
        for light in &mut self.lights {
            light.position = world.point(light.position);
            if let LightKind::Spot { direction, .. } | LightKind::Directional { direction, .. } =
                &mut light.kind
            {
                *direction = world.direction(*direction);
            }
        }
        if let Some(sky) = self.day_night_sky.as_mut() {
            sky.sun = world.direction(sky.sun);
            sky.moon = world.direction(sky.moon);
        }
        if let Some(space) = self.space_sky.as_mut() {
            space.stars = world.rotation(space.stars);
            if let Some(air) = space.atmosphere.as_mut() {
                air.center = world.point(air.center);
            }
        }
        for occluder in &mut self.occluders {
            occluder.center = world.point(occluder.center);
        }
        for line in &mut self.lines {
            line.from = world.point(line.from);
            line.to = world.point(line.to);
        }
        self.wind.direction = world.direction(self.wind.direction);
        if let Some(handle) = self.handle.as_mut() {
            handle.position = world.point(handle.position);
            handle.rotation = world.rotation(handle.rotation);
        }
        self.grass_pusher = self.grass_pusher.map(|feet| world.point(feet));
        self.grass_trail = self.grass_trail.map(|trail| world.point(trail));
        for grass in &mut self.grass {
            grass.origin = world.point(grass.origin);
            grass.turn = world.rotation(grass.turn);
        }
    }
}

fn move_node(node: &mut dyn Node, world: WorldMove) {
    if node.parent().is_some() {
        return;
    }
    let position = world.point(node.position());
    let rotation = world.rotation(node.rotation());
    if node.rigid_handle().is_none() {
        node.set_rotation(rotation).set_position(position);
        return;
    }
    let body = node.rigid_body_mut();
    let speed = body.linvel();
    let spin = body.angvel();
    let pose = Pose3::from_parts(position, rotation);
    body.set_position(pose, false);
    if body.is_kinematic() {
        body.set_next_kinematic_position(pose);
    }
    body.set_linvel(world.direction(Vec3::new(speed.x, speed.y, speed.z)), false);
    body.set_angvel(world.direction(Vec3::new(spin.x, spin.y, spin.z)), false);
    node.position = position;
    node.rotation = rotation;
}

#[cfg(test)]
mod test {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn a_move_turns_then_shifts() {
        let world = WorldMove {
            turn:  Quat::from_rotation_y(FRAC_PI_2),
            shift: Vec3::new(10.0, 0.0, 0.0),
        };
        assert!(close(world.point(Vec3::X), Vec3::new(10.0, 0.0, -1.0)));
        assert!(
            close(world.direction(Vec3::X), Vec3::NEG_Z),
            "a speed is not shifted"
        );
    }

    #[test]
    fn the_inverse_puts_every_point_back() {
        let world = WorldMove {
            turn:  Quat::from_rotation_x(0.3) * Quat::from_rotation_y(-1.2) * Quat::from_rotation_z(0.7),
            shift: Vec3::new(-1500.0, 20.0, 800.0),
        };
        let back = world.inverse();
        for point in [
            Vec3::ZERO,
            Vec3::new(3.0, -4.0, 12.0),
            Vec3::new(-700.0, 15.0, 90.0),
        ] {
            // A float steps 0.12 mm at 1500 m, the way there and back
            // adds a few of those.
            let error = (back.point(world.point(point)) - point).length();
            assert!(error < 1e-3, "{point} came back {error} m off");
        }
        let rotation = Quat::from_rotation_x(0.4);
        assert!(back.rotation(world.rotation(rotation)).angle_between(rotation) < 1e-5);
    }
}
