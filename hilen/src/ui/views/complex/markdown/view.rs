use std::mem::take;

use log::{info, warn};
use ui_proc::view;

use crate::{
    deps::{hreads::on_main, refs::Weak, vents::Event},
    gm::{
        LossyConvert,
        color::WHITE,
        flat::{CornerRadii, LineCap, LineJoin, Point, StrokeStyle, VectorPath},
    },
    system::open_url,
    ui::{
        CodeHighlighter, Container, DrawingView, Label, MarkdownStyle, RunStyle, Setup, TextAlignment,
        TextSelection, UIColor, VerticalAlignment, ViewFrame, ViewSubviews,
        view::{ViewData, ViewTouch},
        views::complex::markdown::{
            model::{Block, Item, Link, Marker, Styled},
            parse::parse,
            selection::marker_text,
        },
    },
};

const CODE_PAD: f32 = 10.0;
const QUOTE_INDENT: f32 = 14.0;
const QUOTE_BAR: f32 = 3.0;
const MARKER_GAP: f32 = 6.0;
const TASK_BOX: f32 = 14.0;
const CELL_PAD_X: f32 = 10.0;
const CELL_PAD_Y: f32 = 6.0;
const MIN_COLUMN: f32 = 48.0;
/// No text is laid out narrower, a deep list in a small window still
/// shows words.
const MIN_WIDTH: f32 = 40.0;
/// A bound no line reaches, to ask a label for the width of its text.
const UNBOUND: f32 = 100_000.0;

/// A markdown text drawn as blocks, one under the other: paragraphs,
/// headings, lists with tasks, quotes, code with colors, tables and
/// rules. Inside a text: bold, italic, struck, code and links. A tap on
/// a link fires `link_tapped` and opens the address, `set_opens_links`
/// turns the opening off for an app that handles its links itself.
///
/// The view does not size itself. `height_for_width` says how high the
/// text is at a width, the owner gives it that room, the way a table
/// asks for the height of a cell. A view that got its text already lays
/// it out again when its width changes.
#[view]
pub struct MarkdownView {
    /// A link was tapped, the value is its address.
    pub link_tapped: Event<String>,
    /// A tap on a link only fires the event.
    keeps_links:     bool,

    pub(super) blocks: Vec<Block>,
    /// The label of every text in reading order, made by the last layout.
    /// Kept only while the text can be selected, see `set_selectable`,
    /// none otherwise.
    pub(super) texts:  Option<Vec<Weak<Label>>>,

    text_color: Option<UIColor>,
    /// The text or its color changed since the last layout.
    stale:      bool,
    /// A layout is already asked for on the next turn of the main loop.
    scheduled:  bool,
    laid_width: f32,
    height:     f32,
}

impl Setup for MarkdownView {
    fn setup(self: Weak<Self>) {
        // The event fires inside the layout pass, the views are made after it.
        self.size_changed().sub(move || {
            on_main(move || {
                if self.is_ok() {
                    self.lay_out_if_needed(self.width());
                }
            });
        });
    }
}

/// Where the next block goes and how wide it may be.
#[derive(Clone, Copy)]
struct Column {
    x:      f32,
    width:  f32,
    /// The space between 2 blocks here.
    gap:    f32,
    color:  UIColor,
    /// The empty strip a label keeps at the side its text starts from.
    /// Everything that is not a text is moved in by it, so a code block
    /// or a table starts where the text above it starts.
    margin: f32,
}

impl MarkdownView {
    /// GitHub flavored markdown, with tables, task lists and strikethrough.
    pub fn set_text(mut self: Weak<Self>, text: &str) -> Weak<Self> {
        self.blocks = parse(text);
        self.changed();
        self
    }

    /// A text shown as it is, one paragraph with no marks read.
    pub fn set_plain_text(mut self: Weak<Self>, text: &str) -> Weak<Self> {
        self.blocks = vec![Block::Paragraph(Styled::plain(text))];
        self.changed();
        self
    }

    /// Whether a tap on a link opens it with `open_url`. On by default.
    pub fn set_opens_links(mut self: Weak<Self>, opens: bool) -> Weak<Self> {
        self.keeps_links = !opens;
        self
    }

    /// The color of the plain text, the one of `MarkdownStyle` when not set.
    pub fn set_text_color(mut self: Weak<Self>, color: impl Into<UIColor>) -> Weak<Self> {
        self.text_color = Some(color.into());
        self.changed();
        self
    }

    /// Lays the text out at a width and gives the height it takes.
    pub fn height_for_width(self: Weak<Self>, width: f32) -> f32 {
        self.lay_out_if_needed(width);
        self.height
    }

