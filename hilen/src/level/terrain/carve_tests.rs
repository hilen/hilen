use super::{
    mesh::{MeshPart, PartKind},
    shape::{CHUNK_SIZE, ChunkKey},
    *,
};
use crate::gm::{color::BLACK, flat::Vertex2D};

pub(super) fn material(surface: bool) -> TerrainMaterial {
    TerrainMaterial {
        fill:      Weak::default(),
        fill_size: 1.0,
        surface:   surface.then(|| TerrainSurface {
            image:  Weak::default(),
            height: 0.5,
            end:    Some(TerrainSurfaceEnd {
                image: Weak::default(),
                width: 0.2,
            }),
        }),
        edge:      Some(TerrainEdge {
            color: BLACK,
            width: 0.1,
        }),
        blend:     None,
    }
}

pub(super) fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Point> {
    vec![
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ]
}

/// Dirt from x -40 to 40 with its top at y 0, over many chunks, and a
/// stone block set into it from x -2 to 2, y -4 to -1.
fn ground() -> (Terrain, MaterialId, MaterialId) {
    let mut terrain = Terrain::new();
    let dirt = terrain.add_material(material(true));
    let stone = terrain.add_material(material(false));
    terrain.add_polygon(dirt, &rect(-40.0, -20.0, 40.0, 0.0), &[]);
    terrain.add_polygon(stone, &rect(-2.0, -4.0, 2.0, -1.0), &[]);
    (terrain, dirt, stone)
}

#[test]
fn material_at_reads_each_material_and_the_air() {
    let (terrain, dirt, stone) = ground();
    assert_eq!(terrain.material_at((10, -5)), Some(dirt));
    assert_eq!(terrain.material_at((0, -2)), Some(stone));
    assert_eq!(terrain.material_at((0, 3)), None);
    assert_eq!(terrain.material_at((50, -5)), None);
}

#[test]
fn holes_in_a_polygon_are_air() {
    let mut terrain = Terrain::new();
    let dirt = terrain.add_material(material(false));
    terrain.add_polygon(dirt, &rect(0.0, 0.0, 10.0, 10.0), &[rect(3.0, 3.0, 6.0, 6.0)]);
    assert_eq!(terrain.material_at((1, 1)), Some(dirt));
    assert_eq!(terrain.material_at((4, 4)), None);
}

#[test]
fn a_carve_cuts_a_circle_out() {
    let (mut terrain, dirt, _) = ground();
    assert!(terrain.carve_circle((10, 0), 2.0, dirt));
    assert_eq!(terrain.material_at((10, -1.5)), None);
    assert_eq!(terrain.material_at((10, -2.5)), Some(dirt));
    assert_eq!(terrain.material_at((12.5, -1)), Some(dirt));
}

#[test]
fn a_carve_in_the_air_changes_nothing() {
    let (mut terrain, dirt, _) = ground();
    let before = terrain.versions();
    assert!(!terrain.carve_circle((10, 5), 2.0, dirt));
    assert_eq!(terrain.versions(), before);
}

#[test]
fn a_carve_takes_only_its_material() {
    let (mut terrain, dirt, stone) = ground();
    assert!(terrain.carve_circle((0, -2), 3.0, dirt));
    assert_eq!(terrain.material_at((0, -2)), Some(stone), "the stone stays");
    assert_eq!(terrain.material_at((0, -4.5)), None, "the dirt under it is gone");

    assert!(terrain.carve_circle((0, -2), 1.0, stone));
    assert_eq!(terrain.material_at((0, -2)), None);
    assert_eq!(terrain.material_at((1.5, -1.5)), Some(stone));
}

/// A carve redraws its chunk and the 8 around it, whose outline band is
/// cut from the same ground, and no chunk further away.
#[test]
fn a_carve_redraws_only_the_chunks_around_it() {
    let (mut terrain, dirt, _) = ground();
    let before = terrain.versions();
    assert!(terrain.carve_circle((12, -4), 1.0, dirt));
    let after = terrain.versions();

    let changed: Vec<_> = before.keys().filter(|key| before[key] != after[key]).copied().collect();
    assert!(changed.contains(&(1, -1)));
    assert!(
        changed.iter().all(|key| (key.0 - 1).abs() <= 1 && (key.1 + 1).abs() <= 1),
        "{changed:?}"
    );
    assert!(before.contains_key(&(3, -1)) && !changed.contains(&(3, -1)));
}

