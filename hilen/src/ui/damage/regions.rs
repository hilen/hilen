//! The parts of the window a frame has to draw again, as rects in pixels.

use crate::gm::flat::{Rect, Size};

/// Rects closer than this many pixels become one. The cells of a small
/// mark are then 1 region and not 1 pass over the tree each.
const NEAR: u32 = 24;

/// More regions than this cost more passes over the tree than they save,
/// so they become 1 rect around all of them.
const MAX_REGIONS: usize = 12;

/// A frame that would draw more than 1 part in this many of the window
/// again is drawn whole, the bookkeeping of a partial frame no longer pays.
const WHOLE_PARTS: u64 = 2;

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Regions {
    rects: Vec<Rect<u32>>,
}

impl Regions {
    pub(crate) fn add(&mut self, rect: Rect<u32>) {
        if rect.width() == 0 || rect.height() == 0 {
            return;
        }

        // A rect that joins 2 regions makes them 1, so every join is
        // checked against the rest again.
        let mut rect = rect;
        while let Some(index) = self.rects.iter().position(|other| near(other, &rect)) {
            rect = union(&self.rects.swap_remove(index), &rect);
        }
        self.rects.push(rect);

        if self.rects.len() > MAX_REGIONS {
            let all = self.rects.iter().skip(1).fold(self.rects[0], |all, rect| union(&all, rect));
            self.rects.clear();
            self.rects.push(all);
        }
    }

    pub(crate) fn rects(&self) -> &[Rect<u32>] {
        &self.rects
    }

    /// Whether the regions are so much of a window of `size` that the
    /// whole frame is drawn.
    pub(crate) fn most_of(&self, size: Size<u32>) -> bool {
        let window = u64::from(size.width) * u64::from(size.height);
        let covered: u64 = self
            .rects
            .iter()
            .map(|rect| u64::from(rect.width()) * u64::from(rect.height()))
            .sum();
        covered * WHOLE_PARTS > window
    }
}

fn near(a: &Rect<u32>, b: &Rect<u32>) -> bool {
    let a_right = a.x() + a.width() + NEAR;
    let a_bottom = a.y() + a.height() + NEAR;
    let b_right = b.x() + b.width() + NEAR;
    let b_bottom = b.y() + b.height() + NEAR;

    a.x() < b_right && b.x() < a_right && a.y() < b_bottom && b.y() < a_bottom
}

fn union(a: &Rect<u32>, b: &Rect<u32>) -> Rect<u32> {
    let x = a.x().min(b.x());
    let y = a.y().min(b.y());
    let right = (a.x() + a.width()).max(b.x() + b.width());
    let bottom = (a.y() + a.height()).max(b.y() + b.height());
    Rect::new(x, y, right - x, bottom - y)
}

#[cfg(test)]
mod tests {
    use super::{NEAR, Regions};
    use crate::gm::flat::{Rect, Size};

    #[test]
    fn an_empty_rect_adds_nothing() {
        let mut regions = Regions::default();
        regions.add(Rect::new(10, 10, 0, 5));
        assert_eq!(regions.rects(), []);
    }

    #[test]
    fn rects_far_apart_stay_apart() {
        let mut regions = Regions::default();
        regions.add(Rect::new(0, 0, 10, 10));
        regions.add(Rect::new(10 + NEAR + 1, 0, 10, 10));
        assert_eq!(regions.rects().len(), 2);
    }

    #[test]
    fn rects_close_by_become_one() {
        let mut regions = Regions::default();
        regions.add(Rect::new(0, 0, 10, 10));
        regions.add(Rect::new(12, 0, 10, 10));
        assert_eq!(regions.rects(), &[Rect::new(0, 0, 22, 10)]);
    }

    #[test]
    fn a_rect_between_two_regions_joins_them() {
        let mut regions = Regions::default();
        regions.add(Rect::new(0, 0, 10, 10));
        regions.add(Rect::new(80, 0, 10, 10));
        assert_eq!(regions.rects().len(), 2);
        regions.add(Rect::new(30, 0, 30, 10));
        assert_eq!(regions.rects(), &[Rect::new(0, 0, 90, 10)]);
    }

    #[test]
    fn too_many_regions_become_one_rect_around_them() {
        let mut regions = Regions::default();
        for column in 0..13 {
            regions.add(Rect::new(column * 100, 0, 10, 10));
        }
        assert_eq!(regions.rects(), &[Rect::new(0, 0, 1210, 10)]);
    }

    #[test]
    fn more_than_half_of_the_window_is_most_of_it() {
        let mut regions = Regions::default();
        regions.add(Rect::new(0, 0, 100, 50));
        assert!(!regions.most_of(Size::new(100, 100)));
        regions.add(Rect::new(0, 50, 100, 1));
        assert!(regions.most_of(Size::new(100, 100)));
    }
}
