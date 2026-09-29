use std::ops::BitOr;

use crate::{deps::refs::Weak, window::image::Image};

/// Which of the 4 neighbors of a cell join it, 16 shapes in all. A tile
/// framed like Terraria frames its blocks draws an edge on every side
/// that does not join.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TileSides(u8);

impl TileSides {
    pub const NONE: Self = Self(0);
    pub const UP: Self = Self(1);
    pub const RIGHT: Self = Self(2);
    pub const DOWN: Self = Self(4);
    pub const LEFT: Self = Self(8);
    pub const ALL: Self = Self(15);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn up(self) -> bool {
        self.contains(Self::UP)
    }

    pub fn right(self) -> bool {
        self.contains(Self::RIGHT)
    }

    pub fn down(self) -> bool {
        self.contains(Self::DOWN)
    }

    pub fn left(self) -> bool {
        self.contains(Self::LEFT)
    }

    /// The shape as a number from 0 to 15, bit 1 up, 2 right, 4 down and
    /// 8 left, the way art files for the shapes are often named.
    pub fn bits(self) -> u8 {
        self.0
    }

    /// Every one of the 16 shapes.
    pub fn every() -> impl Iterator<Item = Self> {
        (0..16).map(Self)
    }

    fn index(self) -> usize {
        usize::from(self.bits())
    }
}

impl BitOr for TileSides {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// The images a framed tile kind picks from, some variations for each of
/// the 16 shapes. A cell picks its shape from its neighbors and its
/// variation from its place in the grid, so it looks the same every frame
/// and changes only when a neighbor does.
#[derive(Debug, Clone, Default)]
pub struct TileFrames {
    shapes: Vec<Vec<Weak<Image>>>,
}

impl TileFrames {
    /// `variations` gives the images of one shape. A shape with none
    /// draws the kind's own image.
    pub fn new(mut variations: impl FnMut(TileSides) -> Vec<Weak<Image>>) -> Self {
        Self {
            shapes: TileSides::every().map(&mut variations).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.iter().all(Vec::is_empty)
    }

    pub(crate) fn pick(&self, sides: TileSides, x: usize, y: usize) -> Option<Weak<Image>> {
        let variations = self.shapes.get(sides.index())?;
        if variations.is_empty() {
            return None;
        }
        let count = u64::try_from(variations.len()).expect("a few variations");
        let pick = usize::try_from(cell_hash(x, y) % count).expect("below the count");
        Some(variations[pick])
    }
}

/// A well mixed number for a cell, the same every time, the splitmix64
/// finalizer over both coordinates.
fn cell_hash(x: usize, y: usize) -> u64 {
    let (x, y) = (
        u64::try_from(x).expect("a grid fits u64"),
        u64::try_from(y).expect("a grid fits u64"),
    );
    let mut z = (x ^ y.rotate_left(32)).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::level::{TileCollision, TileKind, TileMap};

    /// A 6 by 4 grid, dirt on rows 0 and 1, a stone block at x 2 on
    /// row 1, a lone dirt block at x 4 on row 3.
    fn world(outside_solid: bool) -> (TileMap, [crate::level::TileId; 2]) {
        let mut map = TileMap::new(6, 4);
        map.outside_solid = outside_solid;
        let dirt = map.add_kind(TileKind::new(Weak::default(), TileCollision::Solid));
        let stone = map.add_kind(TileKind::new(Weak::default(), TileCollision::Solid));
        map.fill(0, 0, 6, 2, dirt);
        map.set(2, 1, stone);
        map.set(4, 3, dirt);
        (map, [dirt, stone])
    }

    #[test]
    fn a_cell_joins_its_own_kind_and_not_empty_cells() {
        let (map, _) = world(false);
        assert_eq!(map.sides(0, 1), TileSides::RIGHT | TileSides::DOWN);
        assert_eq!(map.sides(4, 3), TileSides::NONE);
        assert_eq!(
            map.sides(4, 1),
            TileSides::LEFT | TileSides::RIGHT | TileSides::DOWN
        );
    }

    #[test]
    fn different_kinds_join_only_when_asked() {
        let (mut map, [dirt, stone]) = world(false);
        assert_eq!(map.sides(2, 1), TileSides::NONE);
        assert!(!map.sides(1, 1).right());
        map.join(stone, dirt);
        assert_eq!(
            map.sides(2, 1),
            TileSides::LEFT | TileSides::RIGHT | TileSides::DOWN
        );
        assert!(map.sides(1, 1).right());
    }

    #[test]
    fn the_border_joins_when_the_outside_is_solid() {
        let (map, _) = world(true);
        assert_eq!(map.sides(0, 0), TileSides::ALL);
        assert_eq!(map.sides(4, 3), TileSides::UP);
    }

    #[test]
    fn digging_a_cell_opens_its_neighbors() {
        let (mut map, _) = world(false);
        assert!(map.sides(4, 0).up());
        map.set(4, 1, crate::level::TileId::EMPTY);
        assert!(!map.sides(4, 0).up());
        assert!(!map.sides(3, 1).right());
        assert!(!map.sides(5, 1).left());
    }

    #[test]
    fn a_cell_keeps_its_variation_and_a_row_uses_them_all() {
        let picks: Vec<u64> = (0..40).map(|x| cell_hash(x, 7) % 3).collect();
        assert_eq!(picks, (0..40).map(|x| cell_hash(x, 7) % 3).collect::<Vec<_>>());
        assert!((0..3).all(|v| picks.contains(&v)), "{picks:?}");
    }

    #[test]
    fn every_shape_is_one_of_16() {
        let shapes: Vec<TileSides> = TileSides::every().collect();
        assert_eq!(shapes.len(), 16);
        assert!(
            TileSides::ALL.up() && TileSides::ALL.right() && TileSides::ALL.down() && TileSides::ALL.left()
        );
        assert!(!TileSides::NONE.up());
    }
}
