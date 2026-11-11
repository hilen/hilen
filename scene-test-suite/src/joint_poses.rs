use anyhow::{Context, Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Mat4, Quat, Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{Camera, Joint, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const BAR: &str = "BoneTest.glb";
const BEND: &str = "Bend";
/// The chain of the bar from its fixed end to its free end.
const BONES: [&str; 4] = ["Bone", "Bone.001", "Bone.002", "Bone.003"];
/// The turn of the third bone about its own z, the way that lifts the
/// free end of the bar.
const TURN: f32 = -0.8;
/// How far the fourth bone is pulled out along its parent, in the units
/// of the model.
const PULL: f32 = 2.0;
const TOLERANCE: f32 = 1e-3;

/// The skinned bar of `BoneTest.glb`, 4 bones in a chain, twice: a gray
/// one behind that stays at rest and an orange one in front whose joints
/// the test places from code with no clip playing. The sun casts, so the
/// shadow of the posed bar comes from the skinned shadow pass. The states:
/// at rest, the third bone turned so the free end rises with the fourth
/// bone on it, the fourth bone also pulled out along the third, the third
/// given back so the bar is straight with a longer end, the bend clip
/// frozen at its middle with the second bone held at its rest pose, and
/// at rest again after `clear_joints`. Each state is also read back with
/// `joint_transform`.
#[scene]
#[derive(Default)]
struct JointPoses {
    bar: Weak<Prop>,
}

impl SceneSetup for JointPoses {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(3.5, 15.0, 27.0),
            target: Vec3::new(3.5, 1.5, -1.0),
            ..Camera::default()
        };
        self.sun.direction = Vec3::new(-0.4, -1.0, -0.3);
        self.sun.shadows = true;
        // The default is 1024 on a phone and in the browser.
        self.sun.shadow_map_size = 2048;

        self.make_node::<Prop>(Shape3::Plane(70.0), Vec3::new(3.5, -1.2, 0.0))
            .set_color(Color::hex("#c8ccd0"))
            .set_roughness(0.9);

        self.make_node::<Prop>(Shape3::Model(Model::get(BAR)), Vec3::new(1.0, 0.0, -6.0))
            .set_color(Color::hex("#7f8fa6"))
            .set_roughness(0.6);

        self.bar = self.make_node::<Prop>(Shape3::Model(Model::get(BAR)), Vec3::new(1.0, 0.0, 1.0));
        self.bar.set_color(Color::hex("#e67e22")).set_roughness(0.6);
    }
}

/// Every answer for the 4 bones, or an error that names the bone with none.
fn whole<T>(answers: [Option<T>; 4], what: &str) -> Result<[T; 4]> {
    for (name, answer) in BONES.iter().zip(&answers) {
        ensure!(answer.is_some(), "{name} has no {what}");
    }
    Ok(answers.map(|answer| answer.expect("checked above")))
}

/// Where every bone of the chain is now, in the space of the model.
fn transforms(scene: Weak<JointPoses>, bones: [Joint; 4]) -> Result<[Mat4; 4]> {
    whole(
        from_main(move || bones.map(|bone| scene.bar.joint_transform(bone))),
        "transform",
    )
}

fn place(transform: Mat4) -> Vec3 {
    transform.transform_point3(Vec3::ZERO)
}

fn same(what: &str, got: Vec3, want: Vec3) -> Result<()> {
    ensure!(got.distance(want) < TOLERANCE, "{what}: got {got}, want {want}");
    Ok(())
}

/// The translation and the rotation of `joint` at rest in its parent.
fn rest_of(joint: Joint) -> Result<(Vec3, Quat)> {
    from_main(move || Model::get(BAR).joint_rest(joint)).context("a joint of the bar has a rest pose")
}