#[test]
fn a_carve_across_a_chunk_border_cuts_both_chunks() {
    let (mut terrain, dirt, _) = ground();
    let border: f32 = CHUNK_SIZE.lossy_convert();
    let before = terrain.versions();
    assert!(terrain.carve_circle((border, -3), 1.5, dirt));
    assert_eq!(terrain.material_at((border - 1.0, -3)), None);
    assert_eq!(terrain.material_at((border + 1.0, -3)), None);
    let after = terrain.versions();
    assert_ne!(before[&(0, -1)], after[&(0, -1)]);
    assert_ne!(before[&(1, -1)], after[&(1, -1)]);
}

/// A small generator so the digs are spread but the same every run.
fn next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    f32::from(u16::try_from(*seed >> 16).unwrap()) / f32::from(u16::MAX)
}

#[test]
fn many_digs_keep_the_points_bounded() {
    let (mut terrain, dirt, _) = ground();
    let mut seed = 7;
    let mut dig = |terrain: &mut Terrain, count: usize| {
        for _ in 0..count {
            let x = next(&mut seed) * 6.0 - 3.0;
            let y = next(&mut seed) * 6.0 - 9.0;
            let radius = 0.3 + next(&mut seed);
            terrain.carve_circle((x, y), radius, dirt);
        }
    };

    dig(&mut terrain, 100);
    let after_100 = terrain.point_count();
    dig(&mut terrain, 400);
    let after_500 = terrain.point_count();
    assert!(
        after_500 < after_100 * 2,
        "{after_100} points after 100 digs, {after_500} after 500"
    );

    for chunk in terrain.chunks.values() {
        for (_, area) in &chunk.areas {
            for polygon in area {
                for ring in std::iter::once(polygon.exterior()).chain(polygon.interiors()) {
                    for line in ring.lines() {
                        let d = line.end - line.start;
                        let on_border = |c: geo::Coord<f64>| {
                            let x = c.x / CHUNK_SIZE;
                            let y = c.y / CHUNK_SIZE;
                            (x - x.round()).abs() < 1e-4 || (y - y.round()).abs() < 1e-4
                        };
                        assert!(
                            d.x.hypot(d.y) >= 0.03 || on_border(line.start) || on_border(line.end),
                            "{line:?} is shorter than the merge distance"
                        );
                    }
                }
            }
        }
    }
}

/// The triangles of a part, each as its middle point and its area.
fn triangles(part: &MeshPart) -> impl Iterator<Item = (Point, f32)> + '_ {
    part.indices.chunks(3).map(|t| {
        let [a, b, c] = [0, 1, 2].map(|i| part.vertices[t[i] as usize].pos);
        let area = ((b - a).x * (c - a).y - (b - a).y * (c - a).x).abs() / 2.0;
        ((a + b + c) / 3.0, area)
    })
}

fn part(mesh: &[MeshPart], kind: PartKind, material: MaterialId) -> Option<&MeshPart> {
    mesh.iter().find(|part| part.kind == kind && part.material == material)
}

/// A chunk of flat ground draws a dirt fill, grass hanging from the top
/// inside the dirt, and an outline band 1 band wide along the top. A dug
/// crater open to the sky grows grass on its floor, and the band follows
/// its wall.
#[test]
fn a_chunk_mesh_has_a_fill_a_surface_and_an_outline() {
    let (mut terrain, dirt, stone) = ground();
    let mesh = terrain.chunk_mesh((0, -1));
    assert!(!part(&mesh, PartKind::Fill, dirt).expect("a dirt fill").indices.is_empty());
    assert!(part(&mesh, PartKind::Surface, stone).is_none());

    let strip = part(&mesh, PartKind::Surface, dirt).expect("a grass strip");
    assert!(
        strip.vertices.iter().all(|v| v.pos.y <= 0.001 && v.pos.y >= -0.501),
        "the strip hangs from the top only"
    );
    for (middle, _) in triangles(strip) {
        assert_eq!(
            terrain.material_at(middle),
            Some(dirt),
            "grass at {middle} is off the dirt"
        );
    }

    let band = part(&mesh, PartKind::Edge, dirt).expect("an outline band");
    let area: f32 = triangles(band)
        .map(|(middle, area)| {
            assert!(
                middle.y < 0.0 && middle.y > -0.1,
                "band at {middle} is off the top"
            );
            area
        })
        .sum();
    // A chunk is 8 wide.
    assert!((area - 0.8).abs() < 0.01, "band area {area}");

    assert!(terrain.carve_circle((4, 0), 2.0, dirt));
    let mesh = terrain.chunk_mesh((0, -1));
    let strip = part(&mesh, PartKind::Surface, dirt).expect("grass");
    assert!(
        strip.vertices.iter().any(|v| v.pos.y < -1.9 && v.uv.y == 0.0),
        "a crater open to the sky grows grass on its floor"
    );
    let band = part(&mesh, PartKind::Edge, dirt).expect("an outline band");
    assert!(
        triangles(band).any(|(middle, _)| (2.0..2.15).contains(&(middle - Point::new(4.0, 0.0)).length())),
        "the band follows the crater wall"
    );
    for (middle, _) in triangles(band) {
        assert_eq!(
            terrain.material_at(middle),
            Some(dirt),
            "band at {middle} is off the dirt"
        );
    }
}

