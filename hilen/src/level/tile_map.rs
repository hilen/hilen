use crate::{
    deps::refs::{Own, Weak},
    gm::{
        LossyConvert,
        flat::{Point, Rect},
    },
    level::{LevelBase, LevelManager},
    window::image::{Image, ToImage},
};

/// Which kind a cell holds, an index from `TileMap::add_kind`.
/// `TileId::EMPTY` is no tile: nothing drawn and nothing solid.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TileId(u16);

impl TileId {
    pub const EMPTY: Self = Self(0);
}

/// What a tile looks like and whether a box stops at it. A kind with no
/// image is not drawn, an invisible wall.
#[derive(Debug, Clone)]
pub struct TileKind {
    pub image: Weak<Image>,
    pub solid: bool,
}

impl TileKind {
    pub fn solid(image: impl ToImage) -> Self {
        Self {
            image: image.to_image(),
            solid: true,
        }
    }

    pub fn decor(image: impl ToImage) -> Self {
        Self {
            image: image.to_image(),
            solid: false,
        }
    }
}

/// Where `TileMap::move_box` stopped a box.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct BoxMove {
    /// The new center of the box.
    pub position: Point,
    pub wall:     bool,
    pub ceiling:  bool,
    pub floor:    bool,
}

impl BoxMove {
    pub fn hit(&self) -> bool {
        self.wall || self.ceiling || self.floor
    }
}

/// How far a box stays off a tile it stopped at. A box resting exactly
/// on a tile edge must not count as inside that tile.
const EDGE: f32 = 0.001;

/// A grid of tiles drawn as one layer. The cell `(x, y)` covers the square
/// from `origin + (x, y) * tile_size` to one tile size more, `y` grows up
/// like the level does. Only the cells on screen are drawn, and the solid
/// ones answer box queries, so an actor can walk and land on the grid
/// without rapier.
#[derive(Debug)]
pub struct TileMap {
    width:  usize,
    height: usize,
    kinds:  Vec<TileKind>,
    tiles:  Vec<TileId>,

    pub origin:        Point,
    pub tile_size:     f32,
    /// Whether the space around the grid stops a box, true by default so
    /// nothing falls out of the world.
    pub outside_solid: bool,
    /// Depth like `SpriteData::z_position`, `add_tile_map` puts a layer
    /// behind every sprite and behind the layers added before it.
    pub z_position:    f32,
}