/// The third bone is turned, the fourth moved out along it, and the third
/// given back, with a check after each.
fn turned_and_pulled(scene: Weak<JointPoses>, bones: [Joint; 4], rest: &[Mat4; 4]) -> Result<()> {
    let [_, _, third, fourth] = bones;
    // The third bone turned: it stays in place with the 2 before it,
    // and the fourth rides up on it.
    let (third_place, third_turn) = rest_of(third)?;
    from_main(move || {
        let mut bar = scene.bar;
        bar.set_joint(third, third_place, third_turn * Quat::from_rotation_z(TURN));
    });
    wait_for_next_frame();
    wait_for_next_frame();
    let turned = transforms(scene, bones)?;
    for index in 0..3 {
        same(BONES[index], place(turned[index]), place(rest[index]))?;
    }
    let lift = place(turned[3]).y - place(rest[3]).y;
    ensure!(lift > 2.0, "the fourth bone rose by {lift}");
    check_colors(TURNED)?;

    // The fourth bone also moved, 2 units out along the turned third.
    let (fourth_place, fourth_turn) = rest_of(fourth)?;
    from_main(move || {
        let mut bar = scene.bar;
        bar.set_joint(fourth, fourth_place + Vec3::Y * PULL, fourth_turn);
    });
    wait_for_next_frame();
    wait_for_next_frame();
    let pulled = transforms(scene, bones)?;
    same("the third bone", place(pulled[2]), place(turned[2]))?;
    let moved = place(pulled[3]).distance(place(turned[3]));
    ensure!(
        (moved - PULL).abs() < TOLERANCE,
        "the fourth bone moved by {moved}"
    );
    ensure!(
        place(pulled[3]).y > place(turned[3]).y + 1.0,
        "the fourth bone moved along the raised third, from {} to {}",
        place(turned[3]),
        place(pulled[3])
    );
    check_colors(PULLED)?;

    // The third bone given back: the bar is straight, its end still
    // pulled out.
    from_main(move || {
        let mut bar = scene.bar;
        bar.clear_joint(third);
    });
    wait_for_next_frame();
    wait_for_next_frame();
    let straight = transforms(scene, bones)?;
    ensure!(
        straight[2].abs_diff_eq(rest[2], TOLERANCE),
        "the cleared third bone is back at rest"
    );
    let moved = place(straight[3]).distance(place(rest[3]));
    ensure!(
        (moved - PULL).abs() < TOLERANCE,
        "the fourth bone is {moved} from its rest"
    );
    check_colors(STRAIGHT)?;
    Ok(())
}

/// A set joint wins over a playing clip, and the other joints play on.
fn over_a_clip(scene: Weak<JointPoses>, bones: [Joint; 4]) -> Result<()> {
    let [_, second, ..] = bones;
    // Over a clip a set joint wins. The clip is frozen at its middle,
    // first alone, then with the second bone held at its rest pose.
    let (second_place, second_turn) = rest_of(second)?;
    from_main(move || {
        let mut bar = scene.bar;
        let model = Model::get(BAR);
        let middle = model.clips()[model.clip(BEND).expect("the bar has the clip")].duration / 2.0;
        bar.clear_joints()
            .play(BEND)
            .set_animation_speed(0.0)
            .set_animation_time(middle);
    });
    let bent = transforms(scene, bones)?;
    let rest_local = Mat4::from_rotation_translation(second_turn, second_place);
    ensure!(
        !(bent[0].inverse() * bent[1]).abs_diff_eq(rest_local, TOLERANCE),
        "the clip turns the second bone"
    );
    from_main(move || {
        let mut bar = scene.bar;
        bar.set_joint(second, second_place, second_turn);
    });
    wait_for_next_frame();
    wait_for_next_frame();
    let held = transforms(scene, bones)?;
    ensure!(
        (held[0].inverse() * held[1]).abs_diff_eq(rest_local, TOLERANCE),
        "the set second bone keeps its rest pose in its parent over the clip"
    );
    ensure!(
        held[0].abs_diff_eq(bent[0], TOLERANCE),
        "the first bone still plays the clip"
    );
    ensure!(
        (held[2].inverse() * held[3]).abs_diff_eq(bent[2].inverse() * bent[3], TOLERANCE),
        "the fourth bone still plays the clip"
    );
    check_colors(OVER_CLIP)?;
    Ok(())
}

impl SceneTest for JointPoses {
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        let bones = whole(
            from_main(move || BONES.map(|name| scene.bar.joint(name))),
            "joint",
        )?;

