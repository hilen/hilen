//! The list of completions of a `CodeEditor`: the names an app offers for
//! the word the user types, in a panel under it. While it shows, Up and
//! Down move the marked row, Enter and Tab pick it, Escape closes it, and
//! every other key goes on to the editor, so the typing never stops.

use super::{line::code_label, style::CodeEditorStyle, text::CodePos, view::CodeEditor};
use crate::{
    self as hilen,
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{ToF32, color::CLEAR},
    ui::{
        CellRegistry, ImageView, Label, NamedKey, Setup, Shadow, TableData, TableView, TextAlignment, View,
        ViewData, ViewFrame, view,
    },
};

const ROW_HEIGHT: f32 = 24.0;
/// The list shows this many rows and scrolls for more.
const MOST_ROWS: usize = 8;
/// The list is as wide as its longest row needs, between these 2.
const LEAST_WIDTH: f32 = 420.0;
const MOST_WIDTH: f32 = 720.0;
const PADDING: f32 = 8.0;
const CORNER_RADIUS: f32 = 6.0;
const ICON_SIZE: f32 = 16.0;
/// Room between the icon and the name.
const ICON_GAP: f32 = 6.0;
/// The least room between the longest name and the details.
const DETAIL_GAP: f32 = 16.0;
/// The most of a row a name takes when the row does not fit the widest
/// list, the detail takes the rest and ends with dots.
const NAME_MOST: f32 = 0.6;

/// 1 row of the list of completions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Completion {
    /// The name that goes into the code.
    pub label:  String,
    /// What the name is, like its type, at the right in a dim color.
    pub detail: String,
    /// A black SVG of `assets/images` for the kind of the name, like a
    /// function or a field, in front of the name in the color of the
    /// detail. Empty for a row with no icon.
    pub icon:   String,
}

/// Where the parts of every row of 1 list go.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Columns {
    /// The width of the list.
    width: f32,
    /// A row has an icon, so every name starts after the room of one.
    icon:  bool,
    /// The width of the names.
    name:  f32,
    /// Where the text of a label starts inside its frame. The icon stands
    /// there, at the place the names of a list with no icon start.
    inset: f32,
}

/// The columns of a list whose longest name and longest detail need these
/// widths. Before, a list was 420 points wide with 55 percent for the
/// name, so a short name left its side empty and the detail was cut.
fn columns(icon: bool, name: f32, detail: f32) -> Columns {
    // Whole points first: a name rounded up after the width was worked out
    // took a part of a point from the detail, which then ended with dots.
    let (name, detail) = (name.ceil(), detail.ceil());
    let start = PADDING + if icon { ICON_SIZE + ICON_GAP } else { 0.0 };
    let width = (start + name + DETAIL_GAP + detail + PADDING).clamp(LEAST_WIDTH, MOST_WIDTH);
    let room = width - start - DETAIL_GAP - PADDING;
    let name = if name + detail <= room {
        name
    } else {
        name.min((room * NAME_MOST).floor())
    };
    Columns {
        width,
        icon,
        name,
        inset: 0.0,
    }
}

/// The list itself, the content of the panel. Its rows and its style are
/// set before it is added, it sizes itself in `setup`.
#[view]
pub(super) struct CompletionList {
    rows:     Vec<Completion>,
    selected: usize,
    /// The first row in view, so the arrows scroll only when the marked
    /// row leaves the view.
    top:      usize,
    style:    CodeEditorStyle,
    columns:  Columns,

    pub(super) picked: Event<usize>,

    #[init]
    table:   TableView,
    /// Measures a name or a detail the way a row draws it.
    measure: Label,
}

impl Setup for CompletionList {
    fn setup(mut self: Weak<Self>) {
        self.set_corner_radius(CORNER_RADIUS);
        self.set_border_width(1);
        self.set_shadow(Shadow::default());
        self.set_color(self.style.list);
        self.set_border_color(self.style.list_border);
        self.measure.set_hidden(true);
        row_label(self.measure, &self.style);
        self.columns = self.measured();
        self.set_size(self.columns.width, Self::height(self.rows.len()));
        self.table.set_data_source(self).register_cell::<CompletionCell>();
        self.table.place().back();
    }
}

/// The font of a name and a detail of a row.
fn row_label(label: Weak<Label>, style: &CodeEditorStyle) {
    code_label(label, style);
    label.set_cell_grid(false);
}

impl CompletionList {
    fn height(count: usize) -> f32 {
        count.min(MOST_ROWS).to_f32() * ROW_HEIGHT
    }

    /// The columns the rows of this list need.
    fn measured(self: Weak<Self>) -> Columns {
        let mut name = 0.0_f32;
        let mut detail = 0.0_f32;
        let mut icon = false;
        for row in &self.rows {
            name = name.max(self.width_of(&row.label));
            detail = detail.max(self.width_of(&row.detail));
            icon |= !row.icon.is_empty();
        }
        Columns {
            inset: self.measure.text_inset(),
            ..columns(icon, name, detail)
        }
    }

    fn width_of(self: Weak<Self>, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        self.measure.set_text(text);
        self.measure.content_size().width
    }

    /// Marks the row 1 up or down, and scrolls only when it left the view.
    fn step(mut self: Weak<Self>, down: bool) {
        let Some(last) = self.rows.len().checked_sub(1) else {
            return;
        };
        self.selected = if down {
            (self.selected + 1).min(last)
        } else {
            self.selected.saturating_sub(1)
        };
        let top = if self.selected < self.top {
            self.selected
        } else if self.selected >= self.top + MOST_ROWS {
            self.selected + 1 - MOST_ROWS
        } else {
            self.top
        };
        if top != self.top {
            self.top = top;
            self.table.scroll_to_row(top);
        }
        self.table.reload_data();
    }
}

