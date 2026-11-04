use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3},
    refs::Weak,
    scene::{Body, Camera, Node, NodeTemplates, SceneCreation, SceneSetup, SceneTest, Wall, scene},
    ui::Color,
    ui_test::checkpoint,
};

const TOLERANCE: f32 = 1e-4;
const PLACE: Vec3 = Vec3::new(1.5, 1.0, -1.0);
const BODY_PLACE: Vec3 = Vec3::new(-1.5, 1.0, -1.0);

fn turn() -> Quat {
    Quat::from_rotation_y(0.7)
}

/// A crate that is a body and a crate that is a wall, both moved and turned
/// by `set_pose`, then the body turned and moved by the 2 calls apart. The
/// scene is stepped, so no physics step runs between a move and its check.
#[scene]
#[derive(Default)]
struct NodePose {
    body: Weak<Body>,
    wall: Weak<Wall>,
}

impl SceneSetup for NodePose {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.body = self.make_node::<Body>(Shape3::cube(1.0), Vec3::new(-1.5, 0.5, 0.0));
        self.body.set_color(Color::hex("#e67e22"));
        self.wall = self.make_node::<Wall>(Shape3::cube(1.0), Vec3::new(1.5, 0.5, 0.0));
        self.wall.set_color(Color::hex("#2e86de"));
        self.camera = Camera {
            position: Vec3::new(0.0, 4.0, 6.0),
            target: Vec3::new(0.0, 0.5, -0.5),
            ..Camera::default()
        };
    }
}

/// Whether `node` stands at `place` turned by `turn()`, from the physics.
fn posed(what: &str, node: Weak<dyn Node>, place: Vec3) -> Result<()> {
    let (position, rotation) = from_main(move || (node.position(), node.rotation()));
    ensure!(
        position.distance(place) < TOLERANCE,
        "{what} at {place}, got {position}"
    );
    ensure!(
        rotation.angle_between(turn()) < TOLERANCE,
        "{what} turned by 0.7 about y, got {rotation}"
    );
    Ok(())
}

impl SceneTest for NodePose {
    fn stepped() -> bool {
        true
    }

    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        let (body, wall) = from_main(move || (scene.body.weak_node(), scene.wall.weak_node()));

        // A world that moves under the player sets every pose each frame, 1
        // write to the physics where the 2 calls took 2.
        from_main(move || {
            scene.body.set_pose(BODY_PLACE, turn());
            scene.wall.set_pose(PLACE, turn());
        });
        posed("the body after set_pose", body, BODY_PLACE)?;
        posed("the wall after set_pose", wall, PLACE)?;
        checkpoint("set_pose moves and turns a body and a wall")?;

        from_main(move || {
            scene.body.set_pose(Vec3::new(-1.5, 0.5, 0.0), Quat::IDENTITY);
            scene.body.set_rotation(turn()).set_position(BODY_PLACE);
        });
        posed("the body turned then moved", body, BODY_PLACE)?;
        checkpoint("a body turned then moved keeps its turn")
    }
}