        let (unknown, names) = from_main(move || {
            let model = Model::get(BAR);
            let names: Vec<String> = model.joint_names().map(ToString::to_string).collect();
            (scene.bar.joint("Bone.004"), names)
        });
        ensure!(unknown.is_none(), "an unknown name gave {unknown:?}");
        for name in BONES {
            ensure!(
                names.iter().any(|known| known == name),
                "{name} is not in {names:?}"
            );
        }

        wait_for_next_frame();
        wait_for_next_frame();
        let rest = transforms(scene, bones)?;
        check_colors(REST)?;

        turned_and_pulled(scene, bones, &rest)?;
        over_a_clip(scene, bones)?;

        from_main(move || {
            let mut bar = scene.bar;
            bar.stop_animation().clear_joints();
        });
        wait_for_next_frame();
        wait_for_next_frame();
        let cleared = transforms(scene, bones)?;
        for index in 0..4 {
            ensure!(
                cleared[index].abs_diff_eq(rest[index], TOLERANCE),
                "{} is back at rest",
                BONES[index]
            );
        }
        check_colors(REST)
    }
}

const REST: &str = r"
       4    4 - #597c95
     188    4 - #597c95
     292    4 - #597c95
     484    4 - #597c95
     592    4 - #597c95
     388   16 - #597c95
      96   20 - #597c95
      44  100 - #597c95
     148  100 - #597c95
     440  100 - #597c95
     548  108 - #597c95
     312  140 - #597c95
     216  176 - #597c95
     104  188 - #597c95
       4  196 - #597c95
     484  196 - #597c95
     592  208 - #c4c8cc
     176  272 - #7f8ea3
     196  272 - #7f8ea3
     244  272 - #7f8ea3
     292  272 - #7f8ea3
     332  272 - #7f8ea3
     380  272 - #7f8ea3
     400  272 - #7f8ea3
     152  276 - #7f8ea3
     212  276 - #7f8ea3
     228  276 - #7f8ea3
     268  276 - #7f8ea3
     312  276 - #7f8ea3
     348  276 - #7f8ea3
     364  276 - #7f8ea3
     420  276 - #7f8ea3
     148  300 - #6e7072
     168  308 - #6e7072
     184  308 - #6e7072
     200  308 - #6e7072
     216  308 - #6e7072
     232  308 - #6e7072
     248  308 - #6e7072
     288  308 - #707274
     304  308 - #6e7072
     324  308 - #6e7072
     336  308 - #6e7072
     348  308 - #6e7072
     368  308 - #6e7072
     388  308 - #6e7072
     416  308 - #6e7072
     556  308 - #c4c8cc
       4  316 - #c4c8cc
     128  316 - #e07e31
     268  316 - #e07e31
     432  316 - #e07e32
     308  320 - #e07e31
     160  324 - #e07e31
     180  324 - #e07e31
     352  324 - #e07e32
     384  324 - #e07e32
     200  328 - #e07e31
     220  328 - #e07e31
     240  328 - #e07e31
     288  328 - #e07e31
     332  328 - #e07e32
     368  328 - #e07e32
     400  328 - #e07e32
     448  328 - #e07e32
     424  332 - #a45c26
     260  336 - #a45c25
     140  340 - #a45b25
     180  344 - #a45b25
     384  344 - #a45c25
     224  348 - #a45b25
     280  348 - #a45b25
     116  352 - #6e7072
     116  356 - #6e7072
     340  356 - #a45c25
     424  356 - #a45c25
     156  360 - #a35b25
     200  360 - #a45b25
     248  360 - #a45b25
     312  360 - #a45b25
     368  360 - #a45c25
     404  360 - #a45c25
     444  360 - #a45c25
     592  408 - #c4c8cc
       4  448 - #c4c8cc
     100  452 - #c4c8cc
     372  456 - #c4c8cc
     280  476 - #c4c8cc
     552  500 - #c4c8cc
     452  504 - #c4c8cc
     160  524 - #c4c8cc
       4  592 - #c4c8cb
      96  592 - #c4c8cb
     312  592 - #c4c8cc
     412  592 - #c4c8cc
     592  592 - #c4c8cc