    /// A text and its color mostly change together, so the layout waits
    /// for the next turn of the main loop and runs once for all of them.
    /// `height_for_width` does not wait.
    pub(super) fn changed(mut self: Weak<Self>) {
        self.stale = true;
        if self.scheduled {
            return;
        }
        self.scheduled = true;
        on_main(move || {
            if self.is_ok() {
                self.scheduled = false;
                self.lay_out_if_needed(self.width());
            }
        });
    }

    fn lay_out_if_needed(mut self: Weak<Self>, width: f32) {
        // A view with no frame yet has nothing to lay out against.
        if width <= 0.0 || (!self.stale && (self.laid_width - width).abs() < 0.5) {
            return;
        }
        self.stale = false;
        self.laid_width = width;

        self.remove_all_subviews();
        if let Some(texts) = &mut self.texts {
            texts.clear();
        }
        let style = MarkdownStyle::current();
        let mut probe = self.add_view::<Label>();
        probe.set_alignment(TextAlignment::Left);
        let margin = probe.text_inset();
        probe.remove_from_superview();

        let column = Column {
            x: 0.0,
            width,
            gap: style.block_gap,
            color: self.text_color.unwrap_or(style.text),
            margin,
        };
        let mut y = 0.0;
        // The list is taken out for the walk, the views are made on `self`.
        let blocks = take(&mut self.blocks);
        self.blocks(&style, &blocks, column, &mut y);
        self.blocks = blocks;
        self.height = y;
    }

    fn blocks(self: Weak<Self>, style: &MarkdownStyle, blocks: &[Block], column: Column, y: &mut f32) {
        for (index, block) in blocks.iter().enumerate() {
            if index > 0 {
                *y += column.gap;
            }
            self.block(style, block, column, y);
        }
    }

    fn block(self: Weak<Self>, style: &MarkdownStyle, block: &Block, column: Column, y: &mut f32) {
        match block {
            Block::Paragraph(text) => self.text(style, text, style.text_size, false, column, y),
            Block::Heading { level, text } => {
                self.text(style, text, style.heading_size(*level), true, column, y);
            }
            Block::Code { language, text } => self.code(style, language, text, column, y),
            Block::Quote(blocks) => self.quote(style, blocks, column, y),
            Block::List(items) => self.list(style, items, column, y),
            Block::Table { head, rows } => self.table(style, head, rows, column, y),
            Block::Rule => {
                self.line(
                    style,
                    column.x + column.margin,
                    *y + 4.0,
                    column.width - column.margin * 2.0,
                    1.0,
                );
                *y += 9.0;
            }
        }
    }

    fn text(
        self: Weak<Self>,
        style: &MarkdownStyle,
        text: &Styled,
        size: f32,
        heading: bool,
        column: Column,
        y: &mut f32,
    ) {
        let width = column.width.max(MIN_WIDTH);
        let label = self.label(style, text, size, heading, column.color);
        let height = label.size_for_width(width).height;
        label.set_frame((column.x, *y, width, height));
        *y += height;
    }

    /// A text with its bold, italic, struck, code and link ranges drawn
    /// as such.
    fn label(
        self: Weak<Self>,
        style: &MarkdownStyle,
        text: &Styled,
        size: f32,
        heading: bool,
        color: UIColor,
    ) -> Weak<Label> {
        let fonts = style.fonts();
        let label = self.add_view::<Label>();
        label
            .set_multiline(true)
            .set_text_size(size)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        if let Some(line_height) = style.line_height_for(size) {
            label.set_line_height(line_height);
        }
        label.set_font(if heading { fonts.bold } else { fonts.regular });
        label.set_text(&text.text);
        label.set_text_color(color);
        label.set_font_runs(text.spans.iter().filter_map(|span| {
            let run = span.style;
            let bold = heading || run.face.is_bold();
            let font = match (run.code, bold, run.face.is_italic()) {
                (true, true, _) => Some(fonts.mono_bold),
                (true, false, _) => Some(fonts.mono),
                (false, true, true) => Some(fonts.bold_italic),
                (false, false, true) => Some(fonts.italic),
                // The font of a heading is bold already.
                (false, true, false) => (!heading).then_some(fonts.bold),
                (false, false, false) => None,
            };
            (font.is_some() || run.link || run.strike).then(|| {
                (
                    span.range.clone(),
                    RunStyle {
                        font,
                        underline: run.link,
                        strikethrough: run.strike,
                    },
                )
            })
        }));
        label.set_color_runs(text.spans.iter().filter_map(|span| {
            let color = if span.style.link {
                style.link
            } else if span.style.code {
                style.inline_code
            } else {
                return None;
            };
            Some((span.range.clone(), color))
        }));
        if !text.links.is_empty() {
            let links = text.links.clone();
            label.enable_touch();
            label.touch().up_inside.val(label, move |touch| {
                // The release of a drag that selected text is no click.
                if self.is_selectable() && !TextSelection::is_empty() {
                    return;
                }
                let Some(url) = link_at(label, &links, touch.position) else {
                    return;
                };
                self.link_tapped.trigger(url.to_string());
                if self.keeps_links {
                    return;
                }
                info!("opening the link {url}");
                if let Err(err) = open_url(url) {
                    warn!("the link {url} did not open: {err:#}");
                }
            });
        }
        self.note_text(label);
        label
    }