impl TileMap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            kinds: vec![],
            tiles: vec![TileId::EMPTY; width * height],
            origin: Point::default(),
            tile_size: 1.0,
            outside_solid: true,
            z_position: LevelManager::default_z_position(),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn add_kind(&mut self, kind: TileKind) -> TileId {
        self.kinds.push(kind);
        TileId(u16::try_from(self.kinds.len()).expect("a tile map takes at most 65535 kinds"))
    }

    pub fn kind(&self, id: TileId) -> Option<&TileKind> {
        let index = usize::from(id.0).checked_sub(1)?;
        self.kinds.get(index)
    }

    pub fn get(&self, x: usize, y: usize) -> TileId {
        assert!(
            x < self.width && y < self.height,
            "tile {x} {y} is outside the map"
        );
        self.tiles[y * self.width + x]
    }

    pub fn set(&mut self, x: usize, y: usize, id: TileId) {
        assert!(
            x < self.width && y < self.height,
            "tile {x} {y} is outside the map"
        );
        assert!(
            id == TileId::EMPTY || self.kind(id).is_some(),
            "tile id {} is not a kind of this map",
            id.0
        );
        self.tiles[y * self.width + x] = id;
    }

    /// Fills the rect of cells from `(x, y)`, `width` by `height` of them.
    pub fn fill(&mut self, x: usize, y: usize, width: usize, height: usize, id: TileId) {
        for cy in y..y + height {
            for cx in x..x + width {
                self.set(cx, cy, id);
            }
        }
    }

    /// The cell a level point falls into, cells outside the grid included.
    pub fn cell_at(&self, point: Point) -> (i32, i32) {
        (
            cell_coord((point.x - self.origin.x) / self.tile_size),
            cell_coord((point.y - self.origin.y) / self.tile_size),
        )
    }

    /// The level rect a cell covers.
    pub fn cell_rect(&self, x: i32, y: i32) -> Rect {
        let x: f32 = x.lossy_convert();
        let y: f32 = y.lossy_convert();
        Rect::new(
            self.origin.x + x * self.tile_size,
            self.origin.y + y * self.tile_size,
            self.tile_size,
            self.tile_size,
        )
    }

    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        let (Ok(ux), Ok(uy)) = (usize::try_from(x), usize::try_from(y)) else {
            return self.outside_solid;
        };
        if ux >= self.width || uy >= self.height {
            return self.outside_solid;
        }
        self.kind(self.get(ux, uy)).is_some_and(|kind| kind.solid)
    }

    /// Whether the box from `min` to `max` overlaps a solid cell. A box
    /// that ends exactly on a cell edge does not touch the next cell.
    pub fn box_hits_solid(&self, min: Point, max: Point) -> bool {
        let (x0, y0) = self.cell_at(min);
        let (x1, y1) = self.cell_at(max - Point::new(EDGE, EDGE));
        (x0..=x1.max(x0)).any(|x| (y0..=y1.max(y0)).any(|y| self.is_solid(x, y)))
    }

    /// Whether a box with this center and half size stands on a solid cell.
    pub fn stands_on_solid(&self, center: Point, half: Point) -> bool {
        let feet = center.y - half.y - EDGE * 2.0;
        self.box_hits_solid(
            Point::new(center.x - half.x, feet),
            Point::new(center.x + half.x, feet + EDGE),
        )
    }

    /// Moves a box with this center and half size by `delta`, first along
    /// x and then along y, and stops it at the first solid cell on each
    /// axis. A long move goes in hops shorter than half a cell, so a fast
    /// box never skips a thin wall.
    pub fn move_box(&self, center: Point, half: Point, delta: Point) -> BoxMove {
        let mut result = BoxMove {
            position: center,
            ..BoxMove::default()
        };

        let longest = delta.x.abs().max(delta.y.abs());
        let hops: f32 = (longest / (self.tile_size * 0.5)).ceil().max(1.0);
        let hop = delta / hops;

        let mut moving = Point::new(hop.x, hop.y);
        let hops: usize = hops.lossy_convert();
        for _ in 0..hops {
            if moving.x != 0.0 && self.move_axis(&mut result, half, moving.x, true) {
                moving.x = 0.0;
            }
            if moving.y != 0.0 && self.move_axis(&mut result, half, moving.y, false) {
                moving.y = 0.0;
            }
            if moving.x == 0.0 && moving.y == 0.0 {
                break;
            }
        }

        result
    }

    /// One hop along one axis, true when a solid cell stopped it.
    fn move_axis(&self, result: &mut BoxMove, half: Point, step: f32, along_x: bool) -> bool {
        let mut pos = result.position;
        if along_x {
            pos.x += step;
        } else {
            pos.y += step;
        }

        if !self.box_hits_solid(pos - half, pos + half) {
            result.position = pos;
            return false;
        }

        if along_x {
            let (min_cell, _) = self.cell_at(pos - half);
            let (max_cell, _) = self.cell_at(pos + half - Point::new(EDGE, 0.0));
            pos.x = if step > 0.0 {
                self.cell_rect(max_cell, 0).origin.x - half.x - EDGE
            } else {
                self.cell_rect(min_cell, 0).max_x() + half.x + EDGE
            };
            result.wall = true;
        } else {
            let (_, min_cell) = self.cell_at(pos - half);
            let (_, max_cell) = self.cell_at(pos + half - Point::new(0.0, EDGE));
            if step > 0.0 {
                pos.y = self.cell_rect(0, max_cell).origin.y - half.y - EDGE;
                result.ceiling = true;
            } else {
                pos.y = self.cell_rect(0, min_cell).max_y() + half.y + EDGE;
                result.floor = true;
            }
        }
        result.position = pos;
        true
    }

    /// Every drawn cell whose square touches `visible`, with its kind.
    pub(crate) fn visible_cells(&self, visible: Rect) -> impl Iterator<Item = (Rect, &TileKind)> {
        let (x0, y0) = self.cell_at(visible.origin);
        let (x1, y1) = self.cell_at(Point::new(visible.max_x(), visible.max_y()));
        let clamp_x = |x: i32| usize::try_from(x.max(0)).unwrap_or(0).min(self.width);
        let clamp_y = |y: i32| usize::try_from(y.max(0)).unwrap_or(0).min(self.height);
        let (x0, x1) = (clamp_x(x0), clamp_x(x1.saturating_add(1)));
        let (y0, y1) = (clamp_y(y0), clamp_y(y1.saturating_add(1)));

        (y0..y1).flat_map(move |y| {
            (x0..x1).filter_map(move |x| {
                let kind = self.kind(self.tiles[y * self.width + x])?;
                kind.image.is_ok().then(|| {
                    let cell = self.cell_rect(cell_index(x), cell_index(y));
                    (cell, kind)
                })
            })
        })
    }
}

