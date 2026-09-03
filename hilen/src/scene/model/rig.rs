use crate::gm::volume::{Mat4, Quat, Vec3, Vec4};

/// The node tree of a model and what moves it: every node's rest
/// transform and parent, the skins over the tree and the animation
/// clips. Built only for a model with a skin or a clip, a static model
/// keeps its parts flattened and never walks the tree again.
#[derive(Debug)]
pub(crate) struct Rig {
    pub nodes: Vec<RigNode>,
    /// Node indices with every parent before its children, so one pass
    /// over it composes the model space transforms.
    pub order: Vec<usize>,
    pub skins: Vec<Skin>,
    pub clips: Vec<Clip>,
}

/// One node at rest, as translation, rotation and scale, what a clip
/// overrides channel by channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RigNode {
    pub parent:      Option<usize>,
    pub translation: Vec3,
    pub rotation:    Quat,
    pub scale:       Vec3,
}

#[derive(Debug)]
pub(crate) struct Skin {
    /// The nodes that are the joints, in the skin's order, what a
    /// vertex's joint index names.
    pub joints:       Vec<usize>,
    /// From model space at bind time into each joint's space.
    pub inverse_bind: Vec<Mat4>,
}

/// One animation of a model, named as in the file.
#[derive(Debug)]
pub struct Clip {
    pub name:            String,
    /// Seconds from the first key to the last.
    pub duration:        f32,
    pub(crate) channels: Vec<Channel>,
}

/// One property of one node over time.
#[derive(Debug)]
pub(crate) struct Channel {
    pub node:          usize,
    /// Key times in seconds, ascending.
    pub times:         Vec<f32>,
    pub interpolation: Interpolation,
    pub track:         Track,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Interpolation {
    Step,
    Linear,
    /// Three values per key: the in tangent, the value and the out
    /// tangent, the glTF layout.
    CubicSpline,
}

#[derive(Debug)]
pub(crate) enum Track {
    Translation(Vec<Vec3>),
    Rotation(Vec<Quat>),
    Scale(Vec<Vec3>),
}

impl Rig {
    /// Every node's model space transform, at rest or with `clip`
    /// sampled at `time` seconds.
    pub fn pose(&self, clip: Option<(&Clip, f32)>) -> Vec<Mat4> {
        let mut local = self.nodes.clone();
        if let Some((clip, time)) = clip {
            for channel in &clip.channels {
                channel.apply(&mut local[channel.node], time);
            }
        }

        let mut globals = vec![Mat4::IDENTITY; self.nodes.len()];
        for &index in &self.order {
            let node = local[index];
            let matrix = Mat4::from_scale_rotation_translation(node.scale, node.rotation, node.translation);
            globals[index] = match node.parent {
                Some(parent) => globals[parent] * matrix,
                None => matrix,
            };
        }
        globals
    }
}

impl Skin {
    /// What takes a vertex of this skin from its bind pose to model
    /// space, one matrix per joint, with the tree posed as `globals`.
    pub fn joint_matrices(&self, globals: &[Mat4]) -> Vec<Mat4> {
        self.joints
            .iter()
            .zip(&self.inverse_bind)
            .map(|(&joint, inverse)| globals[joint] * *inverse)
            .collect()
    }
}

impl Channel {
    fn apply(&self, node: &mut RigNode, time: f32) {
        let (prev, next, t) = self.segment(time);
        let dt = self.times[next] - self.times[prev];
        match &self.track {
            Track::Translation(values) => node.translation = self.sample(values, prev, next, t, dt),
            Track::Scale(values) => node.scale = self.sample(values, prev, next, t, dt),
            Track::Rotation(values) => {
                node.rotation = match self.interpolation {
                    Interpolation::Step => values[prev],
                    Interpolation::Linear => values[prev].slerp(values[next], t),
                    Interpolation::CubicSpline => {
                        let raw: Vec<Vec4> = values.iter().map(|q| Vec4::from(*q)).collect();
                        Quat::from_vec4(self.sample(&raw, prev, next, t, dt)).normalize()
                    }
                };
            }
        }
    }

    /// The keys around `time` and how far between them, 0 to 1. Before
    /// the first key or after the last the nearest key alone.
    fn segment(&self, time: f32) -> (usize, usize, f32) {
        let times = &self.times;
        let last = times.len() - 1;
        if time <= times[0] {
            return (0, 0, 0.0);
        }
        if time >= times[last] {
            return (last, last, 0.0);
        }
        let next = times.partition_point(|key| *key <= time);
        let prev = next - 1;
        (prev, next, (time - times[prev]) / (times[next] - times[prev]))
    }

    fn sample<V: Lerp>(&self, values: &[V], prev: usize, next: usize, t: f32, dt: f32) -> V {
        match self.interpolation {
            Interpolation::Step => values[prev],
            Interpolation::Linear => values[prev].lerp(values[next], t),
            Interpolation::CubicSpline => {
                if prev == next {
                    return values[prev * 3 + 1];
                }
                let p0 = values[prev * 3 + 1];
                let m0 = values[prev * 3 + 2].scale(dt);
                let p1 = values[next * 3 + 1];
                let m1 = values[next * 3].scale(dt);
                let t2 = t * t;
                let t3 = t2 * t;
                p0.scale(2.0 * t3 - 3.0 * t2 + 1.0)
                    .add(m0.scale(t3 - 2.0 * t2 + t))
                    .add(p1.scale(-2.0 * t3 + 3.0 * t2))
                    .add(m1.scale(t3 - t2))
            }
        }
    }
}

/// The little arithmetic a sampled value needs, over `Vec3` and the
/// four components of a rotation.
trait Lerp: Copy {
    fn lerp(self, other: Self, t: f32) -> Self;
    fn scale(self, by: f32) -> Self;
    fn add(self, other: Self) -> Self;
}

impl Lerp for Vec3 {
    fn lerp(self, other: Self, t: f32) -> Self {
        Vec3::lerp(self, other, t)
    }

