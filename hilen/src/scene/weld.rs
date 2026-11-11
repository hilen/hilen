use rapier3d::{
    dynamics::{FixedJointBuilder, ImpulseJointHandle, RigidBodyHandle},
    geometry::ColliderHandle,
};

use crate::{
    deps::refs::Weak,
    gm::{LossyConvert, volume::Vec3},
    scene::{Node, SceneManager, scene::ScenePhysics},
};

/// 2 bodies held together as 1 rigid thing, in the pose they have to each
/// other when it is made, the grip of a hand on a stone. It lasts until
/// `release` or until 1 of the bodies leaves the scene.
pub struct Weld {
    handle: ImpulseJointHandle,
}

impl Weld {
    /// `None` when a node has no body of its own, like a `Prop`.
    pub fn new(a: Weak<dyn Node>, b: Weak<dyn Node>) -> Option<Self> {
        let handle = SceneManager::physics().weld(a.rigid_handle()?, b.rigid_handle()?);
        Some(Self { handle })
    }

    /// The impulse the weld needed in the last step to keep the bodies
    /// together, in N s, so a game can let go of a thing that is hit too
    /// hard. 0 once a body is gone.
    pub fn strain(&self) -> f32 {
        SceneManager::physics().weld_strain(self.handle)
    }

    pub fn release(self) {
        SceneManager::physics().release_weld(self.handle);
    }
}

/// Where 2 nodes touch, see `Node::contact_with`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    /// The deepest point of the touch, in the world.
    pub point:   Vec3,
    /// From the node asked toward the other one.
    pub normal:  Vec3,
    /// What the last step pushed the 2 apart with, in N s. Over the mass
    /// it is the change of speed of a blow.
    pub impulse: f32,
}

impl ScenePhysics {
    pub(crate) fn weld(&mut self, a: RigidBodyHandle, b: RigidBodyHandle) -> ImpulseJointHandle {
        let bodies = &self.sets.rigid_bodies;
        // The frame of the joint is the frame of `a`, seen from each body.
        let from_b = bodies[b].position().inverse() * *bodies[a].position();
        // The 2 lie against each other, as contacts they would push apart
        // what the weld holds together.
        let joint = FixedJointBuilder::new().local_frame2(from_b).contacts_enabled(false).build();
        self.impulse_joints.insert(a, b, joint, true)
    }

    pub(crate) fn weld_strain(&self, weld: ImpulseJointHandle) -> f32 {
        // Rapier keeps the impulse of 1 of the smaller steps it solves a
        // step in.
        let parts: f32 = self.integration_parameters.num_solver_iterations.lossy_convert();
        self.impulse_joints.get(weld).map_or(0.0, |joint| {
            let [x, y, z, ..] = joint.impulses;
            Vec3::new(x, y, z).length() * parts
        })
    }

    pub(crate) fn release_weld(&mut self, weld: ImpulseJointHandle) {
        if self.impulse_joints.remove(weld, true).is_none() {
            log::debug!("a weld was released after 1 of its bodies left the scene");
        }
    }

    pub(crate) fn contact(&self, a: ColliderHandle, b: ColliderHandle) -> Option<Contact> {
        let pair = self.narrow_phase.contact_pair(a, b)?;
        let (manifold, deepest) = pair.find_deepest_contact()?;
        let first = self.sets.colliders.get(pair.collider1)?;
        let normal = manifold.data.normal;
        Some(Contact {
            point:   first.position().transform_point(deepest.local_p1),
            normal:  if pair.collider1 == a { normal } else { -normal },
            impulse: pair.total_impulse_magnitude(),
        })
    }
}

#[cfg(test)]
mod test {
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    use super::*;

    const DT: f32 = 1.0 / 240.0;

    fn ball(physics: &mut ScenePhysics, at: Vec3, falls: bool) -> (RigidBodyHandle, ColliderHandle) {
        let body = RigidBodyBuilder::dynamic()
            .translation(at)
            .gravity_scale(if falls { 1.0 } else { 0.0 })
            .build();
        physics.sets.insert(body, ColliderBuilder::ball(0.1).build())
    }

