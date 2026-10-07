use super::look::{BORDER, DIM_TEXT, FAINT_TEXT, FIELD, HOVER, Icon, RADIUS, TEXT, TEXT_SIZE};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::{color::CLEAR, flat::Point},
    ui::{
        DynamicColor, ImageView, Label, Setup, TextAlignment, TextField, UIColor, ViewCallbacks, ViewData,
        ViewFrame, ViewSubviews, ViewTouch, view,
    },
};

const INSET: f32 = 4.0;
const CRUMB_PADDING: f32 = 6.0;
const SEPARATOR: f32 = 12.0;
const ELLIPSIS: &str = "...";

/// One part of the path in the path bar. A tap opens that folder.
#[view]
pub(super) struct Crumb {
    /// How many parts of the path this crumb leads to.
    depth: usize,

    pub tapped: Event<usize>,

    #[init]
    title: Label,
}

impl Crumb {
    fn fill(mut self: Weak<Self>, title: &str, depth: usize, current: bool) -> f32 {
        self.depth = depth;
        self.title.set_text(title);
        let color: DynamicColor = if current { TEXT } else { DIM_TEXT };
        self.title.set_text_color(color);
        self.title.content_size().width.ceil() + CRUMB_PADDING * 2.0
    }

    pub(super) fn title(&self) -> &str {
        self.title.text()
    }
}

impl Setup for Crumb {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR).set_corner_radius(RADIUS - 2.0);

        self.title
            .set_color(CLEAR)
            .set_text_size(TEXT_SIZE)
            .set_alignment(TextAlignment::Center);
        self.title.place().back();

        self.enable_touch();
        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| {
            let color: UIColor = if hovered { HOVER.into() } else { CLEAR.into() };
            self.set_color(color);
        });
        self.touch().up_inside.sub(self, move || self.tapped.trigger(self.depth));
    }
}

/// The path of the open folder as a row of crumbs, each a tap target. A
/// tap on the free part of the bar turns it into a text field to type or
/// paste a path. A path too long for the bar keeps its end, the start
/// folds into one crumb of dots.
#[view]
pub(super) struct PathBar {
    parts:      Vec<String>,
    /// The path as the source writes it, what the text field starts with.
    text:       String,
    crumbs:     Vec<Weak<Crumb>>,
    separators: Vec<Weak<ImageView>>,
    editing:    bool,

    /// The number of parts of the path the tapped crumb leads to.
    pub crumb_tapped: Event<usize>,
    pub typed:        Event<String>,

    #[init]
    field: TextField,
}

impl PathBar {
    pub(super) fn set_path(mut self: Weak<Self>, parts: Vec<String>, text: String) {
        self.parts = parts;
        self.text = text;
        self.rebuild();
    }

    /// The titles of the crumbs on screen, left to right.
    pub(super) fn crumb_titles(&self) -> Vec<String> {
        self.crumbs.iter().map(|crumb| crumb.title().to_string()).collect()
    }

    /// The middle of the crumb with this title in the points of the
    /// window, for a test that taps it.
    pub(super) fn crumb_center(&self, title: &str) -> Option<Point> {
        let crumb = self.crumbs.iter().find(|crumb| crumb.title() == title)?;
        Some(crumb.absolute_frame().center())
    }

    pub(super) fn is_editing(&self) -> bool {
        self.editing
    }

    pub(super) fn begin_edit(mut self: Weak<Self>) {
        if self.editing {
            return;
        }
        self.editing = true;
        self.show_crumbs(false);
        self.field.set_hidden(false);
        self.field.set_text(&self.text);
        self.field.focus_with_keyboard();
    }

    fn end_edit(mut self: Weak<Self>) {
        if !self.editing {
            return;
        }
        self.editing = false;
        self.field.set_hidden(true);
        self.show_crumbs(true);
    }

    fn show_crumbs(&self, shown: bool) {
        for crumb in &self.crumbs {
            crumb.set_hidden(!shown);
        }
        for separator in &self.separators {
            separator.set_hidden(!shown);
        }
    }

    fn clear_crumbs(&mut self) {
        for mut crumb in self.crumbs.drain(..) {
            crumb.remove_from_superview();
        }
        for mut separator in self.separators.drain(..) {
            separator.remove_from_superview();
        }
    }

