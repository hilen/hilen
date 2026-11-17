use super::look::{
    ACCENT, DIM_TEXT, FAINT_TEXT, FOLDER, HOVER, Icon, ON_ACCENT, RADIUS, SMALL_TEXT_SIZE, TEXT, TEXT_SIZE,
};
use crate::{
    self as hilen,
    deps::refs::Weak,
    filesystem::{FileEntry, date_text, size_text},
    gm::color::CLEAR,
    ui::{
        DynamicColor, ImageView, Label, Setup, TextAlignment, UIColor, ViewCallbacks, ViewData, ViewTouch,
        view,
    },
};

pub(super) const LIST_ROW_HEIGHT: f32 = 30.0;
pub(super) const GRID_CELL_WIDTH: f32 = 104.0;
pub(super) const GRID_CELL_HEIGHT: f32 = 92.0;

const LIST_ICON: f32 = 16.0;
const GRID_ICON: f32 = 40.0;
const PAD: f32 = 8.0;
const COLUMN_GAP: f32 = 12.0;
const SIZE_WIDTH: f32 = 72.0;
const KIND_WIDTH: f32 = 104.0;
const DATE_WIDTH: f32 = 128.0;

/// Where the columns of the list sit inside a row of one width, as the
/// left edge and the width of each. A narrow row drops the date first,
/// then the kind, then the size, the name is always there.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Columns {
    pub name: (f32, f32),
    pub size: Option<(f32, f32)>,
    pub kind: Option<(f32, f32)>,
    pub date: Option<(f32, f32)>,
}

impl Columns {
    pub(super) fn for_width(width: f32) -> Self {
        // A label keeps its own inset before the text.
        let name_x = PAD + LIST_ICON + 2.0;
        let mut right = width - PAD;

        let mut take = |column_width: f32, min_row: f32| {
            (width >= min_row).then(|| {
                right -= column_width;
                let column = (right, column_width);
                right -= COLUMN_GAP;
                column
            })
        };

        let date = take(DATE_WIDTH, 500.0);
        let kind = take(KIND_WIDTH, 380.0);
        let size = take(SIZE_WIDTH, 260.0);

        Self {
            name: (name_x, (right - name_x).max(0.0)),
            size,
            kind,
            date,
        }
    }
}

/// What a cell shows of one entry.
#[derive(Clone, Debug, Default)]
pub(super) struct Row {
    pub entry:    FileEntry,
    pub selected: bool,
    /// A file while only a folder can be picked is shown faint and takes
    /// no selection.
    pub enabled:  bool,
}

impl Row {
    fn icon(&self) -> Icon {
        Icon::of_category(self.entry.category())
    }

    fn icon_tint(&self) -> DynamicColor {
        if self.selected {
            ON_ACCENT
        } else if !self.enabled {
            FAINT_TEXT
        } else if self.entry.is_folder() {
            FOLDER
        } else {
            DIM_TEXT
        }
    }

    fn text_color(&self, main: bool) -> DynamicColor {
        if self.selected {
            ON_ACCENT
        } else if !self.enabled {
            FAINT_TEXT
        } else if main {
            TEXT
        } else {
            DIM_TEXT
        }
    }

    fn background(&self, hovered: bool) -> UIColor {
        if self.selected {
            ACCENT.into()
        } else if hovered && self.enabled {
            HOVER.into()
        } else {
            CLEAR.into()
        }
    }
}

/// A row of the list: icon, name, size, kind and date.
#[view]
pub(super) struct ListCell {
    row:     Row,
    columns: Columns,

    #[init]
    icon: ImageView,
    name: Label,
    size: Label,
    kind: Label,
    date: Label,
}

impl ListCell {
    pub(super) fn fill(mut self: Weak<Self>, row: &Row, columns: Columns) {
        self.row = row.clone();

        self.name.set_text(&row.entry.name);
        self.size.set_text(row.entry.size.map_or_default(size_text));
        self.kind.set_text(row.entry.kind_text());
        self.date.set_text(row.entry.modified.map_or_default(date_text));

        if self.columns != columns {
            self.columns = columns;
            self.layout();
        }
        self.refresh();
    }

