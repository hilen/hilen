use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{
        Body, Light, Material, Node, NodeTemplates, Player, SceneCreation, SceneSetup, SceneTest, Wall, scene,
    },
    ui::{Color, Image},
    ui_test::{capture_screenshot, check_colors, hold_key, release_key, set_record_probe_count},
    window::KeyCode,
};

/// Frames of walking, two seconds, more than the wall is away.
const WALK_FRAMES: usize = 120;
const SETTLE_FRAMES: usize = 20;
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
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        let player = |scene: Weak<Self>| from_main(move || scene.player.as_ref().map(Player::position));

        // Spawned a little above the floor, the player settles onto it
        // first.
        for _ in 0..SETTLE_FRAMES {
            wait_for_next_frame();
        }
        let start = player(scene).expect("the scene has a player");

        hold_key(KeyCode::KeyW);
        for _ in 0..WALK_FRAMES {
            wait_for_next_frame();
        }
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
        wait_for_next_frame();
        release_key(KeyCode::Space);
        for _ in 0..6 {
            wait_for_next_frame();
        }
        let airborne = player(scene).expect("the scene has a player");
        anyhow::ensure!(
            airborne.y > stopped.y + 0.15,
            "the jump did not lift the player, it is at {airborne:?}"
        );
        for _ in 0..90 {
            wait_for_next_frame();
        }
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
      12    4 - #8b887b
      60    4 - #96938c
     368    4 - #66635c
     480    4 - #413e37
     176    8 - #636059
     308   20 - #2b2a22
     240   24 - #4c493d
     420   36 - #5a5851
     536   36 - #807e74
      92   40 - #605b50
     592   44 - #646255
     532   52 - #53534e
     128   76 - #7d7d75
     200   84 - #7e7b77
     272   84 - #35332a
     500   88 - #75746a
      60   92 - #6b6653
     400   92 - #78776d
     456  100 - #77746b
     564  100 - #28261f
     348  112 - #4b4a3c
       4  120 - #656055
     116  128 - #b1ab98
     120  128 - #cfc8ba
     212  132 - #807e74
     172  140 - #2a2821
     504  144 - #837e74
     260  156 - #484738
      56  164 - #6e6959
     432  168 - #706d68
     560  176 - #7a766e
       4  188 - #5f5a4d
     416  196 - #28261f
     224  200 - #38352c
     168  204 - #66623d
     348  204 - #2d2c25
     104  208 - #302f27
     580  228 - #4a4838
       4  240 - #20201e
     316  240 - #29271e
     492  240 - #3a382c
      12  244 - #25251d
     420  252 - #4e4c3b
      92  272 - #292924
     224  272 - #656252
     312  288 - #656354
     516  304 - #36352e
     584  304 - #4d4c3d
      24  308 - #848179
     156  316 - #767261
     380  316 - #656355
     452  328 - #837f74
     228  332 - #8e8b83
      84  340 - #3a3a28
     268  340 - #535044
     328  348 - #555348
     588  352 - #504e39
     200  356 - #75746d
     476  368 - #7f7c74
     524  376 - #34332c
      16  384 - #5a5845
     364  396 - #3c3a33
     152  400 - #837e6f
     440  400 - #817f72
     104  404 - #4f4c38
     276  408 - #57564c
      12  428 - #666350
     228  428 - #605d55
     592  428 - #282721
     176  436 - #797670
      64  444 - #6d6a42
     532  456 - #2c2a22
     308  464 - #403e37
     392  464 - #76746a
     480  464 - #49472f
       4  480 - #949289
     224  480 - #9f9d92
     108  484 - #5c5a3f
     168  500 - #817d75
     556  504 - #2c2b25
     276  512 - #848279
     356  520 - #656258
     432  520 - #7b796c
      56  524 - #4c4d3b
     132  528 - #4f4b33
       4  532 - #3a3627
     232  540 - #8a887d
     296  552 - #282620
     524  564 - #272621
       8  576 - #424034
      92  584 - #23231f
     588  584 - #333024
     460  588 - #2a2a22
     176  592 - #322f24
     256  592 - #292723
     380  592 - #27251c
";
