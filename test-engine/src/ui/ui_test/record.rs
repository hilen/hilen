use std::{
    cmp::Ordering::{Equal, Greater, Less},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    AppRunner,
    gm::color::U8Color,
    ui_test::{
        TEST_NAME,
        human::{human_mode, show_probes},
    },
    window::Screenshot,
};

static RECORD_COLORS: AtomicBool = AtomicBool::new(false);
static CHECK_INDEX: AtomicUsize = AtomicUsize::new(0);
static LAST_TEST: Mutex<String> = Mutex::new(String::new());
static PROBE_COUNT_OVERRIDE: AtomicUsize = AtomicUsize::new(0);
static CANVAS: Mutex<Option<(u32, u32)>> = Mutex::new(None);

const DEFAULT_PROBE_COUNT: usize = 32;
const GRID_STEP: u32 = 4;

/// A probe candidate must have a near uniform neighborhood along at least
/// one axis, at this radius. Filters out antialiased corners, which are the
/// pixels most sensitive to a sub pixel layout shift.
///
/// One axis, not all of them. Demanding every direction is uniform asks for
/// a glyph stem at least 3 pixels wide, which only a blocky font has at a
/// normal text size. Every hairline or striped font produced zero candidates
/// and its labels recorded nothing but the background around the text, so
/// the glyphs went unchecked and the test still passed when they vanished.
/// Along a stem the pixels are identical, however thin it is, which is what
/// this asks for instead.
const STABLE_RADIUS: u32 = 1;
const STABLE_TOLERANCE: i16 = 10;

/// Candidates closer than this in color count as one region.
const CLUSTER_TOLERANCE: i16 = 60;

/// A candidate whose color runs out within this distance in every
/// direction sits in a small enclosed feature, like the hole of an O
/// or the dot of an i. Such candidates cluster separately and get
/// probes first, plain region spread never visits them.
const SMALL_FEATURE_EXTENT: u32 = 12;

/// Print ready to paste `check_colors` blocks instead of asserting them.
/// Enabled by `--record-colors` in ui-test.
pub fn enable_color_recording() {
    RECORD_COLORS.store(true, Ordering::Relaxed);
}

pub fn recording_colors() -> bool {
    RECORD_COLORS.load(Ordering::Relaxed)
}

/// How many probes `--record-colors` picks per check for this test.
/// Call at the start of `perform_test`, the agreed density then lives
/// in the test source. Inert outside record runs. Resets to the
/// default when the next test starts.
pub fn set_record_probe_count(count: usize) {
    PROBE_COUNT_OVERRIDE.store(count, Ordering::Relaxed);
}

pub(crate) fn reset_record_probe_count() {
    PROBE_COUNT_OVERRIDE.store(0, Ordering::Relaxed);
}

/// The window around the canvas is not part of the test and does not even
/// exist on a device screen, so no probe may land there.
pub(crate) fn set_record_canvas(width: u32, height: u32) {
    *CANVAS.lock() = Some((width, height));
}

fn probe_count() -> usize {
    let count = PROBE_COUNT_OVERRIDE.load(Ordering::Relaxed);
    if count == 0 { DEFAULT_PROBE_COUNT } else { count }
}

/// Which `check_colors` call this is within the running test. Names
/// printed blocks and marker hold titles.
pub(crate) fn next_check_index(test_name: &str) -> usize {
    let mut last = LAST_TEST.lock();

    if *last != test_name {
        test_name.clone_into(&mut last);
        CHECK_INDEX.store(0, Ordering::Relaxed);
    }

    CHECK_INDEX.fetch_add(1, Ordering::Relaxed) + 1
}

pub(crate) fn print_recorded_colors() -> Result<()> {
    let screenshot = AppRunner::take_screenshot()?;
    let probes = pick_probes(&screenshot);

    let test_name = TEST_NAME.lock().clone();
    let index = next_check_index(&test_name);

    println!();
    println!("{test_name} check {index}, {} probes:", probes.len());
    println!();
    println!("        check_colors(");
    println!("            r\"");

    for ((x, y), color) in &probes {
        println!("            {:>4} {:>4} - {}", x, y, color.as_hex());
    }

    println!("            \",");
    println!("        )?;");
    println!();

    if human_mode() {
        show_probes(&probes, &test_name, index);
    }

    Ok(())
}

type Probe = ((u32, u32), U8Color);

