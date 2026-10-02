use crate::{
    self as hilen,
    deps::{
        netrun::Function,
        refs::{Own, Weak, weak_from_ref},
    },
    gm::{
        ToF32, Toggle,
        color::{CLEAR, Color, LIGHT_BLUE, WHITE},
        flat::{LineCap, LineJoin, StrokeStyle, VectorPath},
    },
    ui::{
        Button, CellRegistry, Container, DrawingView, ImageView, Label, Setup, Shadow, TableData, TableView,
        TextAlignment, TouchStack, UIColor, UIImages, UIManager, View, ViewData, ViewFrame, ViewSubviews,
        ViewTouch, WeakView, struct_name, view,
    },
};

const ARROW: f32 = 16.0;
const INSET: f32 = 12.0;
const ROW_HEIGHT: f32 = 36.0;
const PANEL_PADDING: f32 = 4.0;
const PANEL_GAP: f32 = 6.0;
const ROW_RADIUS: f32 = 6.0;
const CHECK: f32 = 14.0;
/// The open list never comes closer than this to the edge of the window.
const SCREEN_MARGIN: f32 = 8.0;

/// What a `DropDown` shows. Like `TableData` for a table: the drop down
/// owns no values, it only knows the index of the picked row.
///
/// A plain list of texts needs none of this, `TextDropDown` is that. A
/// custom drop down is its own view, it holds a `DropDown`, owns the data
/// and implements this trait.
pub trait DropDownData {
    fn number_of_rows(&self) -> usize;

    fn row_height(&self, _: usize) -> f32 {
        ROW_HEIGHT
    }

    /// The text of a row that has no cell of its own.
    fn title(&self, _: usize) -> String {
        String::new()
    }

    /// A cell for this row out of the cell types given to
    /// `DropDown::register_cell`. `None` gives the default text row with
    /// `title`. A cell is reused for other rows, so set everything the row
    /// decides on every call. It draws all of itself, also how hovered and
    /// picked look, the drop down adds nothing around it.
    fn setup_cell(&mut self, _: usize, _: &mut CellRegistry) -> Option<WeakView> {
        None
    }

    /// The cell of the closed box for the picked row. The same as the row
    /// cell unless the box needs another look.
    fn setup_box_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Option<WeakView> {
        self.setup_cell(index, registry)
    }

    /// The user picked this row.
    fn row_selected(&mut self, index: usize);
}

/// A closed box with the picked row and a chevron. A tap opens a panel
/// under it, or above when there is more room, with the rows of its data
/// source in a table. The panel is as tall as its rows and scrolls only
/// when they do not fit the window. The box's own color, border and
/// corners are its look, the panel copies them.
#[view]
pub struct DropDown {
    data:   Weak<dyn DropDownData>,
    opened: bool,

    selected_index: usize,

    /// Applied to the collapsed label and to every default row.
    text_color: Option<UIColor>,
    text_size:  Option<f32>,
    accent:     Color,

    /// The border set by the app, put back when the box is idle again.
    idle_border: Option<Color>,
    raised:      bool,

    /// The view whose color, border and corners are the look of the box.
    /// The drop down itself unless a view around it draws the box.
    look: WeakView,

    box_registry: CellRegistry,
    box_cell:     WeakView,
    table:        Weak<TableView>,

    #[init]
    button: Button,
    holder: Container,
    label:  Label,
    arrow:  ImageView,
    panel:  Container,
}

impl DropDown {
    pub fn set_data_source(mut self: Weak<Self>, data: Weak<dyn DropDownData>) -> Weak<Self> {
        self.data = data;
        self.reload();
        self
    }

