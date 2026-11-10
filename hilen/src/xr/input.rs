use anyhow::Result;
use openxr as xr;

use super::pose::{Hand, HandState, Pose, Rig};
use crate::gm::{
    flat::Point,
    volume::{Quat, Vec3},
};

/// The actions of both controllers, bound to the Valve Index controller
/// and to the simple controller every runtime has.
pub(super) struct Input {
    set:     xr::ActionSet,
    grip:    xr::Action<xr::Posef>,
    aim:     xr::Action<xr::Posef>,
    trigger: xr::Action<f32>,
    squeeze: xr::Action<f32>,
    stick:   xr::Action<xr::Vector2f>,
    a:       xr::Action<bool>,
    b:       xr::Action<bool>,
    hands:   [xr::Path; 2],
    grips:   [xr::Space; 2],
    aims:    [xr::Space; 2],
}

impl Input {
    /// The bindings are suggestions the runtime may change, `SteamVR` lets
    /// the player bind them again in its own settings.
    pub(super) fn new(instance: &xr::Instance, session: &xr::Session<xr::D3D12>) -> Result<Self> {
        let hands = [
            instance.string_to_path("/user/hand/left")?,
            instance.string_to_path("/user/hand/right")?,
        ];
        let set = instance.create_action_set("hilen", "Hilen", 0)?;
        let grip = set.create_action::<xr::Posef>("grip", "Grip", &hands)?;
        let aim = set.create_action::<xr::Posef>("aim", "Aim", &hands)?;
        let trigger = set.create_action::<f32>("trigger", "Trigger", &hands)?;
        let squeeze = set.create_action::<f32>("squeeze", "Squeeze", &hands)?;
        let stick = set.create_action::<xr::Vector2f>("stick", "Thumbstick", &hands)?;
        let a = set.create_action::<bool>("a", "A", &hands)?;
        let b = set.create_action::<bool>("b", "B", &hands)?;

        let path = |path: &str| instance.string_to_path(path);
        let mut index = vec![];
        let mut simple = vec![];
        for side in ["left", "right"] {
            let input = |name: &str| path(&format!("/user/hand/{side}/input/{name}"));
            index.extend([
                xr::Binding::new(&grip, input("grip/pose")?),
                xr::Binding::new(&aim, input("aim/pose")?),
                xr::Binding::new(&trigger, input("trigger/value")?),
                xr::Binding::new(&squeeze, input("squeeze/value")?),
                xr::Binding::new(&stick, input("thumbstick")?),
                xr::Binding::new(&a, input("a/click")?),
                xr::Binding::new(&b, input("b/click")?),
            ]);
            simple.extend([
                xr::Binding::new(&grip, input("grip/pose")?),
                xr::Binding::new(&aim, input("aim/pose")?),
                xr::Binding::new(&trigger, input("select/click")?),
            ]);
        }
        instance.suggest_interaction_profile_bindings(
            path("/interaction_profiles/valve/index_controller")?,
            &index,
        )?;
        instance.suggest_interaction_profile_bindings(
            path("/interaction_profiles/khr/simple_controller")?,
            &simple,
        )?;
        session.attach_action_sets(&[&set])?;

        let space =
            |action: &xr::Action<xr::Posef>, hand| action.create_space(session, hand, xr::Posef::IDENTITY);
        let grips = [space(&grip, hands[0])?, space(&grip, hands[1])?];
        let aims = [space(&aim, hands[0])?, space(&aim, hands[1])?];

        Ok(Self {
            set,
            grip,
            aim,
            trigger,
            squeeze,
            stick,
            a,
            b,
            hands,
            grips,
            aims,
        })
    }

    /// Both controllers at `time`, in the world of `rig`.
    pub(super) fn read(
        &self,
        session: &xr::Session<xr::D3D12>,
        base: &xr::Space,
        time: xr::Time,
        rig: Rig,
    ) -> Result<[Option<HandState>; 2]> {
        session.sync_actions(&[(&self.set).into()])?;
        let mut hands = [None, None];
        for hand in Hand::BOTH {
            let i = hand.index();
            let path = self.hands[i];
            if !self.grip.is_active(session, path)? || !self.aim.is_active(session, path)? {
                continue;
            }
            let (Some(grip), Some(aim)) = (
                located(&self.grips[i], base, time)?,
                located(&self.aims[i], base, time)?,
            ) else {
                continue;
            };
            let stick = self.stick.state(session, path)?.current_state;
            hands[i] = Some(HandState {
                grip:    rig.world(grip),
                aim:     rig.world(aim),
                trigger: self.trigger.state(session, path)?.current_state,
                squeeze: self.squeeze.state(session, path)?.current_state,
                stick:   Point::new(stick.x, stick.y),
                a:       self.a.state(session, path)?.current_state,
                b:       self.b.state(session, path)?.current_state,
            });
        }
        Ok(hands)
    }
}

/// Where `space` is in `base` at `time`, None while the tracking lost it.
pub(super) fn located(space: &xr::Space, base: &xr::Space, time: xr::Time) -> Result<Option<Pose>> {
    let location = space.locate(base, time)?;
    let valid = xr::SpaceLocationFlags::POSITION_VALID | xr::SpaceLocationFlags::ORIENTATION_VALID;
    Ok(location.location_flags.contains(valid).then(|| pose(location.pose)))
}

pub(super) fn pose(pose: xr::Posef) -> Pose {
    let (p, o) = (pose.position, pose.orientation);
    Pose {
        position: Vec3::new(p.x, p.y, p.z),
        rotation: Quat::from_xyzw(o.x, o.y, o.z, o.w),
    }
}