    fn place(physics: &ScenePhysics, body: RigidBodyHandle) -> Vec3 {
        physics.sets.rigid_bodies[body].translation()
    }

    #[test]
    fn a_welded_body_goes_with_the_one_that_moves() {
        let mut physics = ScenePhysics::default();
        let (hand, _) = ball(&mut physics, Vec3::ZERO, false);
        let (stone, _) = ball(&mut physics, Vec3::new(0.3, 0.0, 0.0), false);
        physics.weld(hand, stone);
        for _ in 0..240 {
            physics.sets.rigid_bodies[hand].set_linvel(Vec3::new(0.0, 0.0, 1.0), true);
            physics.update_physics(&[], DT);
        }
        // The pull on the hand turns the pair, the stone lies off its
        // line, so the stone is looked at from the hand.
        let hand_pose = *physics.sets.rigid_bodies[hand].position();
        let off = hand_pose.inverse_transform_point(place(&physics, stone));
        assert!(place(&physics, hand).z > 0.5, "the hand did not move");
        assert!(
            (off - Vec3::new(0.3, 0.0, 0.0)).length() < 0.01,
            "the stone is {off} from the hand"
        );
    }

    #[test]
    fn a_weld_holds_a_weight_strains_and_lets_go() {
        let mut physics = ScenePhysics::default();
        let (hand, _) = physics.sets.insert(
            RigidBodyBuilder::fixed().build(),
            ColliderBuilder::ball(0.1).build(),
        );
        let (stone, _) = ball(&mut physics, Vec3::new(0.0, -0.3, 0.0), true);
        let weld = physics.weld(hand, stone);
        for _ in 0..120 {
            physics.update_physics(&[], DT);
        }
        assert!(
            place(&physics, stone).y > -0.31,
            "the held stone sank to {}",
            place(&physics, stone).y
        );
        // The weld carries the weight of the stone, m g dt each step.
        let weight = physics.sets.rigid_bodies[stone].mass() * 9.81 * DT;
        let strain = physics.weld_strain(weld);
        assert!(
            strain > weight * 0.9 && strain < weight * 1.1,
            "the strain is {strain}, the weight of a step {weight}"
        );

        physics.release_weld(weld);
        for _ in 0..120 {
            physics.update_physics(&[], DT);
        }
        assert!(place(&physics, stone).y < -1.0, "the stone did not fall");
        assert!(physics.weld_strain(weld).abs() < f32::EPSILON);
    }

    #[test]
    fn a_contact_tells_the_point_the_side_and_the_blow() {
        let mut physics = ScenePhysics::default();
        let ground = RigidBodyBuilder::fixed().build();
        let (_, ground) = physics.sets.insert(
            ground,
            ColliderBuilder::cuboid(5.0, 0.5, 5.0)
                .translation(Vec3::new(0.0, -0.5, 0.0))
                .build(),
        );
        let (body, stone) = ball(&mut physics, Vec3::new(1.0, 0.5, 0.0), true);
        assert_eq!(physics.contact(stone, ground), None);

        let mut hardest = 0.0_f32;
        for _ in 0..240 {
            physics.update_physics(&[], DT);
            if let Some(contact) = physics.contact(stone, ground) {
                hardest = hardest.max(contact.impulse);
            }
        }
        let contact = physics.contact(stone, ground).expect("the ball lies on the ground");
        assert!(
            (contact.point - Vec3::new(1.0, 0.0, 0.0)).length() < 0.02,
            "the touch is at {}",
            contact.point
        );
        assert!(
            contact.normal.y < -0.99,
            "from the ball the ground is at {}",
            contact.normal
        );
        let back = physics.contact(ground, stone).expect("the same touch from the other side");
        assert!(
            back.normal.y > 0.99,
            "from the ground the ball is at {}",
            back.normal
        );
        // The ball fell 0.4 m, so it landed at sqrt(2 g h) and the ground
        // took at least its momentum.
        let landing = physics.sets.rigid_bodies[body].mass() * (2.0 * 9.81 * 0.4_f32).sqrt();
        assert!(
            hardest > landing * 0.5,
            "the hardest step gave {hardest}, the landing {landing}"
        );
    }
}
