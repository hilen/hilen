use std::collections::HashMap;

use web_time::{Duration, Instant};

use crate::window::text::shaped_layout::ShapedGlyph;

/// How long an unused line stays cached before the sweep drops it.
const KEEP: Duration = Duration::from_secs(10);

/// How often the sweep actually scans the cache.
const SWEEP_EVERY: Duration = Duration::from_secs(1);

/// Shaping inputs that change the glyphs, the outer map key. Float bits
/// stand in for the floats themselves, which are not `Eq`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ShapeParams {
    px_per_unit: u32,
    tracking:    u32,
}

struct CachedLine {
    /// Clusters are relative to the line start.
    glyphs:    Vec<ShapedGlyph>,
    last_used: Instant,
}

/// Caches rustybuzz output per line of text, owned by a `Font`.
///
/// `glyph_brush` has its own shaped section cache, but every
/// `process_queued` call drops the entries absent from that batch, and
/// clip boundaries process several batches per frame, so nothing in it
/// survives a frame and every label reshapes every frame. Shaping is
/// almost the entire cost of a text heavy frame. This cache lives
/// outside those batches, so a line shapes once and repositions cheaply
/// from then on. It also serves `Font::measure`, which shapes through
/// the same path.
#[derive(Default)]
pub(crate) struct ShapeCache {
    lines:      HashMap<ShapeParams, HashMap<String, CachedLine>>,
    last_sweep: Option<Instant>,
}

impl ShapeCache {
    /// The glyphs of `line`, shaping it on a miss. Clusters in the
    /// result are relative to the line start.
    pub(crate) fn get_or_shape(
        &mut self,
        line: &str,
        px_per_unit: f32,
        tracking: f32,
        shape: impl FnOnce() -> Vec<ShapedGlyph>,
    ) -> Vec<ShapedGlyph> {
        let params = ShapeParams {
            px_per_unit: px_per_unit.to_bits(),
            tracking:    tracking.to_bits(),
        };

        let now = Instant::now();
        let lines = self.lines.entry(params).or_default();

        if let Some(cached) = lines.get_mut(line) {
            cached.last_used = now;
            return cached.glyphs.clone();
        }

        let cached = CachedLine {
            glyphs:    shape(),
            last_used: now,
        };
        lines.entry(line.to_string()).or_insert(cached).glyphs.clone()
    }

    /// Drops lines unused for [`KEEP`]. Call freely, the scan itself
    /// runs once per [`SWEEP_EVERY`].
    pub(crate) fn sweep(&mut self) {
        let now = Instant::now();

        if self.last_sweep.is_some_and(|last| now - last < SWEEP_EVERY) {
            return;
        }
        self.last_sweep = Some(now);

        for lines in self.lines.values_mut() {
            lines.retain(|_, line| now - line.last_used < KEEP);
        }
        self.lines.retain(|_, lines| !lines.is_empty());
    }
}
