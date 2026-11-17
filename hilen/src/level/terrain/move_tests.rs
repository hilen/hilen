use super::{
    carve_tests::{material, rect},
    *,
};

const HALF: Point = Point::new(0.4, 0.4);

fn with(polygons: &[Vec<Point>]) -> (Terrain, MaterialId) {
    let mut terrain = Terrain::new();
    let dirt = terrain.add_material(material(true));
    for points in polygons {
        terrain.add_polygon(dirt, points, &[]);
    }
    (terrain, dirt)
}

/// Flat ground from x -20 to 20, its top at y 0.
fn flat() -> (Terrain, MaterialId) {
    with(&[rect(-20.0, -10.0, 20.0, 0.0)])
}

/// Flat ground at y 0 up to x 0, a ramp at `degrees` up to `height`, and
/// flat ground again on top of it.
fn ramp(degrees: f32) -> (Terrain, f32, f32) {
    let height = 5.0;
    let run = height / degrees.to_radians().tan();
    let (terrain, _) = with(&[vec![
        Point::new(-20.0, -10.0),
        Point::new(40.0, -10.0),
        Point::new(40.0, height),
        Point::new(run, height),
        Point::new(0.0, 0.0),
        Point::new(-20.0, 0.0),
    ]]);
    (terrain, run, height)
}

/// How high the ramp ground is at `x`.
fn ramp_height(x: f32, run: f32, height: f32) -> f32 {
    (x / run).clamp(0.0, 1.0) * height
}

/// Moves the box down in small steps like gravity, returns where it ended
/// and whether any step landed it.
fn fall(terrain: &Terrain, mut center: Point, drop_through: bool) -> (Point, bool) {
    let mut landed = false;
    for _ in 0..200 {
        let moved = terrain.move_box(center, HALF, Point::new(0.0, -0.2), drop_through);
        center = moved.position;
        landed |= moved.floor;
    }
    (center, landed)
}

fn bottom(center: Point) -> f32 {
    center.y - HALF.y
}

#[test]
fn a_falling_box_lands_on_flat_ground() {
    let (terrain, _) = flat();
    let (center, landed) = fall(&terrain, Point::new(2.5, 5.0), false);
    assert!(landed);
    assert!(bottom(center).abs() < 0.01, "{center:?}");
    assert!(terrain.stands_on_ground(center, HALF));
    assert!(!terrain.stands_on_ground(center + Point::new(0.0, 0.5), HALF));
}

#[test]
fn a_box_walks_across_chunk_borders() {
    let (terrain, _) = flat();
    let mut center = Point::new(-12.0, HALF.y + 0.001);
    for _ in 0..240 {
        let moved = terrain.move_box(center, HALF, Point::new(0.1, -0.05), false);
        assert!(!moved.wall, "a chunk border is not a wall, {moved:?}");
        center = moved.position;
    }
    assert!((center.x - 12.0).abs() < 0.01, "{center:?}");
    assert!(bottom(center).abs() < 0.01, "{center:?}");
}

#[test]
fn a_box_walks_up_a_gentle_slope() {
    let (terrain, run, height) = ramp(30.0);
    let mut center = Point::new(-3.0, HALF.y + 0.001);
    for _ in 0..200 {
        let moved = terrain.move_box(center, HALF, Point::new(0.1, -0.05), false);
        assert!(!moved.wall, "{moved:?}");
        center = moved.position;
        let ground = ramp_height(center.x, run, height);
        assert!(
            (bottom(center) - ground).abs() < 0.01,
            "the middle rides the slope, {center:?}"
        );
    }
    assert!(center.x > run, "{center:?}");
}

#[test]
fn a_box_walks_down_a_gentle_slope_without_bouncing() {
    let (terrain, run, height) = ramp(45.0);
    let mut center = Point::new(run + 3.0, height + HALF.y + 0.001);
    for _ in 0..120 {
        let moved = terrain.move_box(center, HALF, Point::new(-0.1, -0.01), false);
        center = moved.position;
        assert!(moved.floor, "{moved:?}");
        assert!(terrain.stands_on_ground(center, HALF), "{center:?}");
        let ground = ramp_height(center.x, run, height);
        assert!((bottom(center) - ground).abs() < 0.01, "{center:?}");
    }
    assert!(center.x < 0.0, "{center:?}");
}

#[test]
fn a_steep_slope_is_a_wall() {
    let (terrain, _, _) = ramp(60.0);
    let mut center = Point::new(-3.0, HALF.y + 0.001);
    let mut stopped = false;
    for _ in 0..100 {
        let moved = terrain.move_box(center, HALF, Point::new(0.1, -0.05), false);
        stopped |= moved.wall;
        center = moved.position;
    }
    assert!(stopped);
    assert!(
        center.x + HALF.x <= 0.0 && center.x + HALF.x > -0.01,
        "{center:?}"
    );
    assert!(bottom(center).abs() < 0.01, "{center:?}");
}

#[test]
fn the_slope_limit_is_a_setting() {
    let (mut terrain, run, _) = ramp(60.0);
    terrain.slope_limit = 65.0;
    let mut center = Point::new(-3.0, HALF.y + 0.001);
    for _ in 0..100 {
        let moved = terrain.move_box(center, HALF, Point::new(0.1, -0.05), false);
        assert!(!moved.wall, "{moved:?}");
        center = moved.position;
    }
    assert!(center.x > run, "{center:?}");
}