/// A hole whose top just touches a chunk border keeps its whole outline,
/// in the collision edges and in the drawn band.
#[test]
fn a_hole_touching_a_chunk_border_keeps_its_outline() {
    let (mut terrain, dirt, _) = ground();
    // On the chunk border at y -8.
    let top = Point::new(-20.0, -8.0);
    assert!(terrain.carve_circle(top - Point::new(0.0, 4.0), 4.0, dirt));
    let near = |p: Point| (p - top).length() < 1.0;

    let collision: f32 = terrain
        .chunks
        .values()
        .flat_map(|chunk| chunk.edges.iter())
        .filter(|edge| near(edge.a))
        .map(|edge| (edge.b - edge.a).length())
        .sum();
    assert!(
        collision > 1.9,
        "only {collision} of the outline within 1 of the top"
    );

    // About 2 units of outline, a band 0.1 wide.
    let drawn: f32 = [(-3, -2), (-2, -2), (-3, -1), (-2, -1)]
        .into_iter()
        .map(|key| terrain.chunk_mesh(key))
        .flat_map(|mesh| {
            mesh.into_iter()
                .filter(|part| part.kind == PartKind::Edge)
                .flat_map(|part| triangles(&part).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
        .filter(|(middle, _)| near(*middle))
        .map(|(_, area)| area)
        .sum();
    assert!(drawn > 0.17, "only {drawn} of band area within 1 of the top");
}

/// A blend band of dirt lies only in the stone next to it. Where the
/// border reaches the surface at a slant, the band would stick out into
/// the air, and the clip to the stone keeps it in.
#[test]
fn a_blend_spills_into_the_other_material_only() {
    let mut terrain = Terrain::new();
    let dirt = terrain.add_material(material(true).with_blend(0.5));
    let stone = terrain.add_material(material(false));
    terrain.add_polygon(dirt, &rect(-40.0, -20.0, 40.0, 0.0), &[]);
    terrain.add_polygon(
        stone,
        &[
            Point::new(6.0, -20.0),
            Point::new(40.0, -20.0),
            Point::new(40.0, 0.0),
            Point::new(4.0, 0.0),
        ],
        &[],
    );

    let mesh = terrain.chunk_mesh((0, -1));
    assert!(mesh.iter().all(|part| part.kind != PartKind::Blend || part.material == dirt));
    let band = mesh
        .iter()
        .find(|part| part.kind == PartKind::Blend)
        .expect("a dirt blend band");
    for triangle in band.indices.chunks(3) {
        let [a, b, c] = [0, 1, 2].map(|i| band.vertices[triangle[i] as usize].pos);
        let middle = (a + b + c) / 3.0;
        assert_eq!(
            terrain.material_at(middle),
            Some(stone),
            "{middle} is not in the stone"
        );
    }
    assert!(band.vertices.iter().all(|v| (0.0..=1.0).contains(&v.uv.y)));
    assert!(band.vertices.iter().any(|v| v.uv.y < 0.01));
    assert!(band.vertices.iter().any(|v| v.uv.y > 0.99));
}

/// Every part of that kind in the chunks from `min` to `max` keys.
fn parts_in(terrain: &Terrain, min: ChunkKey, max: ChunkKey, kind: PartKind) -> Vec<MeshPart> {
    (min.0..=max.0)
        .flat_map(|x| (min.1..=max.1).map(move |y| (x, y)))
        .flat_map(|key| terrain.chunk_mesh(key))
        .filter(|part| part.kind == kind)
        .collect()
}

/// A pit dug from the top is open to the sky and grows grass on its floor.
/// A tunnel dug under the ground is covered and stays bare.
#[test]
fn a_pit_grows_grass_and_a_tunnel_does_not() {
    let (mut terrain, dirt, _) = ground();
    assert!(terrain.carve_circle((20, 0), 2.0, dirt));
    assert!(terrain.carve_circle((-20, -10), 2.0, dirt));
    let strips = parts_in(&terrain, (-4, -3), (3, 0), PartKind::Surface);
    let grass_at = |x0: f32, x1: f32, y0: f32, y1: f32| {
        strips
            .iter()
            .flat_map(|part| part.vertices.iter())
            .any(|v| v.uv.y == 0.0 && (x0..x1).contains(&v.pos.x) && (y0..y1).contains(&v.pos.y))
    };
    assert!(grass_at(19.5, 20.5, -2.1, -1.8), "no grass on the pit floor");
    assert!(!grass_at(-21.0, -19.0, -12.1, -11.8), "grass on the tunnel floor");
}

/// Grass stops where its ledge meets a wall, and there the run gets an end
/// piece, on the lower ledge and on the upper one. A run that crosses a
/// chunk border gets none there.
#[test]
fn a_grass_run_ends_at_a_wall_and_not_at_a_chunk_border() {
    let (mut terrain, dirt, _) = ground();
    terrain.add_polygon(dirt, &rect(10.0, -1.0, 40.0, 4.0), &[]);
    let ends: Vec<Point> = parts_in(&terrain, (-5, -1), (4, 0), PartKind::SurfaceEnd)
        .iter()
        .flat_map(|part| part.vertices.iter())
        .filter(|v| v.uv.x == 0.0 && v.uv.y == 0.0)
        .map(|v| v.pos)
        .collect();
    let has_end = |at: Point| ends.iter().any(|end| (*end - at).length() < 0.01);
    assert!(
        has_end(Point::new(10.0, 0.0)),
        "no end on the lower ledge, ends {ends:?}"
    );
    assert!(
        has_end(Point::new(10.0, 4.0)),
        "no end on the upper ledge, ends {ends:?}"
    );
    for border in [-32.0, -24.0, -16.0, -8.0, 0.0, 8.0, 16.0, 24.0, 32.0] {
        assert!(
            !has_end(Point::new(border, 0.0)) && !has_end(Point::new(border, 4.0)),
            "an end at the border {border}"
        );
    }
}

/// Grass on ground whose top sits just over a chunk border hangs down into
/// the chunk below and keeps its full height there.
#[test]
fn grass_hangs_across_a_chunk_border() {
    let mut terrain = Terrain::new();
    let dirt = terrain.add_material(material(true));
    // The top at -7.8, the chunk border at -8, the strip 0.5 tall.
    terrain.add_polygon(dirt, &rect(-40.0, -20.0, 40.0, -7.8), &[]);
    let area: f32 = terrain
        .chunk_mesh((0, -1))
        .iter()
        .filter(|part| part.kind == PartKind::Surface)
        .flat_map(|part| triangles(part).map(|(_, area)| area).collect::<Vec<_>>())
        .sum();
    // A chunk is 8 wide.
    assert!((area - 8.0 * 0.5).abs() < 0.01, "strip area {area}");
}

/// A crater dug into flat ground grows grass on its floor, and the run of
/// grass gets an end piece on each side where the crater wall gets too
/// steep to walk.
#[test]
fn a_crater_floor_gets_grass_ends_at_its_steep_walls() {
    let mut terrain = Terrain::new();
    let dirt = terrain.add_material(material(true));
    terrain.add_polygon(dirt, &rect(-40.0, -30.0, 40.0, 5.0), &[]);
    assert!(terrain.carve_circle((0, 5), 6.0, dirt));
    let ends: Vec<Point> = parts_in(&terrain, (-2, -2), (1, 1), PartKind::SurfaceEnd)
        .iter()
        .flat_map(|part| part.vertices.iter())
        .filter(|v| v.uv.x == 0.0 && v.uv.y == 0.0)
        .map(|v| v.pos)
        .collect();
    let floor_ends = ends.iter().filter(|end| end.y < 4.0).count();
    assert!(floor_ends >= 2, "crater floor ends {ends:?}");
}

/// An end piece on a slope hangs straight down, so its u runs along x:
/// the vertices straight under the end have u 0, the outline column of the
/// image stays a full column down the side. The floor ends lie at x
/// 4.596 either side of the crater middle.
#[test]
fn a_sloped_grass_end_keeps_its_outline_column() {
    let mut terrain = Terrain::new();
    let dirt = terrain.add_material(material(true));
    terrain.add_polygon(dirt, &rect(-40.0, -30.0, 40.0, 5.0), &[]);
    assert!(terrain.carve_circle((0, 5), 6.0, dirt));
    let ends = parts_in(&terrain, (-2, -2), (1, 1), PartKind::SurfaceEnd);
    let column: Vec<&Vertex2D> = ends
        .iter()
        .flat_map(|part| part.vertices.iter())
        .filter(|v| (v.pos.x.abs() - 4.596_266_7).abs() < 0.001 && v.pos.y < 1.0)
        .collect();
    assert!(!column.is_empty(), "no vertex under a floor end");
    assert!(column.iter().all(|v| v.uv.x == 0.0), "{column:?}");
}