    /// A cell type the data source can ask its registry for.
    pub fn register_cell<T: View + Default + 'static>(mut self: Weak<Self>) -> Weak<Self> {
        self.table.register_cell::<T>();
        self.box_registry.constructors.insert(
            struct_name::<T>(),
            Function::new(|()| -> Own<dyn View> { T::new() }),
        );
        self
    }

    /// The rows changed. A picked index past the end falls back to the
    /// first row.
    pub fn reload(mut self: Weak<Self>) {
        if self.selected_index >= self.rows() {
            self.selected_index = 0;
        }
        self.refresh_box();
        if self.opened {
            self.layout_panel();
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Points the drop down at a row and updates the closed box. The list
    /// stays closed and `row_selected` is not called, so restoring a pick
    /// is never mistaken for a user pick. Returns false and changes
    /// nothing when there is no such row.
    pub fn select(mut self: Weak<Self>, index: usize) -> bool {
        if index >= self.rows() {
            return false;
        }
        self.selected_index = index;
        self.refresh_box();
        true
    }

    /// The text shown while the drop down is collapsed, empty when the
    /// box shows a cell of the data source.
    pub fn text(&self) -> &str {
        self.label.text()
    }

    pub fn is_opened(&self) -> bool {
        self.opened
    }

    /// Text color of the collapsed label and the default rows.
    pub fn set_text_color(&mut self, color: impl Into<UIColor>) -> &mut Self {
        let color = color.into();
        self.text_color = Some(color);
        self.label.set_text_color(color);
        self
    }

    pub fn set_text_size(&mut self, size: impl ToF32) -> &mut Self {
        let size = size.to_f32();
        self.text_size = Some(size);
        self.label.set_text_size(size);
        self
    }

    /// The color of the picked default row, its check mark, the hover
    /// wash and the border while hovered or open.
    pub fn set_accent_color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.accent = color.into();
        self
    }

    pub fn accent_color(&self) -> Color {
        self.accent
    }

    /// A view around the drop down draws the box, so its color, border
    /// and corners are the look the panel copies and the hover lights.
    pub fn set_look_source(&mut self, view: WeakView) -> &mut Self {
        self.look = view;
        self
    }

    fn rows(&self) -> usize {
        if self.data.is_null() {
            0
        } else {
            self.data.number_of_rows()
        }
    }

    fn look(&self) -> WeakView {
        if self.look.is_null() {
            self.weak_view()
        } else {
            self.look
        }
    }

    /// The closed box shows the picked row, as a cell of the data source
    /// or as its title.
    fn refresh_box(mut self: Weak<Self>) {
        if self.box_cell.is_ok() {
            self.box_cell.set_hidden(true);
            let old = self.box_cell;
            self.box_registry.load_old_cells(vec![old]);
        }
        self.box_cell = Weak::default();

        if self.rows() == 0 {
            self.label.set_hidden(false);
            self.label.set_text("");
            return;
        }

        let index = self.selected_index;
        let mut data = self.data;
        if let Some(cell) = data.setup_box_cell(index, &mut self.box_registry) {
            cell.place().clear().back();
            self.box_cell = cell;
            self.label.set_hidden(true);
            self.label.set_text("");
        } else {
            self.label.set_hidden(false);
            self.label.set_text(data.title(index));
        }
    }

    fn tapped(mut self: Weak<Self>) {
        if self.opened.flip() {
            self.close();
        } else {
            self.open();
        }
    }

    fn open(mut self: Weak<Self>) {
        // The panel floats over whatever sits under the box. Pushed
        // forward once, later siblings stay behind it.
        if !self.raised {
            self.bump_z_position(0.000_1);
            self.raised = true;
        }

        // Without a hover first, like on a phone, the border is still
        // the app's own here. Kept now, or the panel would copy the accent.
        let look = self.look();
        if self.idle_border.is_none() {
            self.idle_border = Some(*look.border_color());
        }

        look.set_border_color(self.accent);
        self.layout_panel();

        // Drawn in front is not enough, a touch goes to the view that
        // registered last. A sibling under the panel that turned its touch
        // on after this drop down would take the taps meant for the rows.
        TouchStack::raise_subtree(self.weak_view());
    }

    /// The panel is as tall as its rows, up to the room the window has
    /// on the better side of the box. Past that the table scrolls.
    fn layout_panel(mut self: Weak<Self>) {
        let look = self.look();
        let rows = self.rows();
        let data = self.data;
        let wanted: f32 = (0..rows).map(|index| data.row_height(index)).sum::<f32>() + 2.0 * PANEL_PADDING;

        let frame = *self.absolute_frame();
        let window = UIManager::root_view().height();
        let room_below = window - frame.max_y() - PANEL_GAP - SCREEN_MARGIN;
        let room_above = frame.y() - PANEL_GAP - SCREEN_MARGIN;

        let below = wanted <= room_below || room_below >= room_above;
        let room = if below { room_below } else { room_above };
        let height = wanted.min(room.max(ROW_HEIGHT + 2.0 * PANEL_PADDING));
        let y = if below {
            self.height() + PANEL_GAP
        } else {
            -(height + PANEL_GAP)
        };

        self.panel
            .set_color(*look.color())
            .set_corner_radii(look.corner_radii())
            .set_border_width(look.border_width())
            .set_border_color(self.idle_border.unwrap_or(*look.border_color()))
            .set_shadow(Shadow::default());
        self.panel.set_frame((0.0, y, self.width(), height));
        self.panel.set_hidden(false);
        self.table.reload_data();
    }

    fn close(mut self: Weak<Self>) {
        self.opened = false;
        self.panel.set_hidden(true);
        if let Some(border) = self.idle_border {
            self.look().set_border_color(border);
        }
    }

    fn pick(mut self: Weak<Self>, index: usize) {
        self.selected_index = index;
        self.refresh_box();
        self.close();
        let mut data = self.data;
        data.row_selected(index);
    }
}