    fn code(
        self: Weak<Self>,
        style: &MarkdownStyle,
        language: &str,
        code: &str,
        column: Column,
        y: &mut f32,
    ) {
        let x = column.x + column.margin;
        let width = (column.width - column.margin * 2.0).max(MIN_WIDTH);
        // The background first, a later view draws over an earlier one.
        let background = self.add_view::<Container>();
        background.set_color(style.code_background).set_corner_radius(8);

        let label = self.add_view::<Label>();
        label
            .set_multiline(true)
            .set_text_size(style.code_size)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_font(style.fonts().mono);
        label.set_text(code);
        label.set_text_color(column.color);
        if let Some(mut highlighter) = CodeHighlighter::for_language(language) {
            label.set_color_runs(highlighter.color_runs(code));
        }

        let inner = width - CODE_PAD * 2.0;
        let height = label.size_for_width(inner).height;
        background.set_frame((x, *y, width, height + CODE_PAD * 2.0));
        label.set_frame((x + CODE_PAD, *y + CODE_PAD, inner, height));
        self.note_text(label);
        *y += height + CODE_PAD * 2.0;
    }

    fn quote(self: Weak<Self>, style: &MarkdownStyle, blocks: &[Block], column: Column, y: &mut f32) {
        let top = *y;
        let inner = Column {
            x:      column.x + QUOTE_INDENT,
            width:  column.width - QUOTE_INDENT,
            gap:    style.block_gap,
            color:  style.dim_text,
            margin: column.margin,
        };
        self.blocks(style, blocks, inner, y);
        let bar = self.add_view::<Container>();
        bar.set_color(style.line).set_corner_radius(QUOTE_BAR / 2.0);
        bar.set_frame((column.x + column.margin, top, QUOTE_BAR, *y - top));
    }

    fn list(self: Weak<Self>, style: &MarkdownStyle, items: &[Item], column: Column, y: &mut f32) {
        // One line of body text, the height of a marker.
        let line_height = style.body_line();
        // Every marker first, the widest one sets where the texts start.
        let markers: Vec<Option<Weak<Label>>> = items
            .iter()
            .map(|item| {
                let text = marker_text(item.marker)?;
                let label = self.add_view::<Label>();
                label
                    .set_text_size(style.text_size)
                    .set_alignment(TextAlignment::Right)
                    .set_vertical_alignment(VerticalAlignment::Top);
                // The same line box as the text of the item, so the marker
                // stays on the baseline of its first line.
                if let Some(line_height) = style.line_height {
                    label.set_line_height(line_height);
                }
                label.set_font(style.fonts().regular);
                label.set_text(text);
                label.set_text_color(style.dim_text);
                Some(label)
            })
            .collect();
        // A marker starts where a text of this column starts, and the text
        // of an item starts `MARKER_GAP` after the widest marker. The
        // measured width of a label has 1 margin in it, the frame gets 2,
        // so a right aligned marker keeps the left one free.
        let margin = column.margin;
        let marker_width = markers
            .iter()
            .flatten()
            .map(|label| label.size_for_width(UNBOUND).width.ceil() + margin)
            .fold(TASK_BOX + margin * 2.0, f32::max);
        let indent = marker_width - margin * 2.0 + MARKER_GAP;

        let inner = Column {
            x: column.x + indent,
            width: column.width - indent,
            gap: style.item_gap,
            color: column.color,
            margin,
        };
        for (index, (item, marker)) in items.iter().zip(markers).enumerate() {
            if index > 0 {
                *y += style.item_gap;
            }
            let top = *y;
            if let Some(marker) = marker {
                marker.set_frame((column.x, top, marker_width, line_height));
                self.note_text(marker);
            }
            if let Marker::Task(done) = item.marker {
                // In the middle of the first line when the style sets the
                // pitch.
                let drop = style.line_height.map_or(1.0, |height| ((height - TASK_BOX) / 2.0).round());
                self.task_box(style, done, column.x + margin, top + drop);
            }
            self.blocks(style, &item.blocks, inner, y);
            // An item with no text still takes its line.
            *y = y.max(top + line_height);
        }
    }