    fn add_crumb(mut self: Weak<Self>, title: &str, depth: usize, current: bool) -> (Weak<Crumb>, f32) {
        let crumb = self.add_view::<Crumb>();
        let width = crumb.fill(title, depth, current);
        crumb.tapped.val(move |depth| self.crumb_tapped.trigger(depth));
        self.crumbs.push(crumb);
        (crumb, width)
    }

    /// Builds the crumbs for the room the bar has. The end of the path
    /// stays, as many parts as fit, the rest is one crumb of dots that
    /// leads to the deepest folded folder.
    fn rebuild(mut self: Weak<Self>) {
        self.clear_crumbs();

        let room = self.width() - INSET * 2.0;
        if room <= 0.0 || self.parts.is_empty() {
            return;
        }

        let parts = self.parts.clone();
        let last = parts.len() - 1;

        let mut made: Vec<(Weak<Crumb>, f32)> = vec![];
        for (index, part) in parts.iter().enumerate() {
            made.push(self.add_crumb(part, index + 1, index == last));
        }

        // The current folder always shows, then its parents while they fit.
        let mut first = last;
        let mut used = made[last].1;
        while first > 0 {
            let with_parent = used + SEPARATOR + made[first - 1].1;
            let dots = if first > 1 {
                SEPARATOR + ELLIPSIS_WIDTH
            } else {
                0.0
            };
            if with_parent + dots > room {
                break;
            }
            used = with_parent;
            first -= 1;
        }

        let mut x = INSET;
        let height = self.height() - INSET * 2.0;

        if first > 0 {
            for (crumb, _) in made.drain(..first) {
                let mut crumb = crumb;
                self.crumbs.retain(|kept| kept.raw() != crumb.raw());
                crumb.remove_from_superview();
            }
            let (dots, _) = self.add_crumb(ELLIPSIS, first, false);
            // The dots lead the row although they were made last.
            self.crumbs.rotate_right(1);
            dots.set_frame((x, INSET, ELLIPSIS_WIDTH, height));
            x += ELLIPSIS_WIDTH;
            x = self.add_separator(x);
        }

        let count = made.len();
        for (index, (crumb, width)) in made.into_iter().enumerate() {
            let width = width.min(room - (x - INSET)).max(0.0);
            crumb.set_frame((x, INSET, width, height));
            x += width;
            if index + 1 < count {
                x = self.add_separator(x);
            }
        }

        let editing = self.editing;
        self.show_crumbs(!editing);
    }

    fn add_separator(mut self: Weak<Self>, x: f32) -> f32 {
        let separator = self.add_view::<ImageView>();
        separator.set_image(Icon::Forward.image(FAINT_TEXT.resolve()));
        separator.set_frame((x, (self.height() - SEPARATOR) / 2.0, SEPARATOR, SEPARATOR));
        self.separators.push(separator);
        x + SEPARATOR
    }
}

const ELLIPSIS_WIDTH: f32 = 28.0;

impl ViewCallbacks for PathBar {
    /// The separators are tinted icons.
    fn theme_changed(&mut self) {
        let tint = FAINT_TEXT.resolve();
        for separator in &self.separators {
            separator.set_image(Icon::Forward.image(tint));
        }
    }
}

impl Setup for PathBar {
    fn setup(self: Weak<Self>) {
        self.set_color(FIELD)
            .set_corner_radius(RADIUS)
            .set_border_width(1)
            .set_border_color(BORDER);

        self.field
            .set_text_size(TEXT_SIZE)
            .set_text_color(TEXT)
            .set_selected_color(FIELD);
        let mut field = self.field;
        field.set_alignment(TextAlignment::Left);
        self.field.set_color(FIELD).set_corner_radius(RADIUS - 1.0);
        self.field.place().all_sides(1);
        self.field.set_hidden(true);

        // The text is still in the field when the edit ends, `submitted`
        // comes after it and carries the same text.
        self.field.editing_ended.sub(move || self.end_edit());
        self.field.submitted.val(move |text| self.typed.trigger(text));

        // The crumbs sit over the bar and take their own taps, what is
        // left is the free part.
        self.enable_touch();
        self.touch().up_inside.sub(self, move || self.begin_edit());

        self.size_changed().sub(move || self.rebuild());
    }
}