impl LevelBase {
    /// Adds a tile layer, drawn behind every sprite and behind the layers
    /// added before it.
    pub fn add_tile_map(&mut self, mut map: TileMap) -> Weak<TileMap> {
        let layers: f32 = self.tile_maps.len().lossy_convert();
        map.z_position =
            LevelManager::default_z_position() + LevelManager::z_position_offset() * (5.0 - layers * 0.1);
        let map = Own::new(map);
        let weak = map.weak();
        self.tile_maps.push(map);
        weak
    }

    pub fn remove_tile_map(&mut self, map: Weak<TileMap>) {
        self.tile_maps.retain(|own| own.raw() != map.raw());
    }

    pub fn tile_maps(&self) -> &[Own<TileMap>] {
        &self.tile_maps
    }
}

fn cell_coord(value: f32) -> i32 {
    value.floor().lossy_convert()
}

fn cell_index(index: usize) -> i32 {
    i32::try_from(index).expect("a tile map is narrower than i32")
}

#[cfg(test)]
mod test {
    use super::*;

    const HALF: Point = Point::new(0.4, 0.4);

    /// A 10 by 10 grid with a floor row, a one tile wall at x 6 and a
    /// decor tile at x 3 that stops nothing.
    fn room() -> (TileMap, TileId) {
        let mut map = TileMap::new(10, 10);
        let stone = map.add_kind(TileKind {
            image: Weak::default(),
            solid: true,
        });
        let decor = map.add_kind(TileKind {
            image: Weak::default(),
            solid: false,
        });
        map.fill(0, 0, 10, 1, stone);
        map.set(6, 1, stone);
        map.set(3, 1, decor);
        (map, stone)
    }

    #[test]
    fn outside_follows_the_flag() {
        let (mut map, _) = room();
        assert!(map.is_solid(-1, 5));
        assert!(map.is_solid(3, 10));
        assert!(!map.is_solid(3, 1), "decor is not solid");
        map.outside_solid = false;
        assert!(!map.is_solid(-1, 5));
    }

    #[test]
    fn a_box_on_a_tile_edge_does_not_touch_the_next_tile() {
        let (map, _) = room();
        assert!(!map.box_hits_solid(Point::new(0.0, 1.0), Point::new(3.0, 2.0)));
        assert!(map.box_hits_solid(Point::new(0.5, 0.9), Point::new(1.5, 2.0)));
    }

    #[test]
    fn a_falling_box_lands_on_the_floor() {
        let (map, _) = room();
        let mut center = Point::new(2.5, 5.0);
        let mut landed = false;
        for _ in 0..100 {
            let moved = map.move_box(center, HALF, Point::new(0.0, -0.2));
            center = moved.position;
            landed |= moved.floor;
        }
        assert!(landed);
        assert!((center.y - HALF.y - 1.0).abs() < 0.01, "{center:?}");
        assert!(map.stands_on_solid(center, HALF));
    }

    #[test]
    fn a_walking_box_stops_at_a_wall() {
        let (map, _) = room();
        let moved = map.move_box(Point::new(2.5, 1.41), HALF, Point::new(5.0, 0.0));
        assert!(moved.wall);
        assert!(moved.position.x + HALF.x <= 6.0, "{moved:?}");
        assert!(moved.position.x + HALF.x > 5.99, "{moved:?}");
    }

    #[test]
    fn a_fast_box_does_not_skip_a_thin_wall() {
        let (map, _) = room();
        let moved = map.move_box(Point::new(1.5, 1.41), HALF, Point::new(40.0, 0.0));
        assert!(moved.wall);
        assert!(moved.position.x < 6.0, "{moved:?}");
    }

    #[test]
    fn a_ceiling_stops_a_jump() {
        let (map, stone) = room();
        let mut map = map;
        map.set(2, 4, stone);
        let moved = map.move_box(Point::new(2.5, 1.41), HALF, Point::new(0.0, 5.0));
        assert!(moved.ceiling);
        assert!((moved.position.y + HALF.y - 4.0).abs() < 0.01, "{moved:?}");
    }

    #[test]
    fn origin_and_tile_size_move_the_grid() {
        let (mut map, _) = room();
        map.origin = Point::new(-10.0, -20.0);
        map.tile_size = 5.0;
        assert_eq!(map.cell_at(Point::new(-9.0, -19.0)), (0, 0));
        assert_eq!(map.cell_at(Point::new(21.0, -14.0)), (6, 1));
        assert!(map.box_hits_solid(Point::new(20.5, -14.5), Point::new(21.0, -14.0)));
        let moved = map.move_box(Point::new(0.0, 0.0), Point::new(1.0, 1.0), Point::new(0.0, -30.0));
        assert!(moved.floor);
        assert!((moved.position.y - 1.0 - -15.0).abs() < 0.01, "{moved:?}");
    }
}
