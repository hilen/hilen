use super::{ADD_BG, DEL_BG, DISCARD_W, DiffPanel, DiffRow, GUTTER_W, Half, MARKER_W, content_x, reserved_w};
use crate::{
    refs::Weak,
    ui::{
        Button, CLEAR, Color, Container, Label, Setup, TextAlignment, UIColor, ViewData, ViewFrame,
        ViewSubviews, ViewTouch, diff_colors as colors, view,
    },
};

const SELECTION_BG: Color = Color::rgba(0.2, 0.5, 1.0, 0.35);

/// Cuts the code text at the edges of its side while the text scrolls
/// sideways under the pinned gutter.
/// Which half of a row has discard buttons, the left one, or the only
/// one of a unified row, and the right one of a split row.
#[derive(Default)]
struct Discards {
    left:  bool,
    right: bool,
}

#[view]
struct CodeClip {}

impl Setup for CodeClip {
    fn clips_to_bounds(&self) -> bool {
        true
    }

    fn setup(self: Weak<Self>) {}
}

#[view]
pub(super) struct DiffLineCell {
    panel:         Weak<DiffPanel>,
    hunk_index:    usize,
    split:         bool,
    line_index:    Option<usize>,
    left_index:    Option<usize>,
    right_index:   Option<usize>,
    // The discard buttons exist while a row is discardable but show only
    // while the row is hovered, the original's opacity-0 to
    // group-hover:opacity-100. The reserved content width follows these,
    // not the hidden flag, so nothing jumps on hover.
    discards:      Discards,
    hovered:       bool,
    // The selected byte range of this row's text and whether it is the
    // right half of a split row, set by the panel, see selection.rs.
    selection:     Option<(usize, usize, bool)>,
    // The code text and its highlight live inside the clips and move
    // with the sideways scroll of the table. Each highlight is added
    // before its label so it draws behind the glyphs, a later sibling
    // draws in front.
    sel_bg:        Weak<Container>,
    content:       Weak<Label>,
    right_sel_bg:  Weak<Container>,
    right_content: Weak<Label>,

    #[init]
    left_bg:       Container,
    right_bg:      Container,
    old_no:        Label,
    new_no:        Label,
    marker:        Label,
    clip:          CodeClip,
    right_marker:  Label,
    right_clip:    CodeClip,
    divider:       Container,
    discard:       Button,
    right_discard: Button,
}

impl Setup for DiffLineCell {
    fn setup(mut self: Weak<Self>) {
        self.sel_bg = self.clip.add_view();
        self.content = self.clip.add_view();
        self.right_sel_bg = self.right_clip.add_view();
        self.right_content = self.right_clip.add_view();
        for gutter in [self.old_no, self.new_no] {
            gutter
                .set_text_size(10)
                .set_text_color(colors::MUTED)
                .set_alignment(TextAlignment::Right);
        }
        for marker in [self.marker, self.right_marker] {
            marker.set_text_size(12);
        }
        for content in [self.content, self.right_content] {
            content.set_text_size(12).set_alignment(TextAlignment::Left);
        }
        self.divider.set_color(colors::BORDER);
        for sel_bg in [self.sel_bg, self.right_sel_bg] {
            sel_bg.set_color(SELECTION_BG);
            sel_bg.set_hidden(true);
        }
        for discard in [self.discard, self.right_discard] {
            discard.set_text("discard");
            discard.set_text_size(9);
            discard.set_corner_radius(3);
            discard.set_color(CLEAR);
            discard.set_text_color(colors::MUTED);
            // The muted text goes red while the button itself is hovered,
            // and hovering the button keeps the reveal alive, the cell
            // loses the hover event to the topmost view.
            discard.enable_hover();
            discard.touch().hovered.val(self, move |hovered| {
                self.hovered = hovered;
                self.apply_discard_visibility();
                let color: UIColor = if hovered {
                    Color::hex("#ef4444").into()
                } else {
                    colors::MUTED.into()
                };
                discard.set_text_color(color);
            });
        }
        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| {
            self.hovered = hovered;
            self.apply_discard_visibility();
        });
        self.discard.on_tap(move || match self.left_index {
            Some(line_index) => self.panel.discard_line(self.hunk_index, line_index),
            None => match self.line_index {
                Some(line_index) => self.panel.discard_line(self.hunk_index, line_index),
                None => self.panel.discard_hunk(self.hunk_index),
            },
        });
        self.right_discard.on_tap(move || {
            if let Some(line_index) = self.right_index {
                self.panel.discard_line(self.hunk_index, line_index);
            }
        });
        self.size_changed().sub(move || self.layout());
    }
}

