use log::error;

use crate::{
    gm::volume::{Mat4, Quat, Shape3, Vec3},
    scene::{Playback, model::Rig},
};

/// A node of a model's tree that an app places from code, a joint of a
/// skin or any other named node. `Model::joint` finds it by name once, and
/// the handle holds for every node that draws that model, see
/// `NodeTemplates::set_joint`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Joint(pub(crate) usize);

/// The translation and rotation an app set on a joint, in the space of
/// the joint's parent node, in place of the rest ones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct JointPose {
    pub translation: Vec3,
    pub rotation:    Quat,
}

impl Rig {
    /// The first node called `name`. A node without a name is never
    /// found.
    pub fn joint(&self, name: &str) -> Option<Joint> {
        if name.is_empty() {
            return None;
        }
        self.names.iter().position(|node| node == name).map(Joint)
    }

    pub fn joint_names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str).filter(|name| !name.is_empty())
    }

    pub fn joint_rest(&self, joint: Joint) -> Option<(Vec3, Quat)> {
        self.nodes.get(joint.0).map(|node| (node.translation, node.rotation))
    }

    /// Puts `pose` on `joint` in the set poses of a node, false for a
    /// handle this rig has no node for.
    pub fn set_joint(&self, set: &mut Vec<Option<JointPose>>, joint: Joint, pose: JointPose) -> bool {
        if joint.0 >= self.nodes.len() {
            return false;
        }
        if set.len() < self.nodes.len() {
            set.resize(self.nodes.len(), None);
        }
        set[joint.0] = Some(pose);
        true
    }

    /// Every node's model space transform as a node with this clip and
    /// these set joints draws it, `None` at rest, where nothing has to
    /// walk the tree.
    pub fn posed(&self, playback: Option<Playback>, set: &[Option<JointPose>]) -> Option<Vec<Mat4>> {
        if playback.is_none() && set.is_empty() {
            return None;
        }
        let clip = playback.map(|playback| (&self.clips[playback.clip], playback.time));
        Some(self.pose_with(clip, set))
    }
}

/// Takes the set pose of `joint` away. A node with none left is at rest
/// again and draws without walking the tree.
pub(crate) fn clear_joint(set: &mut Vec<Option<JointPose>>, joint: Joint) {
    if let Some(pose) = set.get_mut(joint.0) {
        *pose = None;
    }
    if set.iter().all(Option::is_none) {
        set.clear();
    }
}

/// The handle of the node called `name` in the model `shape` draws,
/// logged when there is none.
pub(crate) fn find_joint(shape: Shape3, name: &str) -> Option<Joint> {
    let joint = with_rig(shape, "joint", |rig| rig.joint(name))?;
    if joint.is_none() {
        error!("joint({name}): the model has no such node");
    }
    joint
}

/// Sets `pose` on `joint` of the model `shape` draws, logged when the
/// model has no such joint.
pub(crate) fn set_joint(shape: Shape3, set: &mut Vec<Option<JointPose>>, joint: Joint, pose: JointPose) {
    if with_rig(shape, "set_joint", |rig| rig.set_joint(set, joint, pose)) == Some(false) {
        error!("set_joint: {joint:?} is not of the model this node draws");
    }
}

/// Where `joint` is in model space with the clip and the set joints of
/// its node, logged when the model has no such joint.
pub(crate) fn joint_transform(
    shape: Shape3,
    playback: Option<Playback>,
    set: &[Option<JointPose>],
    joint: Joint,
) -> Option<Mat4> {
    let transform = with_rig(shape, "joint_transform", |rig| {
        let globals = rig.posed(playback, set).unwrap_or_else(|| rig.pose(None));
        globals.get(joint.0).copied()
    })?;
    if transform.is_none() {
        error!("joint_transform: {joint:?} is not of the model this node draws");
    }
    transform
}

/// Runs `run` on the rig of the model `shape` draws. A node without a
/// model, or a model with no skin and no clip, logs an error under the
/// name of the `call` and gives `None`.
fn with_rig<R>(shape: Shape3, call: &str, run: impl FnOnce(&Rig) -> R) -> Option<R> {
    let Shape3::Model(model) = shape else {
        error!("{call}: the node draws no model");
        return None;
    };
    if !model.is_ok() {
        error!("{call}: the model is gone");
        return None;
    }
    let Some(rig) = &model.rig else {
        error!("{call}: the model has no skin and no clip, so it keeps no joints");
        return None;
    };
    Some(run(rig))
}

#[cfg(test)]
mod test {
    use std::{f32::consts::FRAC_PI_2, fs::read, path::PathBuf};

    use super::*;
    use crate::scene::model::{
        parse::parse_glb,
        rig::{Channel, Clip, Interpolation, RigNode, Skin, Track},
    };

    const CLOSE: f32 = 1e-5;

