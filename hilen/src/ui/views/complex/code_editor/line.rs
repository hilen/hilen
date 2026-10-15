use std::ops::Range;

use super::style::CodeEditorStyle;
use crate::{
    self as hilen,
    deps::refs::Weak,
    gm::color::CLEAR,
    ui::{Container, Label, Setup, TextAlignment, UIColor, View, ViewData, ViewFrame, view},
};

/// The width of the caret in points.
const CARET_WIDTH: f32 = 2.0;
/// The message of a mark starts this many chars after the code.
const MESSAGE_GAP: f32 = 3.0;

/// Where the parts of a row stand, the same for every row.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Columns {
    pub row_height:   f32,
    pub gutter_width: f32,
    /// Where the code starts.
    pub text_x:       f32,
    /// The width of 1 char of the font.
    pub char_width:   f32,
    /// The width of the code label, the widest line and more.
    pub text_width:   f32,
}

/// What 1 row shows.
pub(super) struct LineLook<'a> {
    pub number:      usize,
    pub text:        &'a str,
    pub runs:        &'a [(Range<usize>, UIColor)],
    /// The selected bytes and whether the selection goes on past the end.
    pub selected:    Option<(Range<usize>, bool)>,
    /// The byte of the caret, when the caret is on this line.
    pub caret:       Option<usize>,
    pub mark:        Option<&'a str>,
    pub highlighted: bool,
}

/// Puts the font and the size of the style on a label of the code. The
/// label that measures is filled by this too, so a measure is what a row
/// draws.
pub(super) fn code_label(label: Weak<Label>, style: &CodeEditorStyle) {
    label
        .set_font(style.regular_font())
        .set_text_size(style.text_size)
        .set_cell_grid(true)
        .set_alignment(TextAlignment::Left);
}

/// 1 line of a `CodeEditor`: the selection behind the code, the code, the
/// caret, the message of a mark and the number in the gutter. The gutter
/// is added after the code, so code scrolled sideways goes under it.
#[view]
pub(super) struct CodeLine {
    #[init]
    selection: Container,
    code:      Label,
    caret:     Container,
    message:   Label,
    gutter:    Container,
    number:    Label,
}

impl Setup for CodeLine {
    fn setup(self: Weak<Self>) {
        let moving = [
            self.selection.weak_view(),
            self.code.weak_view(),
            self.caret.weak_view(),
            self.message.weak_view(),
        ];
        for view in moving {
            view.set_moves_sideways(true);
        }
        self.number.set_alignment(TextAlignment::Right);
        self.code.set_color(CLEAR);
        self.message.set_color(CLEAR);
        self.number.set_color(CLEAR);
    }
}

impl CodeLine {
    pub(super) fn show(self: Weak<Self>, look: &LineLook, columns: &Columns, style: &CodeEditorStyle) {
        let height = columns.row_height;

        let background = if look.mark.is_some() {
            style.mark_line
        } else if look.highlighted {
            style.highlight
        } else {
            CLEAR.into()
        };
        self.set_color(background);

        self.gutter.set_color(style.gutter);
        self.gutter.set_frame((0.0, 0.0, columns.gutter_width, height));

        code_label(self.number, style);
        self.number.set_cell_grid(false);
        self.number
            .set_text_color(if look.mark.is_some() {
                style.mark
            } else {
                style.line_number
            })
            .set_text(look.number.to_string());
        // The digits end 1 char before the code, the label keeps a strip
        // beside its text.
        let number_inset = self.number.text_inset();
        self.number.set_frame((
            0.0,
            0.0,
            columns.gutter_width - columns.char_width + number_inset,
            height,
        ));

        code_label(self.code, style);
        self.code.set_text_color(style.text).set_text(look.text);
        self.code.set_color_runs(look.runs.iter().cloned());
        let inset = self.code.text_inset();
        self.code
            .set_frame((columns.text_x - inset, 0.0, columns.text_width + inset, height));

        let layout = self.code.text_layout_for(look.text);
        let x_of = |byte: usize| columns.text_x + layout.x_on_line(0, byte);
        let line_end = x_of(look.text.len());

        // 1 rect as high as the row, so the selected rows join with no
        // seam. A selection over the line break takes 1 char more.
        self.selection.set_hidden(look.selected.is_none());
        if let Some((range, past_end)) = &look.selected {
            let start = x_of(range.start);
            let mut end = x_of(range.end);
            if *past_end {
                end += columns.char_width;
            }
            self.selection.set_color(style.selection);
            self.selection.set_frame((start, 0.0, end - start, height));
        }

        self.caret.set_hidden(look.caret.is_none());
        self.caret.set_color(style.caret);
        if let Some(byte) = look.caret {
            self.caret
                .set_frame((x_of(byte) - CARET_WIDTH / 2.0, 1.0, CARET_WIDTH, height - 2.0));
        }

        self.message.set_hidden(look.mark.is_none());
        if let Some(message) = look.mark {
            code_label(self.message, style);
            self.message.set_text_color(style.mark).set_text(message);
            let x = line_end + columns.char_width * MESSAGE_GAP - inset;
            let width = self.message.content_size().width + inset;
            self.message.set_frame((x, 0.0, width, height));
        }
    }
}