impl DiffLineCell {
    pub(super) fn set_row(
        mut self: Weak<Self>,
        row: DiffRow,
        can_discard: bool,
        panel: Weak<DiffPanel>,
    ) -> Weak<Self> {
        self.panel = panel;
        let mono = panel.mono();
        self.content.set_font(mono);
        self.right_content.set_font(mono);
        self.split = false;
        self.left_index = None;
        self.right_index = None;
        self.line_index = None;
        self.right_marker.set_text("");
        self.right_content.set_text("");
        // Hunk titles and notes stay in place, only code scrolls sideways.
        let code = matches!(row, DiffRow::Line { .. } | DiffRow::Split { .. });
        self.content.set_moves_sideways(code);
        self.sel_bg.set_moves_sideways(code);
        self.right_content.set_moves_sideways(code);
        self.right_sel_bg.set_moves_sideways(code);
        match row {
            DiffRow::Hunk { title, hunk_index } => {
                self.hunk_index = hunk_index;
                self.set_color(colors::BG_BAR);
                self.old_no.set_text("");
                self.new_no.set_text("");
                self.marker.set_text("");
                self.content.set_text(title).set_text_size(11).set_text_color(colors::MUTED);
                self.content.clear_color_runs();
                self.discards.left = can_discard;
            }
            DiffRow::Line {
                old_no,
                new_no,
                kind,
                text,
                hunk_index,
                line_index,
                global,
            } => {
                self.hunk_index = hunk_index;
                self.line_index = Some(line_index);
                let bg: UIColor = match kind {
                    '+' => ADD_BG.into(),
                    '-' => DEL_BG.into(),
                    _ => CLEAR.into(),
                };
                self.set_color(bg);
                self.old_no.set_text(old_no.map_or(String::new(), |n| n.to_string()));
                self.new_no.set_text(new_no.map_or(String::new(), |n| n.to_string()));
                Self::set_marker(self.marker, kind);
                self.set_content(self.content, text, global);
                self.discards.left = can_discard && kind != ' ';
            }
            DiffRow::Split {
                left,
                right,
                hunk_index,
            } => {
                self.split = true;
                self.hunk_index = hunk_index;
                self.set_color(CLEAR);
                self.set_half(left, true, can_discard);
                self.set_half(right, false, can_discard);
            }
            DiffRow::Note(text) => {
                self.set_color(CLEAR);
                self.old_no.set_text("");
                self.new_no.set_text("");
                self.marker.set_text("");
                self.content.set_text(text).set_text_size(12).set_text_color(colors::MUTED);
                self.content.clear_color_runs();
                self.discards.left = false;
            }
        }
        self.apply_discard_visibility();
        self.layout();
        self
    }

    fn set_half(mut self: Weak<Self>, half: Option<Half>, left: bool, can_discard: bool) {
        let (bg_view, gutter, marker, content, discard) = if left {
            (self.left_bg, self.old_no, self.marker, self.content, self.discard)
        } else {
            (
                self.right_bg,
                self.new_no,
                self.right_marker,
                self.right_content,
                self.right_discard,
            )
        };
        let Some(half) = half else {
            // The empty side of an unpaired add or remove.
            bg_view.set_color(colors::BG_PANEL.resolve().with_alpha(0.4));
            gutter.set_text("");
            marker.set_text("");
            content.set_text("");
            content.clear_color_runs();
            if left {
                self.discards.left = false;
            } else {
                self.discards.right = false;
            }
            discard.set_hidden(true);
            return;
        };
        let bg: UIColor = match half.kind {
            '+' => ADD_BG.into(),
            '-' => DEL_BG.into(),
            _ => CLEAR.into(),
        };
        bg_view.set_color(bg);
        gutter.set_text(half.no.map_or(String::new(), |n| n.to_string()));
        Self::set_marker(marker, half.kind);
        self.set_content(content, half.text, half.global);
        let discardable = can_discard && half.kind != ' ';
        if left {
            self.discards.left = discardable;
        } else {
            self.discards.right = discardable;
        }
        if discardable {
            if left {
                self.left_index = Some(half.line_index);
            } else {
                self.right_index = Some(half.line_index);
            }
        }
    }

    // DiffShown only while the cell or the button itself is hovered, and only
    // for the sides that can discard at all.
    fn apply_discard_visibility(self: Weak<Self>) {
        self.discard.set_hidden(!(self.hovered && self.discards.left));
        self.right_discard
            .set_hidden(!(self.hovered && self.split && self.discards.right));
    }

    fn set_marker(marker: Weak<Label>, kind: char) {
        let color: UIColor = match kind {
            '+' => colors::STAT_ADD.into(),
            '-' => colors::STAT_DEL.into(),
            _ => colors::MUTED.into(),
        };
        marker.set_text(if kind == ' ' {
            String::new()
        } else {
            kind.to_string()
        });
        marker.set_text_color(color);
    }

    // The content label with the syntax color runs of its diff line.
    fn set_content(self: Weak<Self>, label: Weak<Label>, text: String, global: usize) {
        label.set_text(text).set_text_size(12).set_text_color(colors::FG);
        match self.panel.runs.get(global) {
            Some(runs) if !runs.is_empty() => {
                label
                    .set_color_runs(runs.iter().map(|(range, color)| (range.clone(), UIColor::from(*color))));
            }
            _ => {
                label.clear_color_runs();
            }
        }
    }