";

const TURNED: &str = r"
       4    4 - #597c95
     316    4 - #597c95
     476    4 - #597c95
     592    4 - #597c95
     396   40 - #597c95
     160   44 - #597c95
      76   48 - #597c95
     548  100 - #597c95
     444  112 - #597c95
     220  120 - #597c95
     100  128 - #597c95
     312  140 - #597c95
       4  168 - #597c95
     392  184 - #597c95
     492  196 - #597c95
     588  196 - #597c95
     176  204 - #de7d31
     192  212 - #de7d31
     204  224 - #de7d31
     152  232 - #a85e27
     224  240 - #de7d30
     180  244 - #a85e27
     236  252 - #de7d30
     204  260 - #a85e27
     248  264 - #de7d30
     232  268 - #a85e27
     156  272 - #7f8ea3
     280  272 - #7f8ea3
     336  272 - #7f8ea3
     384  272 - #7f8ea3
     168  276 - #7f8ea3
     180  276 - #7f8ea3
     192  276 - #7f8ea3
     268  276 - #7f8ea3
     292  276 - #7f8ea3
     308  276 - #7f8ea3
     324  276 - #7f8ea3
     348  276 - #7f8ea3
     360  276 - #7f8ea3
     372  276 - #7f8ea3
     400  276 - #7f8ea3
     420  276 - #7f8ea3
     256  280 - #de7d30
     208  284 - #a85e26
     272  288 - #de7d30
     236  296 - #a85e26
       4  300 - #c4c8cc
     148  300 - #6e7072
     284  300 - #df7d31
     184  308 - #6e7072
     212  308 - #6e7072
     356  308 - #6e7072
     380  308 - #6e7072
     564  308 - #c4c8cc
     260  312 - #a85e26
     308  312 - #c4c8cc
     404  316 - #e07e32
     436  316 - #e07e32
     336  320 - #e07e32
     280  324 - #a85e26
     164  328 - #6e7072
     364  328 - #e07e32
     384  328 - #e07e32
     424  328 - #e07e32
     448  328 - #e07e32
     208  332 - #6e7072
     128  336 - #6e7072
     316  336 - #a45c26
     228  340 - #6e7072
     252  340 - #6e7072
     148  344 - #6e7072
     292  344 - #a65d26
     340  344 - #a45c25
     168  348 - #6e7072
     192  348 - #6e7072
     380  352 - #a45c25
     240  356 - #6e7072
     260  360 - #6e7072
     280  360 - #6e7072
     324  360 - #a45c25
     360  360 - #a45c25
     404  360 - #a45c25
     440  360 - #a45c25
     592  420 - #c4c8cc
       4  432 - #c4c8cc
     108  440 - #c4c8cc
     380  456 - #c4c8cc
     284  480 - #c4c8cc
     552  504 - #c4c8cc
     460  512 - #c4c8cc
     168  524 - #c4c8cc
       4  592 - #c4c8cb
     100  592 - #c4c8cb
     236  592 - #c4c8cc
     332  592 - #c4c8cc
     592  592 - #c4c8cc
";

