use image::RgbImage;

/// Tuning of the two pass comparison. The mean pass forgives the subpixel
/// text anti aliasing that differs between any two renderers, the pixel
/// pass catches flat fills that moved or changed color.
pub struct Options {
    /// Cell size of the grid both passes run on, in pixels.
    pub cell:            u32,
    /// Max per channel delta of a cell mean before the cell is different.
    pub cell_tolerance:  u8,
    /// Max per channel delta of one pixel before that pixel is different.
    pub pixel_tolerance: u8,
    /// Percent of differing pixels that marks a cell even when means agree.
    pub pixel_percent:   u32,
    /// Regions to skip, known state differences like live data.
    pub ignore:          Vec<Rect>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            cell:            8,
            cell_tolerance:  3,
            pixel_tolerance: 12,
            pixel_percent:   35,
            ignore:          Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    fn intersects(self, other: Rect) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }
}

/// One clustered difference, cells connected through their sides.
pub struct Region {
    /// Bounding box in pixels of the first image.
    pub bounds: Rect,
    /// Differing cells inside the box.
    pub cells:  u32,
    /// Mean color of the box in each image.
    pub mean_a: [u8; 3],
    pub mean_b: [u8; 3],
}

/// Compare two same size images and cluster the differences.
/// Regions come back largest first.
///
/// # Panics
///
/// Panics when the images differ in size, the capture procedure
/// guarantees equal sizes.
pub fn run(a: &RgbImage, b: &RgbImage, options: &Options) -> Vec<Region> {
    assert_eq!(a.dimensions(), b.dimensions(), "images must be the same size");

    let (width, height) = a.dimensions();
    let cell = options.cell.max(1);
    let cells_x = width.div_ceil(cell);
    let cells_y = height.div_ceil(cell);

    let mut grid = Grid {
        marked: vec![false; (cells_x * cells_y) as usize],
        cells_x,
        cells_y,
        cell,
        width,
        height,
    };
    for cy in 0..cells_y {
        for cx in 0..cells_x {
            let bounds = cell_bounds(cx, cy, cell, width, height);
            if options.ignore.iter().any(|ignored| ignored.intersects(bounds)) {
                continue;
            }
            grid.marked[(cy * cells_x + cx) as usize] = cell_differs(a, b, bounds, options);
        }
    }

    cluster(&grid, a, b)
}

/// The marked cell grid the clustering walks.
struct Grid {
    marked:  Vec<bool>,
    cells_x: u32,
    cells_y: u32,
    cell:    u32,
    width:   u32,
    height:  u32,
}

fn cell_bounds(cx: u32, cy: u32, cell: u32, width: u32, height: u32) -> Rect {
    let x = cx * cell;
    let y = cy * cell;
    Rect {
        x,
        y,
        w: cell.min(width - x),
        h: cell.min(height - y),
    }
}

fn cell_differs(a: &RgbImage, b: &RgbImage, bounds: Rect, options: &Options) -> bool {
    let mut sum_a = [0_u64; 3];
    let mut sum_b = [0_u64; 3];
    let mut differing = 0_u64;
    let total = u64::from(bounds.w) * u64::from(bounds.h);

    for y in bounds.y..bounds.y + bounds.h {
        for x in bounds.x..bounds.x + bounds.w {
            let pa = a.get_pixel(x, y).0;
            let pb = b.get_pixel(x, y).0;
            let mut delta = 0_u8;
            for channel in 0..3 {
                sum_a[channel] += u64::from(pa[channel]);
                sum_b[channel] += u64::from(pb[channel]);
                delta = delta.max(pa[channel].abs_diff(pb[channel]));
            }
            if delta > options.pixel_tolerance {
                differing += 1;
            }
        }
    }

    let mean_delta = (0..3)
        .map(|channel| (sum_a[channel] / total).abs_diff(sum_b[channel] / total))
        .max()
        .unwrap_or(0);
    mean_delta > u64::from(options.cell_tolerance)
        || differing * 100 > total * u64::from(options.pixel_percent)
}

