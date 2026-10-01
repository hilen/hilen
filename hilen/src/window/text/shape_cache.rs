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
    ///
    /// A `secret` line is shaped every time and never stored. The cache
    /// keeps a copy of every line as its key, for 10 seconds after the
    /// last use and with no end while no frame is drawn, and a secret
    /// must not sit there.
    pub(crate) fn get_or_shape(
        &mut self,
        line: &str,
        px_per_unit: f32,
        tracking: f32,
        secret: bool,
        shape: impl FnOnce() -> Vec<ShapedGlyph>,
    ) -> Vec<ShapedGlyph> {
        if secret {
            return shape();
        }

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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::ShapeCache;

    fn cached_lines(cache: &ShapeCache) -> usize {
        cache.lines.values().map(HashMap::len).sum()
    }

    #[test]
    fn a_plain_line_is_shaped_once_and_kept() {
        let mut cache = ShapeCache::default();
        let mut shaped = 0;

        for _ in 0..2 {
            cache.get_or_shape("hello", 0.01, 0.0, false, || {
                shaped += 1;
                vec![]
            });
        }

        assert_eq!(shaped, 1);
        assert_eq!(cached_lines(&cache), 1);
    }

    #[test]
    fn a_secret_line_is_shaped_every_time_and_never_kept() {
        let mut cache = ShapeCache::default();
        let mut shaped = 0;

        for _ in 0..2 {
            cache.get_or_shape("twelve secret words", 0.01, 0.0, true, || {
                shaped += 1;
                vec![]
            });
        }

        assert_eq!(shaped, 2);
        assert_eq!(cached_lines(&cache), 0);
    }
}