    fn task_box(self: Weak<Self>, style: &MarkdownStyle, done: bool, x: f32, y: f32) {
        let square = self.add_view::<Container>();
        square.set_corner_radius(3);
        square.set_frame((x, y, TASK_BOX, TASK_BOX));
        if !done {
            square.set_border_color(style.dim_text).set_border_width(1.5);
            return;
        }
        square.set_color(style.link);
        let mut check = self.add_view::<DrawingView>();
        check.set_frame((x, y, TASK_BOX, TASK_BOX));
        // White in both themes, the square under it has a strong color in both.
        check.add_stroke(
            &VectorPath::polyline([(3.2, 7.4), (6.0, 10.0), (10.8, 4.4)]),
            WHITE,
            StrokeStyle {
                width: 1.6,
                cap: LineCap::Round,
                join: LineJoin::Round,
                ..StrokeStyle::default()
            },
        );
    }

    fn table(
        self: Weak<Self>,
        style: &MarkdownStyle,
        head: &[Styled],
        rows: &[Vec<Styled>],
        column: Column,
        y: &mut f32,
    ) {
        let columns = rows.iter().map(Vec::len).fold(head.len(), usize::max);
        if columns == 0 {
            return;
        }
        // The frame and the head background first, the texts draw over them.
        let frame = self.add_view::<Container>();
        frame.set_border_color(style.line).set_border_width(1).set_corner_radius(6);
        let head_background = self.add_view::<Container>();
        head_background
            .set_color(style.table_head)
            .set_corner_radii(CornerRadii::top(5));

        let mut cells: Vec<Vec<Weak<Label>>> = Vec::with_capacity(rows.len() + 1);
        cells.push(
            head.iter()
                .map(|text| self.label(style, text, style.text_size, true, column.color))
                .collect(),
        );
        for row in rows {
            cells.push(
                row.iter()
                    .map(|text| self.label(style, text, style.text_size, false, column.color))
                    .collect(),
            );
        }

        let mut widths = vec![MIN_COLUMN; columns];
        for row in &cells {
            for (width, label) in widths.iter_mut().zip(row) {
                let natural = label.size_for_width(UNBOUND).width.ceil() + CELL_PAD_X * 2.0;
                *width = width.max(natural);
            }
        }
        // A table wider than its room gives every column the same share
        // less, the texts wrap inside their cells.
        let total: f32 = widths.iter().sum();
        let left = column.x + column.margin;
        let room = (column.width - column.margin * 2.0).max(MIN_WIDTH);
        if total > room {
            for width in &mut widths {
                *width *= room / total;
            }
        }
        let total = total.min(room);
        let min_row = style.body_line();

        let top = *y;
        for (index, row) in cells.iter().enumerate() {
            let height = row
                .iter()
                .zip(&widths)
                .map(|(label, width)| label.size_for_width(width - CELL_PAD_X * 2.0).height)
                .fold(min_row, f32::max)
                + CELL_PAD_Y * 2.0;
            if index == 0 {
                head_background.set_frame((left + 1.0, top + 1.0, total - 2.0, height - 1.0));
            } else {
                self.line(style, left, *y, total, 1.0);
            }
            let mut x = left;
            for (label, width) in row.iter().zip(&widths) {
                label.set_frame((
                    x + CELL_PAD_X,
                    *y + CELL_PAD_Y,
                    width - CELL_PAD_X * 2.0,
                    height - CELL_PAD_Y * 2.0,
                ));
                x += width;
            }
            *y += height;
        }
        let mut x = left;
        for width in &widths[..columns - 1] {
            x += width;
            self.line(style, x, top, 1.0, *y - top);
        }
        frame.set_frame((left, top, total, *y - top));
    }

    fn line(self: Weak<Self>, style: &MarkdownStyle, x: f32, y: f32, width: f32, height: f32) {
        let line = self.add_view::<Container>();
        line.set_color(style.line);
        line.set_frame((x, y, width, height));
    }
}

/// The address of the link drawn at a point of a label, in the points
/// of the label. A link that wraps is hit on every line it has a piece on.
fn link_at(label: Weak<Label>, links: &[Link], point: Point) -> Option<&str> {
    let layout = label.text_layout_for(label.text());
    let count: f32 = layout.line_count().lossy_convert();
    // The label draws its text from its top edge.
    if point.y < 0.0 || point.y >= count * layout.line_height {
        return None;
    }
    let line = layout.line_at_y(point.y);
    let on_line = layout.line_range(line);
    let x = point.x - label.text_inset();
    links
        .iter()
        .find(|link| {
            let start = link.range.start.max(on_line.start);
            let end = link.range.end.min(on_line.end);
            start < end && x >= layout.x_on_line(line, start) && x <= layout.x_on_line(line, end)
        })
        .map(|link| link.url.as_str())
}