    /// Which half of a split row `x` is on, in the cell's coordinates.
    pub(super) fn on_right_half(&self, x: f32) -> bool {
        self.split && x >= self.width() / 2.0
    }

    /// The byte in this row's text nearest to `x`, cell coordinates.
    pub(super) fn byte_at(&self, x: f32, right: bool) -> usize {
        let label = self.sel_label(right);
        let text = label.text().to_string();
        if text.is_empty() {
            return 0;
        }
        let layout = label.text_layout_for(&text);
        // Absolute frames carry the sideways scroll offset of the label.
        let label_x = label.absolute_frame().origin.x - self.absolute_frame().origin.x;
        layout.nearest_on_line(0, x - label_x)
    }

    pub(super) fn set_selection(mut self: Weak<Self>, selection: Option<(usize, usize, bool)>) {
        self.selection = selection;
        self.apply_selection_frame();
    }

    fn sel_label(&self, right: bool) -> Weak<Label> {
        if right && self.split {
            self.right_content
        } else {
            self.content
        }
    }

    fn apply_selection_frame(self: Weak<Self>) {
        self.sel_bg.set_hidden(true);
        self.right_sel_bg.set_hidden(true);
        let Some((start, end, right)) = self.selection else {
            return;
        };
        let label = self.sel_label(right);
        let text = label.text().to_string();
        if text.is_empty() || start >= end {
            return;
        }
        let layout = label.text_layout_for(&text);
        let from = layout.x_on_line(0, start);
        let to = layout.x_on_line(0, end);
        if to <= from {
            return;
        }
        let sel_bg = if right && self.split {
            self.right_sel_bg
        } else {
            self.sel_bg
        };
        // The label starts at the left edge of its clip, so the highlight
        // shares its coordinates and its sideways scroll.
        sel_bg.set_frame((from, 0, to - from, self.height()));
        sel_bg.set_hidden(false);
    }

    // The width of a code label: the widest line of the diff, so every
    // row scrolls by the same range, and never less than its clip.
    fn label_width(&self, clip_w: f32) -> f32 {
        let text_w = if self.panel.is_ok() {
            self.panel.text_width
        } else {
            0.0
        };
        text_w.max(clip_w)
    }

    fn layout(self: Weak<Self>) {
        if self.split {
            self.layout_split();
            self.apply_selection_frame();
            return;
        }
        let w = self.width();
        let h = self.height();
        self.left_bg.set_hidden(true);
        self.right_bg.set_hidden(true);
        self.divider.set_hidden(true);
        self.right_marker.set_hidden(true);
        self.right_clip.set_hidden(true);
        self.right_discard.set_hidden(true);
        self.old_no.set_frame((0, 0, GUTTER_W, h));
        self.new_no.set_frame((GUTTER_W, 0, GUTTER_W, h));
        self.marker.set_frame((GUTTER_W * 2.0 + 2.0, 0, MARKER_W, h));
        let x = content_x(false);
        let clip_w = (w - x - reserved_w(self.discards.left)).max(0.0);
        self.clip.set_frame((x, 0, clip_w, h));
        self.content.set_frame((0, 0, self.label_width(clip_w), h));
        self.discard.set_frame((w - DISCARD_W - 4.0, 1.0, DISCARD_W, h - 2.0));
        self.apply_selection_frame();
    }

    fn layout_split(self: Weak<Self>) {
        let w = self.width();
        let h = self.height();
        let half = w / 2.0;
        self.left_bg.set_hidden(false);
        self.right_bg.set_hidden(false);
        self.divider.set_hidden(false);
        self.right_marker.set_hidden(false);
        self.right_clip.set_hidden(false);
        self.left_bg.set_frame((0, 0, half, h));
        self.right_bg.set_frame((half, 0, w - half, h));
        self.divider.set_frame((half - 0.5, 0, 1, h));

        let content_x = content_x(true);
        let left_w = (half - content_x - reserved_w(self.discards.left)).max(0.0);
        self.old_no.set_frame((0, 0, GUTTER_W, h));
        self.marker.set_frame((GUTTER_W + 2.0, 0, MARKER_W, h));
        self.clip.set_frame((content_x, 0, left_w, h));
        self.content.set_frame((0, 0, self.label_width(left_w), h));
        self.discard.set_frame((half - DISCARD_W - 4.0, 1.0, DISCARD_W, h - 2.0));

        let right_w = (half - content_x - reserved_w(self.discards.right)).max(0.0);
        self.new_no.set_frame((half, 0, GUTTER_W, h));
        self.right_marker.set_frame((half + GUTTER_W + 2.0, 0, MARKER_W, h));
        self.right_clip.set_frame((half + content_x, 0, right_w, h));
        self.right_content.set_frame((0, 0, self.label_width(right_w), h));
        self.right_discard.set_frame((w - DISCARD_W - 4.0, 1.0, DISCARD_W, h - 2.0));
    }
}

// The highlight draws behind the glyphs, an opaque fill would also hide
// the add and remove row tints under it.
const _: () = assert!(SELECTION_BG.a < 0.5);