    fn layout(self: Weak<Self>) {
        let columns = self.columns;
        let place = |label: Weak<Label>, column: Option<(f32, f32)>| {
            label.set_hidden(column.is_none());
            if let Some((x, width)) = column {
                label.place().clear().tb(0).l(x).w(width);
            }
        };
        place(self.name, Some(columns.name));
        place(self.size, columns.size);
        place(self.kind, columns.kind);
        place(self.date, columns.date);
    }

    fn refresh(&self) {
        self.set_color(self.row.background(self.is_hovered()));
        self.icon.set_image(self.row.icon().image(self.row.icon_tint().resolve()));
        self.name.set_text_color(self.row.text_color(true));
        for label in [self.size, self.kind, self.date] {
            label.set_text_color(self.row.text_color(false));
        }
    }
}

impl ViewCallbacks for ListCell {
    fn theme_changed(&mut self) {
        self.refresh();
    }
}

impl Setup for ListCell {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(RADIUS - 1.0);

        self.icon.place().l(PAD).center_y().size(LIST_ICON, LIST_ICON);

        for label in [self.name, self.size, self.kind, self.date] {
            label
                .set_color(CLEAR)
                .set_text_size(TEXT_SIZE)
                .set_alignment(TextAlignment::Left)
                .set_ellipsize(true);
        }
        self.size.set_alignment(TextAlignment::Right);

        self.enable_hover();
        self.touch().hovered.sub(self, move || self.refresh());
    }
}

/// A cell of the icon grid: a big icon over the name.
#[view]
pub(super) struct GridCell {
    row: Row,

    #[init]
    icon: ImageView,
    name: Label,
}

impl GridCell {
    pub(super) fn fill(mut self: Weak<Self>, row: &Row) {
        self.row = row.clone();
        self.name.set_text(&row.entry.name);
        self.refresh();
    }

    fn refresh(&self) {
        self.set_color(self.row.background(self.is_hovered()));
        self.icon.set_image(self.row.icon().image(self.row.icon_tint().resolve()));
        self.name.set_text_color(self.row.text_color(true));
    }
}

impl ViewCallbacks for GridCell {
    fn theme_changed(&mut self) {
        self.refresh();
    }
}

impl Setup for GridCell {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(RADIUS + 2.0);

        self.icon.place().t(10).center_x().size(GRID_ICON, GRID_ICON);

        self.name
            .set_color(CLEAR)
            .set_text_size(SMALL_TEXT_SIZE)
            .set_alignment(TextAlignment::Center)
            .set_ellipsize(true);
        self.name.place().lr(4).b(8).h(22);

        self.enable_hover();
        self.touch().hovered.sub(self, move || self.refresh());
    }
}

#[cfg(test)]
mod test {
    use super::Columns;

    #[test]
    fn a_wide_row_has_every_column_and_none_overlap() {
        let columns = Columns::for_width(700.0);
        let (name_x, name_width) = columns.name;
        let size = columns.size.unwrap();
        let kind = columns.kind.unwrap();
        let date = columns.date.unwrap();

        assert!(name_x + name_width <= size.0);
        assert!(size.0 + size.1 <= kind.0);
        assert!(kind.0 + kind.1 <= date.0);
        assert!(date.0 + date.1 <= 700.0);
    }

    #[test]
    fn a_narrow_row_drops_the_date_then_the_kind_then_the_size() {
        assert!(Columns::for_width(480.0).date.is_none());
        assert!(Columns::for_width(480.0).kind.is_some());
        assert!(Columns::for_width(360.0).kind.is_none());
        assert!(Columns::for_width(360.0).size.is_some());
        let phone = Columns::for_width(240.0);
        assert!(phone.size.is_none());
        assert!(phone.name.1 > 150.0);
    }
}