/// Picks stable pixels covering every distinct color region. Candidates
/// are clustered by color and each cluster gets a share of the probes,
/// spread spatially, so text bodies get pinned alongside backgrounds.
/// Small enclosed features cluster separately and go first.
fn pick_probes(shot: &Screenshot) -> Vec<Probe> {
    let (width, height) = match *CANVAS.lock() {
        Some((width, height)) => (width.min(shot.size.width), height.min(shot.size.height)),
        None => (shot.size.width, shot.size.height),
    };

    let margin = GRID_STEP.max(STABLE_RADIUS);

    let mut candidates: Vec<Probe> = vec![];

    for y in (margin..height.saturating_sub(margin)).step_by(GRID_STEP as usize) {
        for x in (margin..width.saturating_sub(margin)).step_by(GRID_STEP as usize) {
            if let Some(color) = stable_color(shot, x, y) {
                candidates.push(((x, y), color));
            }
        }
    }

    let (small, open): (Vec<Probe>, Vec<Probe>) =
        candidates.into_iter().partition(|probe| is_small_feature(shot, probe));

    let mut clusters = cluster_by_color(small);
    clusters.append(&mut cluster_by_color(open));

    let center = (f64::from(width) / 2.0, f64::from(height) / 2.0);

    let mut clusters: Vec<Vec<Candidate>> = clusters
        .into_iter()
        .map(|cluster| {
            cluster
                .into_iter()
                .map(|probe| {
                    let (x, y) = position(&probe);
                    Candidate {
                        probe,
                        center_dist: (x - center.0).hypot(y - center.1),
                        selected_dist: f64::INFINITY,
                    }
                })
                .collect()
        })
        .collect();

    let target = probe_count();
    let mut selected: Vec<Probe> = vec![];

    while selected.len() < target {
        let mut exhausted = true;

        for i in 0..clusters.len() {
            if selected.len() == target {
                break;
            }

            let Some(best) = take_best(&mut clusters[i]) else {
                continue;
            };
            exhausted = false;

            let (bx, by) = position(&best);

            for cluster in &mut clusters {
                for candidate in cluster.iter_mut() {
                    let (x, y) = position(&candidate.probe);
                    let dist = (x - bx).hypot(y - by);

                    if dist < candidate.selected_dist {
                        candidate.selected_dist = dist;
                    }
                }
            }

            selected.push(best);
        }

        if exhausted {
            break;
        }
    }

    selected.sort_by_key(|((x, y), _)| (*y, *x));

    selected
}

struct Candidate {
    probe: Probe,

    center_dist: f64,

    /// Distance to the closest already selected probe. Updated on every
    /// selection, so a pick costs one pass instead of a rescan.
    selected_dist: f64,
}

fn position(probe: &Probe) -> (f64, f64) {
    let ((x, y), _) = probe;
    (f64::from(*x), f64::from(*y))
}

/// Removes and returns the cluster member farthest from every selected
/// probe. Before anything is selected all distances tie at infinity,
/// then the tie breaks toward the screen center.
fn take_best(cluster: &mut Vec<Candidate>) -> Option<Probe> {
    if cluster.is_empty() {
        return None;
    }

    let mut best = 0;

    for i in 1..cluster.len() {
        let better = match cluster[i].selected_dist.total_cmp(&cluster[best].selected_dist) {
            Greater => true,
            Less => false,
            Equal => cluster[i].center_dist < cluster[best].center_dist,
        };

        if better {
            best = i;
        }
    }

    Some(cluster.remove(best).probe)
}

/// Sorted by size, so the round robin serves bigger regions first.
fn cluster_by_color(candidates: Vec<Probe>) -> Vec<Vec<Probe>> {
    let mut clusters: Vec<Vec<Probe>> = vec![];

    for candidate in candidates {
        match clusters.iter_mut().find(|c| c[0].1.diff_u8(candidate.1) <= CLUSTER_TOLERANCE) {
            Some(cluster) => cluster.push(candidate),
            None => clusters.push(vec![candidate]),
        }
    }

    clusters.sort_by_key(|c| usize::MAX - c.len());

    clusters
}

/// The color of the candidate runs out within `SMALL_FEATURE_EXTENT`
/// in all eight directions, so it sits in an enclosed pocket.
fn is_small_feature(shot: &Screenshot, probe: &Probe) -> bool {
    const DIRECTIONS: [(i64, i64); 8] = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];

    DIRECTIONS
        .iter()
        .all(|dir| directional_run(shot, probe, *dir) <= SMALL_FEATURE_EXTENT)
}

/// How far the probe color continues in one direction, capped just
/// past `SMALL_FEATURE_EXTENT`. The window edge counts as a boundary.
fn directional_run(shot: &Screenshot, probe: &Probe, (dx, dy): (i64, i64)) -> u32 {
    let ((x, y), color) = probe;

    for step in 1..=SMALL_FEATURE_EXTENT {
        let px = i64::from(*x) + dx * i64::from(step);
        let py = i64::from(*y) + dy * i64::from(step);

        let (Ok(px), Ok(py)) = (u32::try_from(px), u32::try_from(py)) else {
            return step;
        };

        if px >= shot.size.width || py >= shot.size.height {
            return step;
        }

        if shot.get_pixel((px, py)).diff_u8(*color) > CLUSTER_TOLERANCE {
            return step;
        }
    }

    SMALL_FEATURE_EXTENT + 1
}

fn stable_color(shot: &Screenshot, x: u32, y: u32) -> Option<U8Color> {
    let center = shot.get_pixel((x, y));

    let uniform_along = |dx: u32, dy: u32| {
        shot.get_pixel((x - dx, y - dy)).diff_u8(center) <= STABLE_TOLERANCE
            && shot.get_pixel((x + dx, y + dy)).diff_u8(center) <= STABLE_TOLERANCE
    };

    // Vertical catches a stem, horizontal catches a bar.
    if uniform_along(0, STABLE_RADIUS) || uniform_along(STABLE_RADIUS, 0) {
        Some(center)
    } else {
        None
    }
}
