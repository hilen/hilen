use crate::gm::LossyConvert;

/// Where every row of a table starts and how tall it is. Uniform tables
/// answer from one pitch and never allocate. Variable tables carry the
/// running offsets, one per row plus the end, built once per reload.
pub(super) enum Rows<'a> {
    Uniform { count: usize, height: f32, pitch: f32 },
    Variable { offsets: &'a [f32], spacing: f32 },
}

/// The running row offsets of a variable table, one per row plus the end.
pub(super) fn row_offsets(heights: impl Iterator<Item = f32>, spacing: f32) -> Vec<f32> {
    let mut offsets = vec![0.0];
    let mut y = 0.0;

    for height in heights {
        y += height + spacing;
        offsets.push(y);
    }

    offsets
}

impl Rows<'_> {
    pub(super) fn uniform(count: usize, height: f32, spacing: f32) -> Self {
        Self::Uniform {
            count,
            height,
            pitch: height + spacing,
        }
    }

    pub(super) fn count(&self) -> usize {
        match self {
            Self::Uniform { count, .. } => *count,
            Self::Variable { offsets, .. } => offsets.len() - 1,
        }
    }

    pub(super) fn top(&self, row: usize) -> f32 {
        match self {
            Self::Uniform { pitch, .. } => row.lossy_convert() * pitch,
            Self::Variable { offsets, .. } => offsets[row],
        }
    }

    pub(super) fn height(&self, row: usize) -> f32 {
        match self {
            Self::Uniform { height, .. } => *height,
            Self::Variable { offsets, spacing } => offsets[row + 1] - offsets[row] - spacing,
        }
    }

    /// Content height of all rows, no gap after the last one.
    pub(super) fn total(&self) -> f32 {
        if self.count() == 0 {
            return 0.0;
        }

        match self {
            Self::Uniform {
                count, pitch, height, ..
            } => (count - 1).lossy_convert() * pitch + height,
            Self::Variable { offsets, spacing } => offsets[offsets.len() - 1] - spacing,
        }
    }

    /// The row whose span, gap below included, holds `y`. Clamped to the
    /// last row past the end and to the first above the top.
    pub(super) fn row_at(&self, y: f32) -> usize {
        if self.count() == 0 || y <= 0.0 {
            return 0;
        }

        let row = match self {
            Self::Uniform { pitch, .. } => (y / pitch).floor().lossy_convert(),
            Self::Variable { offsets, .. } => offsets.partition_point(|top| *top <= y) - 1,
        };

        row.min(self.count() - 1)
    }

    /// The row a tap at `y` selects. A tap in the gap below a row goes to
    /// the closer of the two rows, so gaps are purely visual for touch.
    pub(super) fn row_for_tap(&self, y: f32) -> usize {
        let row = self.row_at(y);

        if row + 1 < self.count() && y > self.top(row) + self.height(row) {
            let gap_middle = (self.top(row) + self.height(row) + self.top(row + 1)) / 2.0;
            if y >= gap_middle {
                return row + 1;
            }
        }

        row
    }
}
