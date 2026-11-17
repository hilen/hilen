use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Ray, Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, ColliderShape, Node, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sky, Wall,
        scene,
    },
    ui::Color,
    ui_test::checkpoint,
};

/// How close a distance or a point must come to the one worked out by
/// hand from the layout.
const TOLERANCE: f32 = 0.01;
const PILLAR: Vec3 = Vec3::new(-3.0, 0.5, -1.0);
/// The slab's front face faces the player at z 0.1.
const SLAB: Vec3 = Vec3::new(0.0, 1.1, 0.0);
const SWEEP_FROM: Vec3 = Vec3::new(0.8, 0.5, 3.0);
const SWEEP_RADIUS: f32 = 0.3;
const HITBOX: Vec3 = Vec3::new(3.0, 0.6, 0.0);
const HITBOX_SIZE: Vec3 = Vec3::new(1.6, 1.0, 1.6);
const PLAYER: Vec3 = Vec3::new(0.0, 0.9, 6.0);
const PLAYER_SETTLE_FRAMES: usize = 30;

/// Scene queries against static walls. A ray straight down stops on the
/// pillar's top, then passes it by when told to skip it and stops on the
/// floor, the pillar turned see through to show the mark under it. A
/// ball swept at the slab stops just short of it. A box hitbox finds the
/// two balls inside it and not the third, then only one when the other
/// is skipped. Then a player comes in and the view moves to its eyes, and
/// a ray from there meets the slab, not the player's own capsule around
/// them. Every hit gets a yellow marker, the
/// stopped sweep a translucent ball, and every ball the hitbox finds turns
/// red.
#[scene]
#[derive(Default)]
struct SceneQueries {
    floor:  Weak<Wall>,
    pillar: Weak<Wall>,
    slab:   Weak<Wall>,
    balls:  Vec<Weak<Wall>>,
}

impl SceneQueries {
    fn mark(&mut self, point: Vec3) {
        self.make_node::<Prop>(Shape3::Ball(0.12), point)
            .set_color(Color::hex("#ffd400"));
    }
}

impl SceneSetup for SceneQueries {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.floor = self.make_node::<Wall>(Shape3::Plane(20.0), Vec3::ZERO);
        self.floor.set_color(Color::hex("#9aa3aa")).set_roughness(0.9);

        self.pillar = self.make_node::<Wall>(Shape3::cube(1.0), PILLAR);
        self.pillar.set_color(Color::hex("#2e86de"));

        self.slab = self.make_node::<Wall>(Shape3::cuboid(3.0, 2.2, 0.2), SLAB);
        self.slab.set_color(Color::hex("#8e8e8e"));

        for z in [-0.5, 0.5, 2.0] {
            let mut ball = self.make_node::<Wall>(Shape3::Ball(0.3), Vec3::new(3.0, 0.3, z));
            ball.set_color(Color::hex("#f5f5f5"));
            self.balls.push(ball);
        }

        self.make_node::<Prop>(Shape3::Box(HITBOX_SIZE), HITBOX)
            .set_color(Color::hex("#e6503a").with_alpha(0.25));

        // High and to the side, every query in sight until the player
        // comes in for the last one and the view moves to its eyes.
        self.camera = Camera {
            position: Vec3::new(1.5, 5.5, 8.0),
            target: Vec3::new(0.2, 0.5, 0.3),
            ..Camera::default()
        };
    }
}

impl SceneTest for SceneQueries {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        // The broad phase takes the colliders in on the first step.
        wait_for_next_frame();

        let down = Ray {
            origin:    Vec3::new(PILLAR.x, 5.0, PILLAR.z),
            direction: Vec3::NEG_Y,
        };
        let hit = from_main(move || scene.cast_ray(down, 20.0, &[]));
        let hit = hit.ok_or_else(|| anyhow::anyhow!("the ray down found nothing"))?;
        let pillar = from_main(move || scene.pillar.weak_node());
        ensure!(hit.node == pillar, "the ray down stops on the pillar");
        ensure!(
            (hit.distance - 4.0).abs() < TOLERANCE,
            "the pillar top is 4 below, got {}",
            hit.distance
        );
        ensure!(
            hit.normal.distance(Vec3::Y) < TOLERANCE,
            "the top faces up, got {}",
            hit.normal
        );
        from_main(move || scene.mark(hit.point));
        checkpoint("a ray straight down stops on the pillar top")?;

