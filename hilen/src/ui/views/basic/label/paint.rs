use std::mem::discriminant;

use super::{ColorRun, Label};
use crate::ui::damage::PaintKey;

impl Label {
    /// Everything of the label the drawer reads to draw it, beyond what
    /// every view has, see `Walk::key`. A new field of `Label` that
    /// changes its look is fed here in the same change, or a label that
    /// only changes that field keeps its old pixels.
    pub(crate) fn feed_paint(&self, key: &mut PaintKey) {
        key.hash(&self.text)
            .hash(&discriminant(&self.alignment))
            .hash(&discriminant(&self.vertical_alignment))
            .hash(&self.secret)
            .hash(&self.multiline)
            .hash(&self.max_lines)
            .hash(&discriminant(&self.ellipsize))
            .hash(&discriminant(&self.placing))
            .hash(&self.box_chars)
            .pod(&self.text_color)
            .pod(&self.text_size)
            .pod(&self.letter_spacing)
            .hash(&self.font().name);

        if let Some(end) = self.text_end_color {
            key.pod(&end);
        }
        if let Some(height) = self.line_height {
            key.pod(&height);
        }
        if let Some(outline) = self.text_outline() {
            key.debug(&outline);
        }
        if let Some(shadow) = self.text_shadow() {
            key.debug(&shadow);
        }

        feed_runs(key, &self.color_runs);
        // A mark between the 2 lists, the same run in the other list is
        // another look.
        key.hash(&self.color_runs.len());
        feed_runs(key, &self.background_runs);

        for run in &self.font_runs {
            key.hash(&run.range).hash(&run.style.underline).hash(&run.style.strikethrough);
            if let Some(font) = &run.style.font {
                key.hash(&font.name);
            }
        }
    }
}

fn feed_runs(key: &mut PaintKey, runs: &[ColorRun]) {
    for run in runs {
        key.hash(&run.range).pod(&run.color);
    }
}
