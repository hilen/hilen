use std::ops::Range;

use super::{
    decor::MarkKind,
    inlay::{Inlay, Shown},
    style::CodeEditorStyle,
};
use crate::{
    self as hilen,
    deps::refs::Weak,
    gm::color::CLEAR,
    ui::{Container, Label, Setup, TextAlignment, UIColor, View, ViewData, ViewFrame, ViewSubviews, view},
};

/// The width of the caret in points.
const CARET_WIDTH: f32 = 2.0;
/// The message of a mark starts this many chars after the code.
const MESSAGE_GAP: f32 = 3.0;
/// The line under a part of the code: how thick it is and how far over
/// the bottom of the row it lies.
const UNDERLINE_HEIGHT: f32 = 2.0;
const UNDERLINE_GAP: f32 = 1.0;

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
    pub mark:        Option<(&'a str, MarkKind)>,
    /// The lines under parts of the code, byte ranges with their colors.
    pub underlines:  &'a [(Range<usize>, UIColor)],
    /// The colors behind parts of the code, under the matches of the
    /// find bar.
    pub backgrounds: &'a [(Range<usize>, UIColor)],
    /// The texts inside the line that are no part of the file. Every
    /// byte of the other fields is one of `text`, with none of them.
    pub inlays:      &'a [Inlay],
    pub highlighted: bool,
    /// The bytes of the matches of the find bar on the line, each with
    /// whether it is the current match.
    pub matches:     Vec<(Range<usize>, bool)>,
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
/// lines under parts of it, the caret, the message of a mark and the
/// number in the gutter. The gutter is added after the code, so code
/// scrolled sideways goes under it.
#[view]
pub(super) struct CodeLine {
    /// The lines under parts of the code, as many as a row ever needed.
    bars: Vec<Weak<Container>>,

    #[init]
    selection:  Container,
    code:       Label,
    /// Holds the `bars`, so they stay under the gutter like the code.
    underlines: Container,
    caret:      Container,
    message:    Label,
    gutter:     Container,
    number:     Label,
}

impl Setup for CodeLine {
    fn setup(self: Weak<Self>) {
        let moving = [
            self.selection.weak_view(),
            self.code.weak_view(),
            self.underlines.weak_view(),
            self.caret.weak_view(),
            self.message.weak_view(),
        ];
        for view in moving {
            view.set_moves_sideways(true);
        }
        self.number.set_alignment(TextAlignment::Right);
        self.code.set_color(CLEAR);
        self.underlines.set_color(CLEAR);
        self.message.set_color(CLEAR);
        self.number.set_color(CLEAR);
    }
}

impl CodeLine {
    pub(super) fn show(self: Weak<Self>, look: &LineLook, columns: &Columns, style: &CodeEditorStyle) {
        let height = columns.row_height;

        let mark_colors = look.mark.map(|(_, kind)| style.mark_colors(kind));
        let background = match mark_colors {
            Some((_, line)) => line,
            None if look.highlighted => style.highlight,
            None => CLEAR.into(),
        };
        self.set_color(background);

        self.gutter.set_color(style.gutter);
        self.gutter.set_frame((0.0, 0.0, columns.gutter_width, height));

        code_label(self.number, style);
        self.number.set_cell_grid(false);
        self.number
            .set_text_color(mark_colors.map_or(style.line_number, |(text, _)| text))
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

        // The label draws the line with its inlays, so every byte of the
        // line goes through `shown` on its way to the label. A range of
        // the app can be from before an edit, `shown` puts a byte past the
        // line on its end.
        let shown = Shown::of(look.text, look.inlays);
        code_label(self.code, style);
        self.code.set_text_color(style.text).set_text(&*shown.text);
        let runs = look.runs.iter().map(|(range, color)| (shown.span(range), *color));
        // An inlay is painted last, over a run of the app that goes on
        // behind it.
        self.code
            .set_color_runs(runs.chain(shown.inlay_ranges().map(|range| (range, style.inlay))));
        let backgrounds = look.backgrounds.iter().map(|(range, color)| (shown.span(range), *color));
        let grounds = shown.inlay_ranges().map(|range| (range, style.inlay_ground));
        let matches = look.matches.iter().map(|(range, current)| {
            let color = if *current {
                style.find_current
            } else {
                style.find_match
            };
            (shown.span(range), color)
        });
        // A match of the find bar is painted last, over a color of the app.
        self.code.set_background_runs(backgrounds.chain(grounds).chain(matches));
        let inset = self.code.text_inset();
        self.code
            .set_frame((columns.text_x - inset, 0.0, columns.text_width + inset, height));

        let layout = self.code.text_layout_for(&shown.text);
        let shown_x = |byte: usize| columns.text_x + layout.x_on_line(0, byte);
        // Where the caret of a byte of the line stands.
        let x_of = |byte: usize| shown_x(shown.caret(byte));
        let line_end = shown_x(shown.text.len());
        let spans: Vec<(f32, f32, UIColor)> = look
            .underlines
            .iter()
            .map(|(range, color)| {
                let span = shown.span(range);
                (shown_x(span.start), shown_x(span.end), *color)
            })
            .collect();
        self.show_underlines(&spans, columns);

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
        if let (Some((message, _)), Some((color, _))) = (look.mark, mark_colors) {
            code_label(self.message, style);
            self.message.set_text_color(color).set_text(message);
            let x = line_end + columns.char_width * MESSAGE_GAP - inset;
            let width = self.message.content_size().width + inset;
            self.message.set_frame((x, 0.0, width, height));
        }
    }

    /// 1 thin view per underline, at the bottom of the row. Each one comes
    /// as the x of its start and of its end.
    fn show_underlines(mut self: Weak<Self>, underlines: &[(f32, f32, UIColor)], columns: &Columns) {
        let height = columns.row_height;
        self.underlines.set_hidden(underlines.is_empty());
        self.underlines
            .set_frame((0.0, 0.0, columns.text_x + columns.text_width, height));
        while self.bars.len() < underlines.len() {
            let bar = self.underlines.add_view::<Container>();
            self.bars.push(bar);
        }
        for (index, bar) in self.bars.iter().enumerate() {
            let Some((start, end, color)) = underlines.get(index) else {
                bar.set_hidden(true);
                continue;
            };
            // A range with no width, like the place of a missing `;` at the
            // end of a line, still shows as 1 char.
            let width = (end - start).max(columns.char_width);
            bar.set_hidden(false);
            bar.set_color(*color);
            bar.set_frame((
                *start,
                height - UNDERLINE_GAP - UNDERLINE_HEIGHT,
                width,
                UNDERLINE_HEIGHT,
            ));
        }
    }
}