    fn node(parent: Option<usize>, translation: Vec3) -> RigNode {
        RigNode {
            parent,
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    fn rig(nodes: Vec<RigNode>, names: &[&str], clips: Vec<Clip>) -> Rig {
        Rig {
            order: (0..nodes.len()).collect(),
            nodes,
            names: names.iter().map(ToString::to_string).collect(),
            skins: vec![Skin {
                joints:       vec![1, 2, 3],
                inverse_bind: vec![Mat4::IDENTITY; 3],
            }],
            clips,
        }
    }

    /// A root with a chain of 3 joints up the y axis, 1 apart.
    fn chain() -> Rig {
        rig(
            vec![
                node(None, Vec3::ZERO),
                node(Some(0), Vec3::Y),
                node(Some(1), Vec3::Y),
                node(Some(2), Vec3::Y),
            ],
            &["root", "a", "b", "c"],
            vec![],
        )
    }

    /// The same 3 joints side by side under the root, each placed in the
    /// space of the whole model, the way the `WebXR` hand model has them.
    fn siblings() -> Rig {
        rig(
            vec![
                node(None, Vec3::ZERO),
                node(Some(0), Vec3::Y),
                node(Some(0), Vec3::Y * 2.0),
                node(Some(0), Vec3::Y * 3.0),
            ],
            &["root", "a", "b", "c"],
            vec![],
        )
    }

    fn turned(rig: &Rig, name: &str) -> JointPose {
        let (translation, rotation) = rig.joint_rest(rig.joint(name).unwrap()).unwrap();
        JointPose {
            translation,
            rotation: rotation * Quat::from_rotation_z(FRAC_PI_2),
        }
    }

    fn place(globals: &[Mat4], rig: &Rig, name: &str) -> Vec3 {
        globals[rig.joint(name).unwrap().0].transform_point3(Vec3::ZERO)
    }

    fn close(got: Vec3, want: Vec3) {
        assert!(got.distance(want) < CLOSE, "got {got}, want {want}");
    }

    #[test]
    fn a_name_is_looked_up_once_and_an_unknown_one_gives_none() {
        let rig = chain();
        assert_eq!(rig.joint("b"), Some(Joint(2)));
        assert_eq!(rig.joint("nope"), None);
        assert_eq!(rig.joint(""), None);
        assert_eq!(rig.joint_names().collect::<Vec<_>>(), ["root", "a", "b", "c"]);
        assert_eq!(rig.joint_rest(Joint(2)), Some((Vec3::Y, Quat::IDENTITY)));
        assert_eq!(rig.joint_rest(Joint(9)), None);
    }

    #[test]
    fn a_set_joint_in_a_chain_carries_the_joints_below_it() {
        let rig = chain();
        let mut set = vec![];
        assert!(rig.set_joint(&mut set, rig.joint("a").unwrap(), turned(&rig, "a")));

        // A quarter turn about z at `a` lays the rest of the chain along -x.
        let globals = rig.posed(None, &set).expect("a set joint is posed");
        close(place(&globals, &rig, "a"), Vec3::Y);
        close(place(&globals, &rig, "b"), Vec3::new(-1.0, 1.0, 0.0));
        close(place(&globals, &rig, "c"), Vec3::new(-2.0, 1.0, 0.0));
        // The skin follows the same transforms.
        let joints = rig.skins[0].joint_matrices(&globals);
        close(joints[2].transform_point3(Vec3::ZERO), Vec3::new(-2.0, 1.0, 0.0));
    }

    #[test]
    fn a_set_translation_moves_the_joint_and_the_joints_below_it() {
        let rig = chain();
        let mut set = vec![];
        let pose = JointPose {
            translation: Vec3::new(0.5, 1.0, 0.0),
            rotation:    Quat::IDENTITY,
        };
        assert!(rig.set_joint(&mut set, rig.joint("b").unwrap(), pose));

        let globals = rig.posed(None, &set).unwrap();
        close(place(&globals, &rig, "a"), Vec3::Y);
        close(place(&globals, &rig, "b"), Vec3::new(0.5, 2.0, 0.0));
        close(place(&globals, &rig, "c"), Vec3::new(0.5, 3.0, 0.0));
    }

    #[test]
    fn a_cleared_joint_is_back_at_rest() {
        let rig = chain();
        let (a, b) = (rig.joint("a").unwrap(), rig.joint("b").unwrap());
        let mut set = vec![];
        rig.set_joint(&mut set, a, turned(&rig, "a"));
        rig.set_joint(&mut set, b, turned(&rig, "b"));

        clear_joint(&mut set, a);
        let globals = rig.posed(None, &set).expect("b is still set");
        close(place(&globals, &rig, "b"), Vec3::Y * 2.0);
        close(place(&globals, &rig, "c"), Vec3::new(-1.0, 2.0, 0.0));

        clear_joint(&mut set, b);
        assert_eq!(set, vec![]);
        assert!(
            rig.posed(None, &set).is_none(),
            "nothing set and no clip is the rest pose"
        );
        assert_eq!(rig.pose_with(None, &set), rig.pose(None));
    }

    #[test]
    fn a_set_joint_among_siblings_moves_only_itself() {
        let rig = siblings();
        let mut set = vec![];
        let pose = JointPose {
            translation: Vec3::new(1.0, 1.0, 0.0),
            rotation:    Quat::from_rotation_z(FRAC_PI_2),
        };
        assert!(rig.set_joint(&mut set, rig.joint("a").unwrap(), pose));

        let globals = rig.posed(None, &set).unwrap();
        close(place(&globals, &rig, "a"), Vec3::new(1.0, 1.0, 0.0));
        close(globals[1].transform_vector3(Vec3::Y), -Vec3::X);
        close(place(&globals, &rig, "b"), Vec3::Y * 2.0);
        close(place(&globals, &rig, "c"), Vec3::Y * 3.0);
        assert_eq!(globals[2], rig.pose(None)[2]);
    }

    #[test]
    fn a_handle_of_another_model_is_refused() {
        let rig = chain();
        let mut set = vec![];
        assert!(!rig.set_joint(&mut set, Joint(4), turned(&rig, "a")));
        assert_eq!(set, vec![]);
        clear_joint(&mut set, Joint(4));
        assert_eq!(set, vec![]);
    }

    // The clip lifts `a` and `b` by 1 along x each. With `a` set, `a` stays
    // where the app put it and `b` still plays.
    #[test]
    fn a_set_joint_wins_over_the_clip_and_the_other_joints_still_play() {
        let lift = |node: usize| Channel {
            node,
            times: vec![0.0, 1.0],
            interpolation: Interpolation::Linear,
            track: Track::Translation(vec![Vec3::Y, Vec3::new(1.0, 1.0, 0.0)]),
        };
        let clip = Clip {
            name:     "lift".into(),
            duration: 1.0,
            channels: vec![lift(1), lift(2)],
        };
        let mut rig = chain();
        rig.clips.push(clip);
        let playback = Playback {
            clip:   0,
            time:   1.0,
            speed:  0.0,
            looped: false,
        };

        let played = rig.posed(Some(playback), &[]).expect("a clip is posed");
        close(place(&played, &rig, "b"), Vec3::new(2.0, 2.0, 0.0));

        let mut set = vec![];
        let rest = JointPose {
            translation: Vec3::Y,
            rotation:    Quat::IDENTITY,
        };
        rig.set_joint(&mut set, rig.joint("a").unwrap(), rest);
        let both = rig.posed(Some(playback), &set).unwrap();
        close(place(&both, &rig, "a"), Vec3::Y);
        close(place(&both, &rig, "b"), Vec3::new(1.0, 2.0, 0.0));
    }

    // The bar of `BoneTest.glb` is a real chain of 4 bones under an
    // armature. Its third bone turned carries the fourth and leaves the
    // 2 before it, and the mesh node is found by name too.
    #[test]
    fn the_bones_of_the_bar_are_found_by_name_and_a_set_one_carries_its_child() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/models/BoneTest.glb");
        let model = parse_glb(&read(path).unwrap(), "BoneTest.glb").unwrap();
        let rig = model.rig.as_ref().expect("a rig");

        let names: Vec<&str> = rig.joint_names().collect();
        for name in ["Armature", "Bone", "Bone.001", "Bone.002", "Bone.003", "Cube"] {
            assert!(names.contains(&name), "{name} in {names:?}");
        }
        assert_eq!(rig.joint("Bone.004"), None);

        let third = rig.joint("Bone.002").unwrap();
        let (translation, rotation) = rig.joint_rest(third).unwrap();
        assert!(
            translation.distance(Vec3::new(0.0, 3.825_286, 0.0)) < 1e-4,
            "{translation}"
        );
        let mut set = vec![];
        let pose = JointPose {
            translation,
            rotation: rotation * Quat::from_rotation_z(-0.8),
        };
        assert!(rig.set_joint(&mut set, third, pose));

        let rest = rig.pose(None);
        let posed = rig.posed(None, &set).unwrap();
        for name in ["Bone", "Bone.001", "Bone.002"] {
            close(place(&posed, rig, name), place(&rest, rig, name));
        }
        let lift = place(&posed, rig, "Bone.003").y - place(&rest, rig, "Bone.003").y;
        assert!(lift > 2.0, "the fourth bone rose by {lift}");

        // The skin draws through the same pose, its last joint moved and
        // its first did not.
        let skin = &rig.skins[0];
        let (rest_joints, posed_joints) = (skin.joint_matrices(&rest), skin.joint_matrices(&posed));
        let last = skin.joints.iter().position(|joint| *joint == rig.joint("Bone.003").unwrap().0);
        let first = skin.joints.iter().position(|joint| *joint == rig.joint("Bone").unwrap().0);
        let (last, first) = (last.unwrap(), first.unwrap());
        assert!(!posed_joints[last].abs_diff_eq(rest_joints[last], 1e-3));
        assert!(posed_joints[first].abs_diff_eq(rest_joints[first], 1e-6));
    }
}