    fn scale(self, by: f32) -> Self {
        self * by
    }

    fn add(self, other: Self) -> Self {
        self + other
    }
}

impl Lerp for Vec4 {
    fn lerp(self, other: Self, t: f32) -> Self {
        Vec4::lerp(self, other, t)
    }

    fn scale(self, by: f32) -> Self {
        self * by
    }

    fn add(self, other: Self) -> Self {
        self + other
    }
}

#[cfg(test)]
mod test {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    fn node(parent: Option<usize>, translation: Vec3) -> RigNode {
        RigNode {
            parent,
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    fn translation(interpolation: Interpolation, times: &[f32], values: &[Vec3]) -> Channel {
        Channel {
            node: 0,
            times: times.to_vec(),
            interpolation,
            track: Track::Translation(values.to_vec()),
        }
    }

    fn sampled(channel: &Channel, time: f32) -> Vec3 {
        let mut node = node(None, Vec3::ZERO);
        channel.apply(&mut node, time);
        node.translation
    }

    #[test]
    fn linear_keys_blend_and_clamp_at_the_ends() {
        let channel = translation(
            Interpolation::Linear,
            &[1.0, 2.0, 4.0],
            &[Vec3::ZERO, Vec3::X, Vec3::X * 3.0],
        );
        assert_eq!(sampled(&channel, 0.0), Vec3::ZERO);
        assert_eq!(sampled(&channel, 1.5), Vec3::X * 0.5);
        assert_eq!(sampled(&channel, 3.0), Vec3::X * 2.0);
        assert_eq!(sampled(&channel, 9.0), Vec3::X * 3.0);
    }

    #[test]
    fn step_keys_hold_until_the_next() {
        let channel = translation(Interpolation::Step, &[0.0, 1.0], &[Vec3::ZERO, Vec3::Y]);
        assert_eq!(sampled(&channel, 0.99), Vec3::ZERO);
        assert_eq!(sampled(&channel, 1.0), Vec3::Y);
    }

    // With zero tangents the spline is the smoothstep between the keys,
    // so the midpoint is the average and the quarter point is below it.
    #[test]
    fn cubic_keys_ease_between_values() {
        let channel = translation(
            Interpolation::CubicSpline,
            &[0.0, 2.0],
            &[
                Vec3::ZERO,
                Vec3::ZERO,
                Vec3::ZERO,
                Vec3::ZERO,
                Vec3::X * 4.0,
                Vec3::ZERO,
            ],
        );
        assert_eq!(sampled(&channel, 0.0), Vec3::ZERO);
        assert_eq!(sampled(&channel, 1.0), Vec3::X * 2.0);
        assert!(sampled(&channel, 0.5).x < 1.0);
        assert_eq!(sampled(&channel, 2.0), Vec3::X * 4.0);
        assert_eq!(sampled(&channel, 5.0), Vec3::X * 4.0);
    }

    #[test]
    fn rotations_take_the_short_way_round() {
        let channel = Channel {
            node:          0,
            times:         vec![0.0, 1.0],
            interpolation: Interpolation::Linear,
            track:         Track::Rotation(vec![Quat::IDENTITY, Quat::from_rotation_z(FRAC_PI_2)]),
        };
        let mut node = node(None, Vec3::ZERO);
        channel.apply(&mut node, 0.5);
        let turned = node.rotation * Vec3::X;
        assert!(
            (turned - Vec3::new(0.5f32.sqrt(), 0.5f32.sqrt(), 0.0)).length() < 1e-5,
            "{turned}"
        );
    }

    // A child inherits its parent's motion, and the joint matrix undoes
    // the bind pose first, so a joint at rest moves nothing.
    #[test]
    fn a_posed_child_follows_its_parent_and_rest_joints_are_identity() {
        let rig = Rig {
            nodes: vec![node(None, Vec3::ZERO), node(Some(0), Vec3::Y * 2.0)],
            order: vec![0, 1],
            skins: vec![Skin {
                joints:       vec![0, 1],
                inverse_bind: vec![Mat4::IDENTITY, Mat4::from_translation(Vec3::Y * -2.0)],
            }],
            clips: vec![Clip {
                name:     "lift".into(),
                duration: 1.0,
                channels: vec![translation(
                    Interpolation::Linear,
                    &[0.0, 1.0],
                    &[Vec3::ZERO, Vec3::X],
                )],
            }],
        };

        let rest = rig.pose(None);
        let joints = rig.skins[0].joint_matrices(&rest);
        assert_eq!(joints, vec![Mat4::IDENTITY, Mat4::IDENTITY]);

        let lifted = rig.pose(Some((&rig.clips[0], 1.0)));
        assert_eq!(lifted[1].transform_point3(Vec3::ZERO), Vec3::new(1.0, 2.0, 0.0));
        let joints = rig.skins[0].joint_matrices(&lifted);
        // A vertex bound at the child's origin moves with the child.
        assert_eq!(
            joints[1].transform_point3(Vec3::Y * 2.0),
            Vec3::new(1.0, 2.0, 0.0)
        );
    }

    #[test]
    fn parents_come_before_children_in_any_index_order() {
        let rig = Rig {
            nodes: vec![node(Some(1), Vec3::X), node(None, Vec3::Y)],
            order: vec![1, 0],
            skins: vec![],
            clips: vec![],
        };
        let rest = rig.pose(None);
        assert_eq!(rest[0].transform_point3(Vec3::ZERO), Vec3::new(1.0, 1.0, 0.0));
    }
}