const PULLED: &str = r"
       4    4 - #597c95
     340    4 - #597c95
     472    4 - #597c95
     592    4 - #597c95
     172   32 - #597c95
     260   56 - #597c95
      80   72 - #597c95
     372   84 - #597c95
     552  100 - #597c95
     448  120 - #597c95
     224  128 - #597c95
     304  136 - #597c95
       4  140 - #597c95
     152  176 - #de7d31
     380  184 - #597c95
     168  188 - #de7d31
     496  196 - #597c95
     588  196 - #597c95
     120  204 - #a85e27
     156  204 - #a85e27
     184  204 - #de7d31
     208  224 - #de7d31
     152  232 - #a85e27
     224  240 - #de7d30
     184  244 - #a85e27
     216  256 - #a85e27
       4  260 - #c4c8cc
     244  260 - #de7d30
     172  272 - #7f8ea3
     280  272 - #7f8ea3
     308  272 - #7f8ea3
     336  272 - #7f8ea3
     384  272 - #7f8ea3
     152  276 - #7f8ea3
     188  276 - #7f8ea3
     272  276 - #7f8ea3
     288  276 - #7f8ea3
     300  276 - #7f8ea3
     320  276 - #7f8ea3
     348  276 - #7f8ea3
     360  276 - #7f8ea3
     372  276 - #7f8ea3
     400  276 - #7f8ea3
     420  276 - #7f8ea3
     264  280 - #de7d30
     228  288 - #a85e26
     280  296 - #df7d31
     160  308 - #6e7072
     200  308 - #6e7072
     224  308 - #6e7072
     292  308 - #e07e31
     328  308 - #6f7173
     360  308 - #6f7174
     564  308 - #c4c8cc
     308  312 - #c4c8cc
     244  316 - #a85e26
     424  316 - #e07e32
     276  320 - #a85e26
     400  320 - #e07e32
     128  324 - #6e7072
     152  328 - #6e7072
     316  328 - #e27f32
     336  328 - #e07e32
     356  328 - #e07e32
     376  328 - #e07e32
     448  328 - #e07e32
     184  332 - #6e7072
     224  336 - #6e7072
      96  340 - #6e7072
     260  340 - #6e7072
     304  340 - #a55c26
     428  340 - #a45c26
     136  344 - #6e7072
     396  344 - #a45c25
     160  348 - #6e7072
     200  352 - #6e7072
     280  352 - #6e7072
     344  352 - #a45c25
     240  356 - #6e7072
     444  356 - #a45c25
     320  360 - #a45c25
     372  360 - #a45c25
     416  360 - #a45c25
     592  420 - #c4c8cc
     100  436 - #c4c8cc
       4  448 - #c4c8cc
     364  460 - #c4c8cc
     268  476 - #c4c8cc
     540  504 - #c4c8cc
     152  516 - #c4c8cc
     444  520 - #c4c8cc
       4  592 - #c4c8cb
     100  592 - #c4c8cb
     204  592 - #c4c8cc
     300  592 - #c4c8cc
     592  592 - #c4c8cc
";

const STRAIGHT: &str = r"
       4    4 - #597c95
     188    4 - #597c95
     292    4 - #597c95
     484    4 - #597c95
     592    4 - #597c95
     388   16 - #597c95
      48   92 - #597c95
     148  100 - #597c95
     440  100 - #597c95
     548  104 - #597c95
     276  140 - #597c95
     372  172 - #597c95
       4  180 - #597c95
     104  188 - #597c95
     484  196 - #597c95
     592  204 - #c4c8cc
     156  272 - #7f8ea3
     192  272 - #7f8ea3
     228  272 - #7f8ea3
     248  272 - #7f8ea3
     296  272 - #7f8ea3
     332  272 - #7f8ea3
     348  272 - #7f8ea3
     396  272 - #7f8ea3
     176  276 - #7f8ea3
     212  276 - #7f8ea3
     264  276 - #7f8ea3
     280  276 - #7f8ea3
     320  276 - #7f8ea3
     372  276 - #7f8ea3
     420  276 - #7f8ea3
       4  280 - #c4c8cc
     148  300 - #6e7072
     176  308 - #6e7072
     200  308 - #6e7072
     228  308 - #6e7072
     256  308 - #6e7072
     276  308 - #6e7173
     296  308 - #6e7173
     308  308 - #707375
     348  308 - #6e7072
     364  308 - #6e7072
     416  308 - #6e7072
     560  312 - #c4c8cc
      96  316 - #e07e31
     156  316 - #e07e31
     328  316 - #e07e32
     376  316 - #e07e32
     396  316 - #e07e32
     440  316 - #e07e32
     116  320 - #e07e31
     292  320 - #e07e31
     208  324 - #e07e31
     136  328 - #e07e31
     172  328 - #e07e31
     244  328 - #e07e31
     268  328 - #e07e31
     352  328 - #e07e32
     424  328 - #e07e32
     224  332 - #a45b25
     448  332 - #a45c26
     332  336 - #a45c25
     380  336 - #a45c26
     404  336 - #a45c26
     108  340 - #a45b25
     156  344 - #a45b25
     196  344 - #a45b25
     312  344 - #a45c25
     252  348 - #a45b25
      80  352 - #6e7072
     360  352 - #a45c25
      80  356 - #6e7072
      76  360 - #6e7072
      80  360 - #6e7072
     288  360 - #a45b25
     336  360 - #a45c25
     384  360 - #a45c25
     424  360 - #a45c25
     100  364 - #6e7072
     120  364 - #6e7072
     148  364 - #6e7072
     172  364 - #6e7072
     216  364 - #a5a8ab
     496  420 - #c4c8cc
     592  420 - #c4c8cc
     104  460 - #c4c8cc
     376  460 - #c4c8cc
       4  464 - #c4c8cc
     280  484 - #c4c8cc
     464  508 - #c4c8cc
     168  536 - #c4c8cc
     244  588 - #c4c8cc
       4  592 - #c4c8cb
     336  592 - #c4c8cc
     428  592 - #c4c8cc
     592  592 - #c4c8cc
