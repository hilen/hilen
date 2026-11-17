//! The coverage of text spread over the pixels around it, for an outline and
//! a soft shadow. Both run in 2 passes, along the rows and then down the
//! columns, so the cost grows with the reach and not with its square.

/// A gaussian blur of a coverage map with a radius in pixels. The sigma is
/// half the radius, like the blur of a CSS text shadow, cut off at the
/// radius. The map is `width` wide, row after row.
pub fn blur_coverage(coverage: &[f32], width: usize, radius: f32) -> Vec<f32> {
    if width == 0 || radius <= 0.0 {
        return coverage.to_vec();
    }
    let height = coverage.len() / width;
    let sigma = (radius * 0.5).max(0.001);
    let weights: Vec<f32> = (0..=reach_steps(radius))
        .map(|step| {
            let distance = f32::from(step);
            (-distance * distance / (2.0 * sigma * sigma)).exp()
        })
        .collect();
    let total: f32 = weights[0] + 2.0 * weights[1..].iter().sum::<f32>();

    // A sample past the edge of the map is no coverage, it adds nothing.
    let along = |source: &[f32], out: &mut [f32], count: usize, stride: usize, lines: usize, step: usize| {
        for line in 0..lines {
            for at in 0..count {
                let mut sum = source[line * step + at * stride] * weights[0];
                for (offset, weight) in weights.iter().enumerate().skip(1) {
                    if at >= offset {
                        sum += source[line * step + (at - offset) * stride] * weight;
                    }
                    if at + offset < count {
                        sum += source[line * step + (at + offset) * stride] * weight;
                    }
                }
                out[line * step + at * stride] = sum / total;
            }
        }
    };

    let mut rows = vec![0.0; coverage.len()];
    along(coverage, &mut rows, width, 1, height, width);
    let mut out = vec![0.0; coverage.len()];
    along(&rows, &mut out, height, width, width, 1);
    out
}

/// The least coverage that shows in a frame of 8 bits a channel. A glyph
/// raster leaves float noise far below it all over the box of the glyph,
/// and a widening that took the noise for an edge filled the whole box.
const VISIBLE: f32 = 1.0 / 255.0;

/// Every edge of a coverage map moved out by `reach` pixels, round at the
/// corners. A pixel takes the nearest covered pixel within the reach: its
/// coverage, less the distance past the reach. So a straight edge keeps its
/// own soft pixel and lands exactly `reach` further out.
pub fn widen_coverage(coverage: &[f32], width: usize, reach: f32) -> Vec<f32> {
    if width == 0 || reach <= 0.0 {
        return coverage.to_vec();
    }
    let height = coverage.len() / width;
    let steps = usize::from(reach_steps(reach));

    // Along the rows: for every pixel the covered pixel of its row that
    // reaches it best, as the columns between them and its coverage.
    // `None` when the row has no coverage within the steps.
    let mut rows: Vec<Option<(f32, f32)>> = vec![None; coverage.len()];
    for y in 0..height {
        let row = &coverage[y * width..(y + 1) * width];
        for x in 0..width {
            let from = x.saturating_sub(steps);
            let to = (x + steps).min(width - 1);
            let mut best: Option<(f32, f32)> = None;
            for (other, covered) in row.iter().enumerate().take(to + 1).skip(from) {
                if *covered < VISIBLE {
                    continue;
                }
                let apart = pixels_apart(other, x);
                if best.is_none_or(|(columns, had)| apart - covered < columns - had) {
                    best = Some((apart, *covered));
                }
            }
            rows[y * width + x] = best;
        }
    }

    // Down the columns: the best of those over the rows within the steps,
    // now by the real distance.
    let mut out = vec![0.0; coverage.len()];
    for y in 0..height {
        let from = y.saturating_sub(steps);
        let to = (y + steps).min(height - 1);
        for x in 0..width {
            let mut most: f32 = 0.0;
            for other in from..=to {
                let Some((columns, covered)) = rows[other * width + x] else {
                    continue;
                };
                let lines = pixels_apart(other, y);
                let distance = (columns * columns + lines * lines).sqrt();
                most = most.max((reach + covered - distance).clamp(0.0, 1.0));
            }
            out[y * width + x] = most;
        }
    }
    out
}

/// Whole pixels a spread of `reach` looks at to each side, the reach
/// rounded up.
fn reach_steps(reach: f32) -> u16 {
    let mut steps = 0;
    while f32::from(steps) < reach && steps < u16::MAX {
        steps += 1;
    }
    steps
}

/// Pixels between 2 places of a row or a column. They are at most the
/// steps of a reach apart, which fit `u16`.
fn pixels_apart(a: usize, b: usize) -> f32 {
    f32::from(u16::try_from(a.abs_diff(b)).unwrap_or(u16::MAX))
}

