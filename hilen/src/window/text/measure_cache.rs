use std::collections::HashMap;

use web_time::{Duration, Instant};

use crate::{gm::flat::Size, window::text::Shaping};

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

impl MeasureKey {
    /// The key of one measurement, `None` for a secret. The key holds a
    /// copy of the text and the cache keeps it for 10 seconds after the
    /// last use, with no end while no frame is drawn, so a secret is
    /// measured every time and never stored.
    pub(crate) fn of(text: &str, size: f32, width: Option<f32>, shaping: &Shaping) -> Option<Self> {
        if shaping.secret {
            return None;
        }
        Some(Self {
            text:        text.to_string(),
            size:        size.to_bits(),
            width:       width.map(f32::to_bits),
            tracking:    shaping.tracking.to_bits(),
            line_height: shaping.line_height.map(f32::to_bits),
            runs:        shaping
                .runs
                .iter()
                .map(|run| (run.font.name.clone(), run.range.start, run.range.end))
                .collect(),
        })
    }
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
    use crate::{gm::flat::Size, window::text::Shaping};

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

    #[test]
    fn a_plain_text_gets_the_key_the_cache_stores_it_under() {
        let made = MeasureKey::of("hello", 12.0, None, &Shaping::default());

        assert!(made == Some(key("hello")));
    }

    #[test]
    fn a_secret_gets_no_key_so_the_cache_never_holds_its_text() {
        let shaping = Shaping {
            secret: true,
            ..Shaping::default()
        };

        assert!(MeasureKey::of("twelve secret words", 12.0, None, &shaping).is_none());
    }
}