        let hit = from_main(move || scene.cast_ray(down, 20.0, &[pillar]));
        let hit = hit.ok_or_else(|| anyhow::anyhow!("the ray skipping the pillar found nothing"))?;
        let floor = from_main(move || scene.floor.weak_node());
        ensure!(hit.node == floor, "skipping the pillar the ray reaches the floor");
        ensure!(
            (hit.distance - 5.0).abs() < TOLERANCE,
            "the floor is 5 below, got {}",
            hit.distance
        );
        from_main(move || {
            scene.mark(hit.point);
            // See through the pillar to the marker on the floor under it.
            scene.pillar.set_color(Color::hex("#2e86de").with_alpha(0.3));
        });
        checkpoint("skipping the pillar, the same ray reaches the floor under it")?;

        let ball = ColliderShape::Ball(SWEEP_RADIUS);
        let hit =
            from_main(move || scene.cast_shape(&ball, SWEEP_FROM, Quat::IDENTITY, Vec3::NEG_Z, 10.0, &[]));
        let hit = hit.ok_or_else(|| anyhow::anyhow!("the sweep found nothing"))?;
        let slab = from_main(move || scene.slab.weak_node());
        ensure!(hit.node == slab, "the swept ball runs into the slab");
        let expected = SWEEP_FROM.z - 0.1 - SWEEP_RADIUS;
        ensure!(
            (hit.distance - expected).abs() < TOLERANCE,
            "the ball stops {expected} along, got {}",
            hit.distance
        );
        ensure!(
            (hit.point.z - 0.1).abs() < TOLERANCE,
            "it touches the slab's face, got {}",
            hit.point
        );
        let stop = SWEEP_FROM + Vec3::NEG_Z * hit.distance;
        from_main(move || {
            scene.mark(hit.point);
            scene
                .make_node::<Prop>(Shape3::Ball(SWEEP_RADIUS), stop)
                .set_color(Color::hex("#ff9a1a").with_alpha(0.5));
        });
        checkpoint("a ball swept at the slab stops just short of it")?;

        hitbox_and_eyes(scene, slab)
    }
}

/// The hitbox finds the balls inside it, then a player comes in and a ray
/// from its eyes meets the slab, not its own capsule.
fn hitbox_and_eyes(mut scene: Weak<SceneQueries>, slab: Weak<dyn Node>) -> Result<()> {
    let hitbox = ColliderShape::Box(HITBOX_SIZE);
    let shape = hitbox.clone();
    let inside = from_main(move || scene.overlapping(&shape, HITBOX, Quat::IDENTITY, &[]));
    let balls = from_main(move || scene.balls.iter().map(|ball| ball.weak_node()).collect::<Vec<_>>());
    ensure!(
        inside.len() == 2,
        "the hitbox finds 2 nodes, got {}",
        inside.len()
    );
    ensure!(
        inside.contains(&balls[0]) && inside.contains(&balls[1]),
        "the hitbox finds the two balls inside it"
    );
    from_main(move || {
        for mut node in inside {
            node.set_color(Color::hex("#e74c3c"));
        }
    });
    checkpoint("the hitbox finds the two balls inside it, they turn red")?;

    let skipped = balls[0];
    let inside = from_main(move || scene.overlapping(&hitbox, HITBOX, Quat::IDENTITY, &[skipped]));
    ensure!(
        inside == [balls[1]],
        "skipping one ball the hitbox finds only the other"
    );

    from_main(move || {
        scene.add_player(PLAYER);
    });
    // The player settles onto the floor before its eyes are asked for.
    for _ in 0..PLAYER_SETTLE_FRAMES {
        wait_for_next_frame();
    }
    let eye = from_main(move || {
        let player = scene.player.as_ref().expect("the scene has a player");
        player.position() + Vec3::Y * player.eye_height
    });
    let look = Ray {
        origin:    eye,
        direction: Vec3::NEG_Z,
    };
    let hit = from_main(move || scene.cast_ray(look, 20.0, &[]));
    let hit = hit.ok_or_else(|| anyhow::anyhow!("the ray from the eyes found nothing"))?;
    ensure!(
        hit.node == slab,
        "from the eyes the ray meets the slab, not the player"
    );
    ensure!(
        (hit.distance - (eye.z - 0.1)).abs() < TOLERANCE,
        "the slab is {} ahead, got {}",
        eye.z - 0.1,
        hit.distance
    );
    from_main(move || scene.mark(hit.point));
    checkpoint("a ray from the player's eyes meets the slab, not the player")
}