impl TableData for DropDown {
    fn cell_height(&self, index: usize) -> f32 {
        self.data.row_height(index)
    }

    fn number_of_cells(&self) -> usize {
        self.rows()
    }

    fn cell_selected(&mut self, index: usize) {
        weak_from_ref(self).pick(index);
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> WeakView {
        let mut data = self.data;
        if let Some(cell) = data.setup_cell(index, registry) {
            return cell;
        }

        let row = registry.cell::<DropDownRow>();
        row.setup_row(
            data.title(index),
            index == self.selected_index,
            self.accent,
            self.text_color,
            self.text_size,
        );
        row
    }
}

impl Setup for DropDown {
    fn setup(mut self: Weak<Self>) {
        self.accent = LIGHT_BLUE;
        self.set_color(WHITE);

        self.button.set_color(CLEAR).place().back();
        self.button.on_tap(move || self.tapped());

        self.holder.set_color(CLEAR);
        self.holder.place().l(0).r(INSET + ARROW).tb(0);
        let holder = self.holder.weak_view();
        self.box_registry.set_parent(holder);

        self.label.set_color(CLEAR).set_alignment(TextAlignment::Left);
        self.label.place().l(INSET).r(INSET + ARROW + 6.0).tb(0);

        self.arrow.set_image(UIImages::chevron_down());
        self.arrow.place().r(INSET - 2.0).center_y().size(ARROW, ARROW);

        self.panel.set_hidden(true);
        self.table = self.panel.add_view::<TableView>();
        self.table.set_color(CLEAR);
        self.table.place().all_sides(PANEL_PADDING);
        self.table
            .set_data_source(self)
            .register_cell::<DropDownRow>()
            .set_variable_heights(true);

        // The border the app set is what idle looks like. Read on the
        // first hover or open, after the app's setup has run.
        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| {
            let look = self.look();
            if self.idle_border.is_none() {
                self.idle_border = Some(*look.border_color());
            }
            if hovered {
                look.set_border_color(self.accent);
            } else if !self.opened
                && let Some(border) = self.idle_border
            {
                look.set_border_color(border);
            }
        });
    }
}

/// The default row of the open panel: a text, and a check mark on the
/// picked one.
#[view]
struct DropDownRow {
    accent:   Color,
    selected: bool,

    #[init]
    label: Label,
    check: DrawingView,
}

impl DropDownRow {
    fn setup_row(
        mut self: Weak<Self>,
        text: String,
        selected: bool,
        accent: Color,
        text_color: Option<UIColor>,
        text_size: Option<f32>,
    ) {
        self.accent = accent;
        self.selected = selected;
        self.label.set_text(text);
        if let Some(color) = text_color {
            self.label.set_text_color(color);
        }
        if let Some(size) = text_size {
            self.label.set_text_size(size);
        }
        // A reused row may have been the picked one before.
        self.check.remove_all_paths();
        if selected {
            self.label.set_text_color(accent);
            let path = VectorPath::polyline([(1.5, 7.5), (5.5, 11.5), (12.5, 3.0)]);
            self.check.add_stroke(
                &path,
                accent,
                StrokeStyle::width(2.2).cap(LineCap::Round).join(LineJoin::Round),
            );
        }
        self.check.set_hidden(!selected);
        self.refresh(false);
    }

    fn refresh(self: Weak<Self>, hovered: bool) {
        if hovered {
            self.set_color(self.accent.with_alpha(0.14));
        } else if self.selected {
            self.set_color(self.accent.with_alpha(0.08));
        } else {
            self.set_color(CLEAR);
        }
    }
}

impl Setup for DropDownRow {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(ROW_RADIUS);

        self.label.set_color(CLEAR).set_alignment(TextAlignment::Left);
        self.label.place().l(INSET - PANEL_PADDING).r(INSET + CHECK).tb(0);

        self.check.set_color(CLEAR);
        self.check.place().r(INSET - PANEL_PADDING).center_y().size(CHECK, CHECK);

        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| self.refresh(hovered));
    }
}