fn cluster(grid: &Grid, a: &RgbImage, b: &RgbImage) -> Vec<Region> {
    let Grid {
        marked,
        cells_x,
        cells_y,
        cell,
        width,
        height,
    } = grid;
    let (cells_x, cells_y, cell, width, height) = (*cells_x, *cells_y, *cell, *width, *height);
    let mut seen = vec![false; marked.len()];
    let mut regions = Vec::new();

    for start in 0..marked.len() {
        if !marked[start] || seen[start] {
            continue;
        }
        let mut cells = 0_u32;
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0_u32, 0_u32);
        let mut queue = vec![start];
        seen[start] = true;
        while let Some(index) = queue.pop() {
            cells += 1;
            let index_u32 = u32::try_from(index).expect("cell index fits u32");
            let cx = index_u32 % cells_x;
            let cy = index_u32 / cells_x;
            min_x = min_x.min(cx);
            min_y = min_y.min(cy);
            max_x = max_x.max(cx);
            max_y = max_y.max(cy);
            for (dx, dy) in [(-1_i64, 0_i64), (1, 0), (0, -1), (0, 1)] {
                let nx = i64::from(cx) + dx;
                let ny = i64::from(cy) + dy;
                if nx < 0 || ny < 0 || nx >= i64::from(cells_x) || ny >= i64::from(cells_y) {
                    continue;
                }
                let neighbor = usize::try_from(ny * i64::from(cells_x) + nx).expect("cell index fits usize");
                if marked[neighbor] && !seen[neighbor] {
                    seen[neighbor] = true;
                    queue.push(neighbor);
                }
            }
        }
        let bounds = Rect {
            x: min_x * cell,
            y: min_y * cell,
            w: ((max_x + 1) * cell).min(width) - min_x * cell,
            h: ((max_y + 1) * cell).min(height) - min_y * cell,
        };
        regions.push(Region {
            bounds,
            cells,
            mean_a: mean_color(a, bounds),
            mean_b: mean_color(b, bounds),
        });
    }

    regions.sort_by_key(|region| core::cmp::Reverse(region.cells));
    regions
}

fn mean_color(image: &RgbImage, bounds: Rect) -> [u8; 3] {
    let mut sum = [0_u64; 3];
    for y in bounds.y..bounds.y + bounds.h {
        for x in bounds.x..bounds.x + bounds.w {
            let pixel = image.get_pixel(x, y).0;
            for channel in 0..3 {
                sum[channel] += u64::from(pixel[channel]);
            }
        }
    }
    let total = u64::from(bounds.w) * u64::from(bounds.h);
    [0, 1, 2].map(|channel| u8::try_from(sum[channel] / total).expect("mean fits u8"))
}

#[cfg(test)]
mod tests {
    use image::Rgb;

    use super::*;

    fn flat(width: u32, height: u32, color: [u8; 3]) -> RgbImage {
        RgbImage::from_pixel(width, height, Rgb(color))
    }

    fn fill(image: &mut RgbImage, rect: Rect, color: [u8; 3]) {
        for y in rect.y..rect.y + rect.h {
            for x in rect.x..rect.x + rect.w {
                image.put_pixel(x, y, Rgb(color));
            }
        }
    }

    #[test]
    fn identical_images_have_no_regions() {
        let a = flat(64, 64, [237, 237, 237]);
        assert!(run(&a, &a.clone(), &Options::default()).is_empty());
    }

    #[test]
    fn noise_below_tolerance_is_forgiven() {
        let a = flat(64, 64, [237, 237, 237]);
        let mut b = a.clone();
        // A couple of anti aliasing style single pixel differences.
        b.put_pixel(10, 10, Rgb([180, 180, 180]));
        b.put_pixel(40, 33, Rgb([200, 200, 200]));
        assert!(run(&a, &b, &Options::default()).is_empty());
    }

    #[test]
    fn a_moved_block_flags_the_vacated_and_the_new_strip() {
        let a = flat(64, 64, [237, 237, 237]);
        let mut b = a.clone();
        let mut moved = a.clone();
        fill(
            &mut b,
            Rect {
                x: 8,
                y: 8,
                w: 16,
                h: 16,
            },
            [24, 160, 88],
        );
        fill(
            &mut moved,
            Rect {
                x: 16,
                y: 8,
                w: 16,
                h: 16,
            },
            [24, 160, 88],
        );
        // The overlap 16..24 is identical in both images, so the two
        // differing strips cluster separately.
        let regions = run(&b, &moved, &Options::default());
        assert_eq!(regions.len(), 2);
        let covers = |x: u32| {
            regions
                .iter()
                .any(|region| region.bounds.x <= x && x < region.bounds.x + region.bounds.w)
        };
        assert!(covers(8) && covers(31));
    }

    #[test]
    fn a_color_shift_is_reported_with_both_means() {
        let a = flat(64, 64, [245, 158, 11]);
        let b = flat(64, 64, [217, 119, 6]);
        let regions = run(&a, &b, &Options::default());
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].mean_a, [245, 158, 11]);
        assert_eq!(regions[0].mean_b, [217, 119, 6]);
    }

    #[test]
    fn ignored_rects_are_skipped() {
        let a = flat(64, 64, [237, 237, 237]);
        let mut b = a.clone();
        fill(
            &mut b,
            Rect {
                x: 8,
                y: 8,
                w: 16,
                h: 16,
            },
            [24, 160, 88],
        );
        let options = Options {
            ignore: vec![Rect {
                x: 0,
                y: 0,
                w: 32,
                h: 32,
            }],
            ..Options::default()
        };
        assert!(run(&a, &b, &options).is_empty());
    }
}
