use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::{
        LossyConvert, ToF32, Toggle,
        color::{CLEAR, Color, LIGHT_BLUE, WHITE},
        flat::{LineCap, LineJoin, StrokeStyle, VectorPath},
    },
    ui::{
        Button, Container, DrawingView, ImageView, Label, Setup, Shadow, TextAlignment, ToLabel, UIColor,
        UIEvent, UIImages, ViewData, ViewFrame, ViewSubviews, ViewTouch, view,
    },
};

const ARROW: f32 = 16.0;
const INSET: f32 = 12.0;
const ROW_HEIGHT: f32 = 36.0;
const PANEL_PADDING: f32 = 4.0;
const PANEL_GAP: f32 = 6.0;
const ROW_RADIUS: f32 = 6.0;
const CHECK: f32 = 14.0;

/// A closed box with the picked value and a chevron. A tap opens a
/// panel under it, or above when there is no room, with one row per
/// value. Rows light up on hover, the picked one shows in the accent
/// color with a check mark. The box's own color, border and corners are
/// its look, the panel copies them.
#[view]
pub struct DropDown<T: 'static> {
    values:  Vec<T>,
    opened:  bool,
    changed: Event<T>,

    custom_format: Option<Box<dyn Fn(T) -> String>>,

    selected_index: usize,

    /// Applied to the collapsed label and to every row.
    text_color: Option<UIColor>,
    text_size:  Option<f32>,
    accent:     Color,

    /// The border set by the app, put back when the box is idle again.
    idle_border: Option<Color>,
    raised:      bool,

    rows: Vec<Weak<DropDownRow>>,

    #[init]
    button: Button,
    label:  Label,
    arrow:  ImageView,
    panel:  Container,
}

impl<T: ToLabel + Clone + 'static> DropDown<T> {
    pub fn on_changed(&self, action: impl FnMut(T) + Send + 'static) {
        self.changed.val(action);
    }

    pub fn try_get_value(&self) -> Option<&T> {
        self.values.get(self.selected_index)
    }

    pub fn value(&self) -> &T {
        assert!(!self.values.is_empty());
        self.values.get(self.selected_index).unwrap()
    }

    /// The text shown while the drop down is collapsed.
    pub fn text(&self) -> &str {
        self.label.text()
    }

    pub fn is_opened(&self) -> bool {
        self.opened
    }

    pub fn set_values(&mut self, values: Vec<T>) {
        self.values = values;
        self.select_index(0);
    }

    /// Text color of the collapsed label and the rows.
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

    /// The color of the picked row, the check mark, the hover wash and
    /// the border while hovered or open.
    pub fn set_accent_color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.accent = color.into();
        self
    }

    pub fn custom_format(&mut self, format: impl Fn(T) -> String + 'static) {
        self.custom_format = Some(Box::new(format));
        self.set_values(self.values.clone());
    }

    fn format(&self, value: T) -> String {
        match &self.custom_format {
            Some(format) => format(value),
            None => value.to_label(),
        }
    }

    fn select_index(&mut self, index: usize) {
        self.selected_index = index;

        let Some(value) = self.values.get(index).cloned() else {
            self.label.set_text("");
            return;
        };

        let text = self.format(value);
        self.label.set_text(text);
    }

    fn tapped(mut self: Weak<Self>) {
        if self.opened.toggle() {
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

        self.set_border_color(self.accent);

        let text_color = self.text_color;
        let text_size = self.text_size;
        let accent = self.accent;

        self.panel.remove_all_subviews();
        self.rows.clear();

        let count = self.values.len();
        let panel_height = ROW_HEIGHT * count.lossy_convert() + 2.0 * PANEL_PADDING;
        let width = self.width();

        let below = self.superview().height() - self.max_y() >= panel_height + PANEL_GAP;
        let y = if below {
            self.height() + PANEL_GAP
        } else {
            -(panel_height + PANEL_GAP)
        };

        self.panel
            .set_color(*self.color())
            .set_corner_radii(self.corner_radii())
            .set_border_width(self.border_width())
            .set_border_color(self.idle_border.unwrap_or(*self.border_color()))
            .set_shadow(Shadow::default());
        self.panel.set_frame((0.0, y, width, panel_height));
        self.panel.set_hidden(false);

        for (index, value) in self.values.clone().into_iter().enumerate() {
            let row = self.panel.add_view::<DropDownRow>();
            row.set_frame((
                PANEL_PADDING,
                PANEL_PADDING + ROW_HEIGHT * index.lossy_convert(),
                width - 2.0 * PANEL_PADDING,
                ROW_HEIGHT,
            ));
            row.setup_row(
                self.format(value),
                index == self.selected_index,
                accent,
                text_color,
                text_size,
            );
            row.picked.val(self, move |()| self.pick(index));
            self.rows.push(row);
        }
    }

    fn close(mut self: Weak<Self>) {
        self.panel.set_hidden(true);
        self.panel.remove_all_subviews();
        self.rows.clear();
        if let Some(border) = self.idle_border {
            self.set_border_color(border);
        }
    }

    fn pick(mut self: Weak<Self>, index: usize) {
        self.select_index(index);
        self.changed.trigger(self.values[index].clone());
        self.opened = false;
        self.close();
    }
}

impl<T: ToLabel + Clone + PartialEq + 'static> DropDown<T> {
    /// Points the drop down at `value` and updates the collapsed text.
    /// The list stays closed and `changed` does not fire, so restoring a
    /// selection is never mistaken for a user pick. Returns false and
    /// changes nothing when the value is not among the current ones.
    pub fn set_value(&mut self, value: &T) -> bool {
        let Some(index) = self.values.iter().position(|existing| existing == value) else {
            return false;
        };

        self.select_index(index);

        true
    }
}

impl<T: ToLabel + Clone + 'static> Setup for DropDown<T> {
    fn setup(mut self: Weak<Self>) {
        self.accent = LIGHT_BLUE;
        self.set_color(WHITE);

        self.button.set_color(CLEAR).place().back();
        self.button.on_tap(move || self.tapped());

        self.label.set_color(CLEAR).set_alignment(TextAlignment::Left);
        self.label.place().l(INSET).r(INSET + ARROW + 6.0).tb(0);

        self.arrow.set_image(UIImages::chevron_down());
        self.arrow.place().r(INSET - 2.0).center_y().size(ARROW, ARROW);

        self.panel.set_hidden(true);

        // The border the app set is what idle looks like. Read on the
        // first hover or open, after the app's setup has run.
        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| {
            if self.idle_border.is_none() {
                self.idle_border = Some(*self.border_color());
            }
            if hovered {
                self.set_border_color(self.accent);
            } else if !self.opened
                && let Some(border) = self.idle_border
            {
                self.set_border_color(border);
            }
        });
    }
}

/// One row of the open panel.
#[view]
struct DropDownRow {
    picked:   UIEvent,
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

        self.enable_touch();
        self.touch().up_inside.sub(self, move || self.picked.trigger(()));

        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| self.refresh(hovered));
    }
}
