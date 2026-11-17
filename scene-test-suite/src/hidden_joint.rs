use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Mat4, Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{Camera, Joint, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const BAR: &str = "BoneTest.glb";
/// The chain of the bar from its fixed end to its free end. The third
/// bone carries the fourth.
const BONES: [&str; 4] = ["Bone", "Bone.001", "Bone.002", "Bone.003"];
const TOLERANCE: f32 = 1e-4;

/// The skinned bar of `BoneTest.glb` over a floor, under a sun that
/// casts. Its third bone is hidden from the camera: the camera sees the
/// bar end after the second bone, while the shadow on the floor keeps the
/// whole bar, the third and fourth bones too. The pose itself does not
/// change, `joint_transform` reads the same places. Shown to the camera
/// again the bar is whole once more.
#[scene]
#[derive(Default)]
struct HiddenJoint {
    bar: Weak<Prop>,
}

impl SceneSetup for HiddenJoint {
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

        self.bar = self.make_node::<Prop>(Shape3::Model(Model::get(BAR)), Vec3::new(1.0, 0.0, 1.0));
        self.bar.set_color(Color::hex("#e67e22")).set_roughness(0.6);
    }
}

/// Where every bone of the bar is now, in the space of the model.
fn transforms(scene: Weak<HiddenJoint>, bones: [Joint; 4]) -> Result<[Mat4; 4]> {
    let answers = from_main(move || bones.map(|bone| scene.bar.joint_transform(bone)));
    for (name, answer) in BONES.iter().zip(&answers) {
        ensure!(answer.is_some(), "{name} has no transform");
    }
    Ok(answers.map(|answer| answer.expect("checked above")))
}

impl SceneTest for HiddenJoint {
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        let bones = from_main(move || BONES.map(|name| scene.bar.joint(name)));
        for (name, bone) in BONES.iter().zip(&bones) {
            ensure!(bone.is_some(), "the bar has no {name}");
        }
        let bones = bones.map(|bone| bone.expect("checked above"));
        let third = bones[2];

        wait_for_next_frame();
        wait_for_next_frame();
        let rest = transforms(scene, bones)?;
        check_colors(REST)?;

        from_main(move || {
            let mut bar = scene.bar;
            bar.hide_joint_from_camera(third);
        });
        wait_for_next_frame();
        wait_for_next_frame();
        let hidden = transforms(scene, bones)?;
        for index in 0..4 {
            ensure!(
                hidden[index].abs_diff_eq(rest[index], TOLERANCE),
                "{} keeps its place while hidden",
                BONES[index]
            );
        }
        check_colors(HIDDEN)?;

        from_main(move || {
            let mut bar = scene.bar;
            bar.show_joint_to_camera(third);
        });
        wait_for_next_frame();
        wait_for_next_frame();
        check_colors(REST)
    }
}

const REST: &str = r"
       4    4 - #597c95
     144    4 - #597c95
     444    4 - #597c95
     592    4 - #597c95
     300   24 - #597c95
     520   24 - #597c95
      76   28 - #597c95
     220   28 - #597c95
     376   36 - #597c95
     132   76 - #597c95
     468   80 - #597c95
      64  100 - #597c95
     252  104 - #597c95
     344  104 - #597c95
     540  108 - #597c95
       4  128 - #597c95
     168  140 - #597c95
     428  148 - #597c95
     592  148 - #597c95
      64  168 - #597c95
     296  184 - #597c95
       4  196 - #597c95
     504  196 - #597c95
     592  212 - #c4c8cc
     104  220 - #c4c8cc
     212  224 - #c4c8cc
     380  224 - #c4c8cc
     540  308 - #c4c8cc
       4  316 - #c4c8cc
     144  316 - #e07e31
     188  316 - #e07e31
     228  316 - #e07e31
     268  316 - #e07e31
     284  316 - #e07e31
     304  316 - #e07e31
     320  316 - #e07e31
     336  316 - #e07e32
     356  316 - #e07e32
     372  316 - #e07e32
     400  316 - #e07e32
     416  316 - #e07e32
     432  316 - #e07e32
     120  324 - #e07e31
     208  324 - #e07e31
     168  328 - #e07e31
     248  328 - #e07e31
     296  328 - #e07e31
     312  328 - #e07e32
     328  328 - #e07e32
     344  328 - #e07e32
     364  328 - #e07e32
     448  328 - #e07e32
     424  332 - #a45c26
     184  336 - #a45b25
     276  336 - #a45c25
     388  336 - #a45c26
     224  340 - #a45b25
     144  344 - #a45b25
     200  344 - #a45b25
     320  344 - #a45c25
     364  344 - #a45c25
     404  344 - #a45c26
     260  348 - #a45b25
     116  352 - #6e7072
     116  356 - #6e7072
     116  360 - #6e7072
     136  360 - #a35b25
     168  360 - #a45b25
     188  360 - #a45b25
     208  360 - #a45b25
     228  360 - #a45b25
     272  360 - #a45b25
     292  360 - #a45b25
     312  360 - #a45b25
     344  360 - #a45c25
     388  360 - #a45c25
     416  360 - #a45c25
     444  360 - #a45c25
     592  408 - #c4c8cc
     504  424 - #c4c8cc
       4  448 - #c4c8cc
     192  448 - #c4c8cc
     100  456 - #c4c8cc
     364  456 - #c4c8cc
     272  480 - #c4c8cc
     452  500 - #c4c8cc
     552  500 - #c4c8cc
      48  520 - #c4c8cc
     160  528 - #c4c8cc
     500  576 - #c4c8cc
       4  592 - #c4c8cb
      96  592 - #c4c8cb
     224  592 - #c4c8cc
     312  592 - #c4c8cc
     412  592 - #c4c8cc
     592  592 - #c4c8cc
