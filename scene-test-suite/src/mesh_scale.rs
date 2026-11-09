use anyhow::{Result, anyhow, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Ray, Shape3, Vec3},
    refs::Weak,
    scene::{Camera, Node, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Wall, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const PLAIN: Vec3 = Vec3::new(-1.5, 0.5, 0.0);
const STRETCHED: Vec3 = Vec3::new(1.5, 0.5, 0.0);
const TOLERANCE: f32 = 0.01;

/// 2 cubes of 1 model, the left as it is and the right stretched by
/// `set_mesh_scale` to twice its width and half its height, then back. The
/// drawing stretches and the collider stays a cube: a ray from above stops
/// on the top of the unstretched cube.
#[scene]
#[derive(Default)]
struct MeshScale {
    stretched: Weak<Wall>,
}

impl SceneSetup for MeshScale {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 3.0, 6.0),
            target: Vec3::new(0.0, 0.5, 0.0),
            ..Camera::default()
        };
        self.ambient = Color::hex("#606060");
        self.make_node::<Prop>(Shape3::Plane(20.0), Vec3::ZERO)
            .set_color(Color::hex("#808080"))
            .set_roughness(0.9);
        self.make_node::<Prop>(Shape3::cube(1.0), PLAIN)
            .set_color(Color::hex("#2e86de"));
        self.stretched = self.make_node::<Wall>(Shape3::cube(1.0), STRETCHED);
        self.stretched.set_color(Color::hex("#e67e22"));
    }
}

impl SceneTest for MeshScale {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        check_colors(SAME)?;

        from_main(move || {
            scene.stretched.set_mesh_scale(Vec3::new(2.0, 0.5, 1.0));
        });
        wait_for_next_frame();
        wait_for_next_frame();
        check_colors(STRETCHED_COLORS)?;

        let down = Ray {
            origin:    STRETCHED + Vec3::new(0.0, 5.0, 0.0),
            direction: Vec3::NEG_Y,
        };
        let stretched = from_main(move || scene.stretched.weak_node());
        let hit = from_main(move || scene.cast_ray(down, 20.0, &[]))
            .ok_or_else(|| anyhow!("the ray down found nothing"))?;
        ensure!(hit.node == stretched, "the ray stops on the stretched cube");
        ensure!(
            (hit.point.y - 1.0).abs() < TOLERANCE,
            "the collider keeps the top of the cube at 1, got {}",
            hit.point.y
        );

        from_main(move || {
            scene.stretched.set_mesh_scale(Vec3::ONE);
        });
        wait_for_next_frame();
        wait_for_next_frame();
        check_colors(SAME)
    }
}

const SAME: &str = r"
       4    4 - #597c95
     312    4 - #597c95
     592    4 - #597c95
     160   80 - #597c95
     452  104 - #597c95
     304  144 - #597c95
       4  168 - #597c95
     592  176 - #597c95
     148  252 - #2e76c2
     168  252 - #2e76c2
     188  252 - #2e76c2
     216  252 - #2e76c2
     444  252 - #c96f26
     412  256 - #c96f26
     380  260 - #583013
     160  264 - #2e76c2
     144  272 - #2e76c2
     172  272 - #2e76c2
     200  272 - #2e76c2
     124  276 - #2662a1
     376  276 - #583013
     476  280 - #a75c20
     384  288 - #583013
     424  288 - #a75c20
     376  292 - #583013
     300  300 - #737373
     380  300 - #583013
     224  304 - #22568e
     456  304 - #a75c20
     140  308 - #2662a1
     376  312 - #583013
     172  316 - #2662a1
     384  324 - #583013
     432  324 - #a75c20
     220  336 - #22568e
     468  336 - #a75c20
     132  340 - #2662a1
     380  340 - #583013
     164  348 - #2662a1
     196  352 - #2662a1
     412  352 - #a75c20
       4  432 - #737373
     592  432 - #737373
     448  512 - #737373
     156  516 - #737373
       4  592 - #737373
     304  592 - #737373
     592  592 - #737373
";

const STRETCHED_COLORS: &str = r"
       4    4 - #597c95
     416    4 - #597c95
     212   44 - #597c95
     592  112 - #597c95
     120  128 - #597c95
     336  132 - #597c95
     464  140 - #597c95
       4  168 - #597c95
     148  252 - #2e76c2
     168  252 - #2e76c2
     188  252 - #2e76c2
     216  252 - #2e76c2
     160  264 - #2e76c2
     380  268 - #c96f26
     424  268 - #c96f26
     476  268 - #c96f26
     144  272 - #2e76c2
     172  272 - #2e76c2
     200  272 - #2e76c2
     124  276 - #2662a1
     340  284 - #583013
     340  288 - #583013
     340  292 - #583013
     340  296 - #583013
     300  300 - #737373
     340  300 - #583013
     224  304 - #22568e
     340  304 - #583013
     428  304 - #a75c20
     516  304 - #a75c20
     140  308 - #2662a1
     340  308 - #583013
     172  316 - #2662a1
     340  320 - #583013
     396  324 - #a75c20
     452  332 - #a75c20
     492  332 - #a75c20
     220  336 - #22568e
     132  340 - #2662a1
     164  348 - #2662a1
     196  352 - #2662a1
       4  432 - #737373
     592  440 - #737373
     456  500 - #737373
     160  520 - #737373
       4  592 - #737373
     320  592 - #737373
     592  592 - #737373
";