impl TableData for CompletionList {
    fn cell_height(&self, _: usize) -> f32 {
        ROW_HEIGHT
    }

    fn number_of_cells(&self) -> usize {
        self.rows.len()
    }

    fn cell_selected(&mut self, index: usize) {
        weak_from_ref(self).picked.trigger(index);
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<CompletionCell>();
        if let Some(row) = self.rows.get(index) {
            cell.fill(row, index == self.selected, &self.style, self.columns);
        }
        cell
    }
}

/// 1 row: the icon of its kind, the name in the font of the code, and its
/// detail at the right.
#[view]
struct CompletionCell {
    #[init]
    icon:   ImageView,
    name:   Label,
    detail: Label,
}

impl Setup for CompletionCell {
    fn setup(self: Weak<Self>) {
        for label in [self.name, self.detail] {
            label.set_color(CLEAR);
            label.set_ellipsize(true);
        }
    }
}

impl CompletionCell {
    fn fill(self: Weak<Self>, row: &Completion, selected: bool, style: &CodeEditorStyle, columns: Columns) {
        let mut left = PADDING;
        self.icon.set_hidden(row.icon.is_empty());
        if columns.icon {
            if !row.icon.is_empty() {
                self.icon.set_icon(&row.icon, style.line_number);
            }
            self.icon.set_frame((
                left + columns.inset,
                (ROW_HEIGHT - ICON_SIZE) / 2.0,
                ICON_SIZE,
                ICON_SIZE,
            ));
            left += ICON_SIZE + ICON_GAP;
        }
        self.name.set_frame((left, 0.0, columns.name, ROW_HEIGHT));
        let detail = left + columns.name + DETAIL_GAP;
        let room = (columns.width - PADDING - detail).max(0.0);
        self.detail.set_frame((detail, 0.0, room, ROW_HEIGHT));
        for label in [self.name, self.detail] {
            row_label(label, style);
        }
        self.name.set_text_color(style.text).set_text(&row.label);
        self.detail
            .set_alignment(TextAlignment::Right)
            .set_text_color(style.line_number)
            .set_text(&row.detail);
        self.set_color(if selected {
            style.list_selected
        } else {
            CLEAR.into()
        });
    }
}

impl CodeEditor {
    /// Shows `items` as a list under the place `at`, like the start of
    /// the word the user types, with the first one marked. While it
    /// shows, Up and Down move the mark, Enter and Tab fire
    /// `completion_picked` with the index of the marked row, a press on a
    /// row does the same, and Escape closes the list. Each of these also
    /// closes the list. Every other key goes to the editor as always, and
    /// the app gives a new list after it or hides this one. An empty list
    /// hides it.
    pub fn show_completions(&self, at: CodePos, items: Vec<Completion>) {
        let mut this = weak_from_ref(self);
        if items.is_empty() {
            this.hide_completions();
            return;
        }
        let mut list = CompletionList::new();
        list.rows = items;
        list.style = this.style();
        let list_ref = list.weak();
        this.show_panel(at, list);
        list_ref.picked.val(move |index| this.pick_completion(index));
        this.completions = list_ref;
    }

    /// Closes the list of completions.
    pub fn hide_completions(&self) {
        let mut this = weak_from_ref(self);
        if this.completions.is_ok() {
            this.completions = Weak::default();
            this.hide_panel();
        }
    }

    pub fn completions_shown(&self) -> bool {
        self.completions.is_ok()
    }

    /// The index of the marked row, `None` while no list shows.
    pub fn marked_completion(&self) -> Option<usize> {
        self.completions.is_ok().then(|| self.completions.selected)
    }

    fn pick_completion(self: Weak<Self>, index: usize) {
        self.hide_completions();
        self.completion_picked.trigger(index);
    }

    /// A named key while the list shows. `true` when the list took it.
    pub(super) fn completion_key(self: Weak<Self>, key: NamedKey) -> bool {
        if !self.completions.is_ok() {
            return false;
        }
        match key {
            NamedKey::ArrowDown => self.completions.step(true),
            NamedKey::ArrowUp => self.completions.step(false),
            NamedKey::Enter | NamedKey::Tab => {
                let marked = self.completions.selected;
                self.pick_completion(marked);
            }
            NamedKey::Escape => self.hide_completions(),
            // The cursor leaves the word, the list is for another place.
            NamedKey::ArrowLeft
            | NamedKey::ArrowRight
            | NamedKey::Home
            | NamedKey::End
            | NamedKey::PageUp
            | NamedKey::PageDown => {
                self.hide_completions();
                return false;
            }
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_is_as_wide_as_its_longest_row_needs() {
        let short = columns(false, 60.0, 80.0);
        assert_eq!((short.width, short.name), (LEAST_WIDTH, 60.0));

        // The signatures of the methods of a number: both parts whole, also
        // when their widths are no whole points.
        let long = columns(true, 199.4, 329.6);
        let start = PADDING + ICON_SIZE + ICON_GAP;
        assert_eq!(long.width, start + 200.0 + DETAIL_GAP + 330.0 + PADDING);
        assert_eq!(long.name, 200.0);
        let detail_room = long.width - PADDING - (start + long.name + DETAIL_GAP);
        assert!(detail_room >= 329.6, "the detail has {detail_room} of 329.6");

        // A row longer than the widest list: the name gives way first.
        let most = columns(false, 500.0, 900.0);
        assert_eq!(most.width, MOST_WIDTH);
        assert_eq!(
            most.name,
            ((MOST_WIDTH - PADDING * 2.0 - DETAIL_GAP) * NAME_MOST).floor()
        );
    }
}