";

const HIDDEN: &str = r"
       4    4 - #597c95
     228    4 - #597c95
     452    4 - #597c95
     592    4 - #597c95
     152    8 - #597c95
     300   28 - #597c95
     380   36 - #597c95
      96   76 - #597c95
     220   92 - #597c95
     540  104 - #597c95
     348  108 - #597c95
      20  124 - #597c95
     432  140 - #597c95
     140  156 - #597c95
     288  176 - #597c95
     368  196 - #597c95
     504  196 - #597c95
     592  208 - #c4c8cc
     204  240 - #c4c8cc
       4  244 - #c4c8cc
     104  248 - #c4c8cc
     328  316 - #e07e32
     352  316 - #e07e32
     364  316 - #e07e32
     376  316 - #e07e32
     396  316 - #e07e32
     420  316 - #e07e32
     432  316 - #e07e32
     560  316 - #c4c8cc
     340  320 - #e07e32
     408  320 - #e07e32
     444  320 - #e07e32
     304  324 - #7e471f
     304  328 - #7e471f
     324  328 - #e07e32
     360  328 - #e07e32
     372  328 - #e07e32
     384  328 - #e07e32
     420  328 - #e07e32
     432  328 - #e07e32
     300  332 - #7e471f
     304  332 - #7e471f
     336  332 - #a45c25
     400  332 - #a45c26
     300  336 - #7e471f
     304  336 - #7e471f
     308  336 - #9e5824
     300  340 - #7e471f
     304  340 - #7e471f
     308  340 - #9a5624
     348  340 - #a45c25
     424  340 - #a45c26
      16  344 - #c4c8cc
     136  344 - #6e7072
     160  344 - #6e7072
     176  344 - #6e7072
     192  344 - #6e7072
     236  344 - #6e7072
     260  344 - #6e7072
     276  344 - #6e7072
     300  344 - #7e471f
     304  344 - #7e471f
     332  344 - #a45c25
     376  344 - #a45c25
     392  344 - #a45c25
     408  344 - #a45c26
     208  348 - #6e7072
     304  348 - #7e471f
     360  348 - #a45c25
     448  348 - #a45c26
     304  352 - #7e471f
     116  360 - #6e7072
     148  360 - #6e7072
     184  360 - #6e7072
     224  360 - #6e7072
     240  360 - #6e7072
     256  360 - #6e7072
     272  360 - #6e7072
     288  360 - #6e7072
     320  360 - #a45b25
     336  360 - #a45c25
     352  360 - #a45c25
     372  360 - #a45c25
     400  360 - #a45c25
     428  360 - #a45c25
     592  428 - #c4c8cc
       4  444 - #c4c8cc
     488  452 - #c4c8cc
     208  456 - #c4c8cc
     300  480 - #c4c8cc
     128  508 - #c4c8cc
     420  536 - #c4c8cc
       4  592 - #c4c8cb
     252  592 - #c4c8cc
     496  592 - #c4c8cc
     592  592 - #c4c8cc
";
