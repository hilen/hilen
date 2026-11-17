use super::{
    ansi::{Look, Marks, Styled, parse},
    data::LogLine,
    style::{LogStyle, dimmed},
};
use crate::{
    self as hilen,
    deps::refs::Weak,
    ui::{
        Label, RunStyle, Setup, TextAlignment, UIColor, VerticalAlignment, ViewData, ViewFrame,
        text_selection::Lead, view,
    },
};

/// Where the 2 texts of a row stand, the same for every row.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Columns {
    pub prefix_x:     f32,
    pub prefix_width: f32,
    pub text_x:       f32,
    pub text_width:   f32,
}

/// Puts a parsed text into `label` the way a row draws it. The label
/// that measures a line is filled by this too, so a measured height is
/// the height the row gets.
pub(super) fn fill(label: Weak<Label>, styled: &Styled, base: UIColor, style: &LogStyle) {
    label
        .set_font(style.regular_font())
        .set_text_size(style.text_size)
        .set_text_color(base);
    // A new text clears the runs of the text before it.
    label.set_text(styled.text.as_str());

    label.set_color_runs(styled.spans.iter().filter_map(|(range, look)| {
        let color = look.color.map(|color| style.color(color));
        let color = match (color, look.marks.has(Marks::DIM)) {
            (None, false) => return None,
            (color, false) => color.unwrap_or(base),
            (color, true) => dimmed(color.unwrap_or(base)),
        };
        Some((range.clone(), color))
    }));

    label.set_font_runs(
        styled
            .spans
            .iter()
            .filter_map(|(range, look)| Some((range.clone(), run_style(*look, style)?))),
    );
}

fn run_style(look: Look, style: &LogStyle) -> Option<RunStyle> {
    let underline = look.marks.has(Marks::UNDERLINE);
    let strike = look.marks.has(Marks::STRIKE);

    let mut run = if look.marks.has(Marks::BOLD) {
        RunStyle::font(style.strong_font())
    } else if underline {
        return Some(if strike {
            RunStyle::underline().struck()
        } else {
            RunStyle::underline()
        });
    } else if strike {
        return Some(RunStyle::strikethrough());
    } else {
        return None;
    };

    if underline {
        run = run.underlined();
    }
    if strike {
        run = run.struck();
    }
    Some(run)
}

/// 1 line of a log: the prefix in its column and the text beside it.
#[view]
pub(super) struct LogCell {
    #[init]
    prefix: Label,
    text:   Label,
}

impl Setup for LogCell {
    fn setup(mut self: Weak<Self>) {
        for label in [self.prefix, self.text] {
            label
                .set_alignment(TextAlignment::Left)
                .set_vertical_alignment(VerticalAlignment::Top)
                .set_selectable(true);
        }
        self.text.set_multiline(true);
        // A copy of a row is the prefix, a space and the text.
        self.text.selection_lead = Lead::Space;
    }
}

impl LogCell {
    pub(super) fn show(self: Weak<Self>, line: &LogLine, columns: &Columns, height: f32, style: &LogStyle) {
        let prefix = parse(&line.prefix);

        // A line with no prefix has 1 text, so a copy of it starts with
        // no space.
        self.prefix.set_hidden(prefix.text.is_empty());
        if !prefix.text.is_empty() {
            fill(self.prefix, &prefix, style.prefix, style);
            self.prefix.set_frame((columns.prefix_x, 0.0, columns.prefix_width, height));
        }

        fill(self.text, &parse(&line.text), style.text, style);
        self.text.set_frame((columns.text_x, 0.0, columns.text_width, height));
    }
}
