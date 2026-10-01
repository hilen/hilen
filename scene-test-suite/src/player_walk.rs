use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::volume::{Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{
        Body, Light, Material, Node, NodeTemplates, Player, SceneCreation, SceneSetup, SceneTest, Wall,
        scene, step_scene,
    },
    ui::{Color, Image},
    ui_test::{capture_screenshot, check_colors, hold_key, release_key, set_record_probe_count},
    window::KeyCode,
};

/// Frames of walking, two seconds, more than the wall is away.
const WALK_FRAMES: u32 = 120;
const SETTLE_FRAMES: u32 = 20;
const CRATE_START: Vec3 = Vec3::new(0.6, 0.5, 1.5);

/// A first person player walks forward into a crate and on into a brick
/// wall. The crate has to be shoved aside, the wall has to stop the
/// capsule at its surface, a jump has to lift the player and gravity
/// bring it back, and the camera has to look out of the player's eyes,
/// so the last frame is the wall up close with its bricks in relief
/// under the grazing lamp.
#[scene]
#[derive(Default)]
struct PlayerWalk {
    crate_box: Weak<Body>,
}

impl SceneSetup for PlayerWalk {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.lights
            .push(Light::point(Vec3::new(-2.5, 1.6, -2.3)).intensity(3.0).range(8.0));

        self.make_node::<Wall>(Shape3::Plane(20.0), Vec3::ZERO)
            .set_color(Color::hex("#8d9aa5"))
            .set_roughness(0.9);

        let bricks = Material {
            color:        Color::hex("#ffffff"),
            metallic:     0.0,
            roughness:    0.8,
            texture:      Some(Image::get("bricks.jpg")),
            normal_map:   Some(Image::get("bricks_normal.jpg")),
            normal_scale: 1.5,
            emissive:     0.0,
            cutout:       None,
        };
        for x in [-2.0, 0.0, 2.0] {
            self.make_node::<Wall>(Shape3::cuboid(2.0, 2.0, 0.5), Vec3::new(x, 1.0, -3.0))
                .set_material(bricks);
        }

        let mut crate_box = self.make_node::<Body>(Shape3::cube(1.0), CRATE_START);
        crate_box.set_color(Color::hex("#ffffff")).set_roughness(0.7);
        crate_box.material.texture = Some(Image::get("crate_box.png"));
        self.crate_box = crate_box;

        self.add_player(Vec3::new(0.0, 1.0, 4.0));
    }
}

impl SceneTest for PlayerWalk {
    fn stepped() -> bool {
        true
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        let player = |scene: Weak<Self>| from_main(move || scene.player.as_ref().map(Player::position));

        // Spawned a little above the floor, the player settles onto it
        // first.
        step_scene(SETTLE_FRAMES);
        let start = player(scene).expect("the scene has a player");

        hold_key(KeyCode::KeyW);
        step_scene(WALK_FRAMES);
        release_key(KeyCode::KeyW);

        let stopped = player(scene).expect("the scene has a player");
        let crate_box = from_main(move || scene.crate_box.position());
        // The shove deflects the capsule sideways a little, the wall is
        // what stops it.
        anyhow::ensure!(
            stopped.z > -2.5 && stopped.z < -2.2 && stopped.x.abs() < 0.8,
            "the wall did not stop the player, it is at {stopped:?}"
        );
        anyhow::ensure!(
            (stopped.y - start.y).abs() < 0.05,
            "the player left the floor while walking, it is at {stopped:?}"
        );
        anyhow::ensure!(
            crate_box.z < 1.0 && crate_box.x > CRATE_START.x,
            "the player did not shove the crate, it is at {crate_box:?}"
        );

        hold_key(KeyCode::Space);
        step_scene(1);
        release_key(KeyCode::Space);
        step_scene(6);
        let airborne = player(scene).expect("the scene has a player");
        anyhow::ensure!(
            airborne.y > stopped.y + 0.15,
            "the jump did not lift the player, it is at {airborne:?}"
        );
        step_scene(90);
        let landed = player(scene).expect("the scene has a player");
        anyhow::ensure!(
            (landed.y - stopped.y).abs() < 0.05,
            "the player did not land, it is at {landed:?}"
        );

        let (camera, eye) = from_main(move || {
            let player = scene.player.as_ref().expect("the scene has a player");
            (
                scene.camera.position,
                player.position() + Vec3::Y * player.eye_height,
            )
        });
        anyhow::ensure!(
            (camera - eye).length() < 1e-4,
            "the camera is at {camera:?}, the eye at {eye:?}"
        );

        capture_screenshot()?;
        check_colors(WALL)
    }
}

const WALL: &str = r"
      68    4 - #8a887d
     124    4 - #86837b
     304    4 - #4d4b44
     532    4 - #47433b
      12    8 - #3e3d37
     232    8 - #676459
     424    8 - #807f7a
     372   20 - #4f4f41
     476   20 - #9b9a99
     500   32 - #9a9690
     592   40 - #868175
     184   52 - #827d68
     140   68 - #7f7c6f
     568   76 - #7e7c74
       4   80 - #6f6d68
     252   80 - #88867c
     444   80 - #7a786d
      76   84 - #534f41
     332   88 - #33322c
     204   92 - #868479
     504   96 - #3f3c36
     140  116 - #b4b2a8
     172  128 - #b7b1a6
     480  132 - #7d7b76
      12  136 - #949386
     392  136 - #39382e
      52  140 - #77756c
     136  144 - #8a8783
     556  144 - #878175
      88  164 - #474332
     284  168 - #7d7c61
     220  172 - #484537
     508  172 - #302e28
     412  196 - #2f2e26
     484  200 - #5d5a4a
      24  212 - #5d5849
     268  212 - #25231e
     100  224 - #29261d
     188  232 - #2a2922
     364  240 - #27251d
     580  240 - #313124
     436  256 - #3e3c34
      56  260 - #51503f
       4  288 - #828072
     228  304 - #827e6f
      80  312 - #787770
     288  312 - #69665b
     492  312 - #58564a
      40  328 - #474644
     380  328 - #504e43
     436  328 - #79756b
     144  332 - #39382d
     252  360 - #8f8e7b
     544  360 - #9d9884
     592  360 - #333325
       8  364 - #77746f
     484  364 - #7b7765
     328  372 - #4b4841
     224  376 - #767161
      44  404 - #a6a394
     256  404 - #7a766d
     432  404 - #424037
     128  416 - #41402d
     480  416 - #929089
     508  416 - #8f8b7b
     368  424 - #504d44
       4  432 - #7f7d75
     304  436 - #737063
      56  444 - #7e7b68
     196  468 - #7a7666
     448  472 - #817f76
     564  472 - #484532
     500  476 - #888373
     272  480 - #a09e96
     400  480 - #7d7a6f
      36  484 - #9b9888
     324  492 - #514f4a
     116  504 - #59563c
     208  516 - #535147
      68  520 - #7a755e
     396  520 - #4f4a44
     512  524 - #7f7a66
     464  528 - #88867b
     588  528 - #2d2b22
       8  532 - #575548
     260  536 - #787569
     304  540 - #898569
     344  552 - #27251f
      60  572 - #696851
     128  588 - #28271e
     592  588 - #2f2e25
      12  592 - #353328
     208  592 - #2e2c21
     292  592 - #23231d
     416  592 - #2b2a21
     512  592 - #3b3927
";
