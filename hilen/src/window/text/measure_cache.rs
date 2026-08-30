use std::collections::HashMap;

use web_time::{Duration, Instant};

use crate::gm::flat::Size;

/// How long an unused measurement stays cached before the sweep drops it.
const KEEP: Duration = Duration::from_secs(10);

/// How often the sweep actually scans the cache.
const SWEEP_EVERY: Duration = Duration::from_secs(1);

/// Everything `Font::measure` depends on. Float bits stand in for the
/// floats themselves, which are not `Eq`. Runs are keyed by font name
/// and byte range, the two parts of a run that change the result.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct MeasureKey {
    pub text:        String,
    pub size:        u32,
    pub width:       Option<u32>,
    pub tracking:    u32,
    pub line_height: Option<u32>,
    pub runs:        Vec<(String, usize, usize)>,
}

struct CachedMeasure {
    size:      Size,
    last_used: Instant,
}

/// Caches `Font::measure` results, owned by a `Font`.
///
/// The shape cache below it removes the shaping cost, but `glyph_brush`
/// still walks every glyph's bounds through `ttf_parser` on every call.
/// A log pane measuring hundreds of spans per rebuild spent seconds
/// there, so the finished size is cached whole.
#[derive(Default)]
pub(crate) struct MeasureCache {
    sizes:      HashMap<MeasureKey, CachedMeasure>,
    last_sweep: Option<Instant>,
}

impl MeasureCache {
    /// Split get and insert instead of one closure taking entry point,
    /// because the measurement itself needs the same `&mut Font` that
    /// owns this cache.
    pub(crate) fn get(&mut self, key: &MeasureKey) -> Option<Size> {
        let cached = self.sizes.get_mut(key)?;
        cached.last_used = Instant::now();
        Some(cached.size)
    }

    pub(crate) fn insert(&mut self, key: MeasureKey, size: Size) {
        self.sizes.insert(
            key,
            CachedMeasure {
                size,
                last_used: Instant::now(),
            },
        );
    }

    /// Drops measurements unused for [`KEEP`]. Call freely, the scan
    /// itself runs once per [`SWEEP_EVERY`].
    pub(crate) fn sweep(&mut self) {
        let now = Instant::now();

        if self.last_sweep.is_some_and(|last| now - last < SWEEP_EVERY) {
            return;
        }
        self.last_sweep = Some(now);

        self.sizes.retain(|_, cached| now - cached.last_used < KEEP);
    }
}

#[cfg(test)]
mod tests {
    use super::{MeasureCache, MeasureKey};
    use crate::gm::flat::Size;

    fn key(text: &str) -> MeasureKey {
        MeasureKey {
            text:        text.to_string(),
            size:        12.0_f32.to_bits(),
            width:       None,
            tracking:    0.0_f32.to_bits(),
            line_height: None,
            runs:        vec![],
        }
    }

    #[test]
    fn a_hit_returns_the_cached_size() {
        let mut cache = MeasureCache::default();

        assert_eq!(cache.get(&key("hello")), None);
        cache.insert(key("hello"), Size::new(50.0, 14.0));
        assert_eq!(cache.get(&key("hello")), Some(Size::new(50.0, 14.0)));
    }

    #[test]
    fn different_inputs_are_separate_entries() {
        let mut cache = MeasureCache::default();

        cache.insert(key("hello"), Size::new(50.0, 14.0));

        let mut other = key("hello");
        other.size = 13.0_f32.to_bits();
        assert_eq!(cache.get(&other), None);
    }
}
