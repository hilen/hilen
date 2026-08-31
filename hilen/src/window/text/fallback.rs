//! Glyph fallback. A char the effective font has no glyph for is drawn
//! with the first registered fallback font that covers it, through a
//! synthesized font run, so wrapping and measuring follow automatically.

use std::ops::Range;

use crate::{
    deps::refs::Weak,
    window::{Font, text::FontRun},
};

/// Rebuilds `runs` so every char an effective font misses shapes with
/// the first fallback that covers it. With no fallbacks registered the
/// explicit runs come back untouched. Whitespace and control chars stay
/// with their font, shapers handle them without a glyph.
pub(crate) fn runs_with_fallbacks(text: &str, base: Weak<Font>, runs: Vec<FontRun>) -> Vec<FontRun> {
    let fallbacks = Font::fallbacks();
    if fallbacks.is_empty() || text.is_empty() {
        return runs;
    }

    let effective =
        |byte: usize| runs.iter().find(|run| run.range.contains(&byte)).map_or(base, |run| run.font);

    let mut result: Vec<FontRun> = vec![];
    let mut push = |range: Range<usize>, font: Weak<Font>| {
        if let Some(last) = result.last_mut()
            && last.font.name == font.name
            && last.range.end == range.start
        {
            last.range.end = range.end;
            return;
        }
        result.push(FontRun { range, font });
    };

    for (byte, char) in text.char_indices() {
        let font = effective(byte);
        let range = byte..byte + char.len_utf8();

        if char.is_whitespace() || char.is_control() || font.has_glyph(char) {
            push(range, font);
            continue;
        }

        let font = fallbacks
            .iter()
            .copied()
            .find(|fallback| fallback.has_glyph(char))
            .unwrap_or(font);
        push(range, font);
    }

    // Base font spans need no run, absence of one means the base font.
    result.retain(|run| run.font.name != base.name);
    result
}