#[cfg(test)]
mod tests {
    use super::{blur_coverage, widen_coverage};

    const SIDE: usize = 81;
    const MIDDLE: usize = 40;

    /// One fully covered pixel in the middle of an empty map.
    fn dot() -> Vec<f32> {
        let mut map = vec![0.0; SIDE * SIDE];
        map[MIDDLE * SIDE + MIDDLE] = 1.0;
        map
    }

    fn at(map: &[f32], x: usize, y: usize) -> f32 {
        map[y * SIDE + x]
    }

    /// A reach of 0 changes nothing, the plain text is its own outline.
    #[test]
    fn no_reach_is_the_map_itself() {
        let mut map = dot();
        map[3] = 0.4;
        assert_eq!(widen_coverage(&map, SIDE, 0.0), map);
        assert_eq!(blur_coverage(&map, SIDE, 0.0), map);
    }

    /// A dot widened by 30 pixels, far past the old limit of 12, is a disc:
    /// full inside the reach, empty past it, the same along an axis and
    /// along the diagonal.
    #[test]
    fn a_dot_widens_into_a_disc() {
        let wide = widen_coverage(&dot(), SIDE, 30.0);
        assert!(
            (at(&wide, MIDDLE + 30, MIDDLE) - 1.0).abs() < 1e-4,
            "the rim on the axis"
        );
        assert!(
            at(&wide, MIDDLE + 31, MIDDLE).abs() < 1e-4,
            "past the rim on the axis"
        );
        assert!(
            (at(&wide, MIDDLE, MIDDLE - 30) - 1.0).abs() < 1e-4,
            "the rim upward"
        );
        // 21 columns and 21 lines are 29.7 pixels away, 22 and 22 are 31.1.
        assert!(
            (at(&wide, MIDDLE + 21, MIDDLE + 21) - 1.0).abs() < 1e-4,
            "inside on the diagonal"
        );
        assert!(
            at(&wide, MIDDLE + 22, MIDDLE + 22).abs() < 1e-4,
            "a square corner, not a disc"
        );
    }

    /// Coverage too faint to see is no edge, the raster of a glyph has such
    /// noise all over its box.
    #[test]
    fn noise_is_not_widened() {
        let mut map = vec![1e-6; SIDE * SIDE];
        map[MIDDLE * SIDE + MIDDLE] = 1.0;
        let wide = widen_coverage(&map, SIDE, 4.0);
        assert!(
            at(&wide, MIDDLE + 6, MIDDLE).abs() < 1e-4,
            "the noise grew into an edge"
        );
        assert!((at(&wide, MIDDLE + 4, MIDDLE) - 1.0).abs() < 1e-4);
    }

    /// A straight edge with a half covered pixel moves out by the reach and
    /// keeps that half covered pixel.
    #[test]
    fn an_edge_keeps_its_soft_pixel() {
        let mut map = vec![0.0; SIDE * SIDE];
        for y in 0..SIDE {
            for x in 0..10 {
                map[y * SIDE + x] = 1.0;
            }
            map[y * SIDE + 10] = 0.5;
        }
        let wide = widen_coverage(&map, SIDE, 4.0);
        assert!((at(&wide, 13, MIDDLE) - 1.0).abs() < 1e-4);
        assert!((at(&wide, 14, MIDDLE) - 0.5).abs() < 1e-4);
        assert!(at(&wide, 15, MIDDLE).abs() < 1e-4);
    }

    /// The blur of a dot is the product of the same curve along both axes,
    /// it keeps all of the coverage, and it reaches as far as its radius.
    #[test]
    fn a_dot_blurs_into_a_gaussian() {
        let radius = 30.0;
        let soft = blur_coverage(&dot(), SIDE, radius);
        let total: f32 = soft.iter().sum();
        assert!((total - 1.0).abs() < 1e-3, "the blur lost coverage, {total}");

        let peak = at(&soft, MIDDLE, MIDDLE);
        let sigma = radius * 0.5;
        for step in [1_u16, 7, 15, 30] {
            let distance = f32::from(step);
            let step = usize::from(step);
            let curve = (-distance * distance / (2.0 * sigma * sigma)).exp();
            let on_axis = at(&soft, MIDDLE + step, MIDDLE) / peak;
            assert!(
                (on_axis - curve).abs() < 1e-4,
                "{step} pixels out: {on_axis} for {curve}"
            );
            let diagonal = at(&soft, MIDDLE + step, MIDDLE + step) / peak;
            assert!(
                (diagonal - curve * curve).abs() < 1e-4,
                "{step} out on the diagonal"
            );
        }
        assert!(
            at(&soft, MIDDLE + 31, MIDDLE).abs() < f32::EPSILON,
            "past the radius"
        );
    }
}