";

const OVER_CLIP: &str = r"
       4    4 - #597c95
      96    4 - #597c95
     344    4 - #597c95
     448    4 - #597c95
     592    4 - #597c95
     176   48 - #597c95
     268   68 - #597c95
      72   96 - #597c95
     468  104 - #597c95
     572  104 - #597c95
     332  140 - #597c95
       4  184 - #597c95
      96  184 - #597c95
     212  192 - #cb722d
     496  196 - #597c95
     196  200 - #d2762e
     592  204 - #c4c8cc
     180  208 - #d2762e
     220  216 - #cb722d
     176  224 - #9c5724
     224  224 - #cb722d
     228  232 - #cb722d
     232  236 - #cb722d
     232  240 - #cb722d
     200  244 - #9c5724
     236  244 - #cc722d
     236  248 - #cc722d
     240  252 - #cc732d
     240  256 - #cc722d
     244  260 - #cd732d
     244  264 - #cd732d
     248  268 - #ce742d
     164  272 - #7f8ea3
     184  272 - #7f8ea3
     248  272 - #ce732d
     268  272 - #7f8ea3
     312  272 - #7f8ea3
     356  272 - #7f8ea3
     416  272 - #7f8ea3
     200  276 - #7f8ea3
     252  276 - #d3762e
     284  276 - #7f8ea3
     300  276 - #7f8ea3
     332  276 - #7f8ea3
     368  276 - #7f8ea3
     384  276 - #7f8ea3
     400  276 - #7f8ea3
     224  280 - #9c5824
      20  304 - #c4c8cc
     144  304 - #6e7072
     228  304 - #a05a25
     264  304 - #dd7c30
     180  308 - #6e7072
     204  308 - #6e7072
     280  308 - #e27f31
     324  308 - #6e7072
     352  308 - #6e7072
     296  312 - #e58133
     372  316 - #e07e32
     440  316 - #e07e32
     560  316 - #c4c8cc
     420  320 - #e07e32
     328  324 - #e17e32
     400  324 - #e07e32
     248  328 - #a65d26
     312  328 - #e17f32
     344  328 - #e07e32
     384  328 - #e07e32
     288  332 - #a65d26
     208  336 - #6e7072
     176  340 - #6e7072
     364  340 - #a45c25
     432  340 - #a45c26
     144  348 - #6e7072
     272  348 - #a75d26
     192  352 - #6e7072
     212  356 - #6e7072
     236  356 - #6e7072
     348  356 - #a45c25
     408  356 - #a45c25
     256  360 - #6e7072
     292  360 - #6e7072
     316  360 - #a45c25
     376  360 - #a45c25
     444  360 - #a45c25
       4  428 - #c4c8cc
     592  428 - #c4c8cc
     108  444 - #c4c8cc
     384  456 - #c4c8cc
     288  480 - #c4c8cc
     464  512 - #c4c8cc
     172  524 - #c4c8cc
       4  592 - #c4c8cb
     104  592 - #c4c8cb
     336  592 - #c4c8cc
     592  592 - #c4c8cc
";