#[test]
fn a_box_slides_down_a_steep_slope() {
    let (terrain, run, height) = ramp(60.0);
    let start = Point::new(run * 0.5, height + 1.0);
    let (center, landed) = fall(&terrain, start, false);
    assert!(landed, "it lands at the foot");
    assert!(center.x < start.x - 0.5, "{center:?}");
    assert!(bottom(center).abs() < 0.01, "{center:?}");
}

#[test]
fn a_fast_box_does_not_tunnel() {
    let (terrain, _) = with(&[
        rect(-20.0, -10.0, 20.0, 0.0),
        rect(5.0, 0.0, 5.05, 5.0),
        rect(-2.0, 3.0, 2.0, 3.03),
    ]);

    let moved = terrain.move_box(
        Point::new(3.0, HALF.y + 0.001),
        HALF,
        Point::new(40.0, 0.0),
        false,
    );
    assert!(moved.wall, "{moved:?}");
    assert!(
        moved.position.x + HALF.x <= 5.0 && moved.position.x + HALF.x > 4.99,
        "{moved:?}"
    );

    let moved = terrain.move_box(Point::new(0.0, 20.0), HALF, Point::new(0.0, -40.0), false);
    assert!(moved.floor, "{moved:?}");
    assert!((bottom(moved.position) - 3.03).abs() < 0.01, "{moved:?}");
}

#[test]
fn a_ceiling_stops_a_jump() {
    let (terrain, _) = with(&[rect(-20.0, -10.0, 20.0, 0.0), rect(-3.0, 3.0, 3.0, 4.0)]);
    let moved = terrain.move_box(Point::new(0.0, HALF.y + 0.001), HALF, Point::new(0.0, 5.0), false);
    assert!(moved.ceiling, "{moved:?}");
    assert!((moved.position.y + HALF.y - 3.0).abs() < 0.01, "{moved:?}");
}

#[test]
fn a_box_falls_into_a_carved_hole() {
    let (mut terrain, dirt) = flat();
    let (center, _) = fall(&terrain, Point::new(0.0, 2.0), false);
    assert!(terrain.stands_on_ground(center, HALF));

    assert!(terrain.carve_circle((0, 0), 2.0, dirt));
    assert!(!terrain.stands_on_ground(center, HALF));
    let (center, landed) = fall(&terrain, center, false);
    assert!(landed);
    assert!((bottom(center) + 2.0).abs() < 0.01, "{center:?}");
}

#[test]
fn a_box_stands_over_a_crack_narrower_than_it() {
    let (mut terrain, dirt) = flat();
    assert!(terrain.carve_polygon(&rect(-0.2, -3.0, 0.2, 0.5), dirt));
    let (center, landed) = fall(&terrain, Point::new(0.0, 2.0), false);
    assert!(landed);
    assert!(bottom(center).abs() < 0.01, "{center:?}");
    assert!(terrain.stands_on_ground(center, HALF));
}

/// Flat ground at y 0 and a one way segment from x 1 to 5 at y 5.
fn ledge() -> Terrain {
    let (mut terrain, _) = flat();
    terrain.add_one_way((1, 5), (5, 5));
    terrain
}

#[test]
fn a_falling_box_lands_on_a_one_way_segment() {
    let terrain = ledge();
    let (center, landed) = fall(&terrain, Point::new(2.5, 8.0), false);
    assert!(landed);
    assert!((bottom(center) - 5.0).abs() < 0.01, "{center:?}");
    assert!(terrain.stands_on_ground(center, HALF));
}

#[test]
fn a_jump_passes_up_through_a_one_way_segment_and_lands_on_it() {
    let terrain = ledge();
    let moved = terrain.move_box(Point::new(2.5, HALF.y + 0.001), HALF, Point::new(0.0, 6.0), false);
    assert!(!moved.hit(), "{moved:?}");
    assert!((moved.position.y - 6.401).abs() < 0.01, "{moved:?}");

    let (center, landed) = fall(&terrain, moved.position, false);
    assert!(landed);
    assert!((bottom(center) - 5.0).abs() < 0.01, "{center:?}");
}

#[test]
fn a_walking_box_passes_through_a_one_way_segment() {
    let terrain = ledge();
    let moved = terrain.move_box(Point::new(-1.0, 5.0), HALF, Point::new(8.0, 0.0), false);
    assert!(!moved.hit(), "{moved:?}");
    assert!((moved.position.x - 7.0).abs() < 0.01, "{moved:?}");
    assert!((moved.position.y - 5.0).abs() < 0.01, "{moved:?}");
}

#[test]
fn a_box_drops_through_a_one_way_segment() {
    let terrain = ledge();
    let (on_top, _) = fall(&terrain, Point::new(2.5, 8.0), false);
    assert!(terrain.stands_on_ground(on_top, HALF));

    let (center, landed) = fall(&terrain, on_top, true);
    assert!(landed, "the ground stops it");
    assert!(bottom(center).abs() < 0.01, "{center:?}");
}

#[test]
fn a_box_walks_along_a_one_way_segment_and_off_its_end() {
    let terrain = ledge();
    let (mut center, _) = fall(&terrain, Point::new(2.5, 8.0), false);
    for _ in 0..20 {
        center = terrain.move_box(center, HALF, Point::new(0.1, -0.05), false).position;
        assert!((bottom(center) - 5.0).abs() < 0.01, "{center:?}");
    }
    for _ in 0..40 {
        center = terrain.move_box(center, HALF, Point::new(0.1, -0.3), false).position;
    }
    assert!(
        bottom(center).abs() < 0.01,
        "it fell off the end to the ground, {center:?}"
    );
}
