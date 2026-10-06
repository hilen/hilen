use super::{
    icon_button::IconButton,
    look::{BORDER, DIM_TEXT, FAINT_TEXT, FIELD, Icon, PANEL, RADIUS, TEXT, TEXT_SIZE},
    path_bar::PathBar,
};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    ui::{Container, ImageView, Setup, TextAlignment, TextField, ViewCallbacks, ViewData, ViewFrame, view},
};

const ROW: f32 = 44.0;
const PAD: f32 = 8.0;
const BUTTON: f32 = 28.0;
const GAP: f32 = 2.0;
const CONTROL_HEIGHT: f32 = 28.0;
const SEARCH_WIDTH: f32 = 150.0;
const SEARCH_ICON: f32 = 14.0;
/// Below this width the search and the switches move to a second row.
const ONE_ROW_WIDTH: f32 = 560.0;

/// The bar over the list: back, forward and up, the path bar, the search
/// field, the hidden files switch and the list or grid switch.
#[view]
pub(super) struct Toolbar {
    pub back:           Event,
    pub forward:        Event,
    pub up:             Event,
    pub search_changed: Event<String>,
    pub hidden_tapped:  Event,
    pub grid_tapped:    Event,

    #[init]
    pub(super) back_button:    IconButton,
    pub(super) forward_button: IconButton,
    pub(super) up_button:      IconButton,
    pub(super) path_bar:       PathBar,
    search_box:                Container,
    search_icon:               ImageView,
    pub(super) search:         TextField,
    pub(super) hidden_button:  IconButton,
    pub(super) grid_button:    IconButton,
    line:                      Container,
}

impl Toolbar {
    /// The height the toolbar needs at a width: 1 row, or 2 on a narrow
    /// screen.
    pub(super) fn height_for(width: f32) -> f32 {
        if width >= ONE_ROW_WIDTH {
            ROW
        } else {
            ROW * 2.0 - PAD
        }
    }

    pub(super) fn set_grid(&self, grid: bool) {
        self.grid_button.set_icon(if grid { Icon::List } else { Icon::Grid });
    }

    fn layout(self: Weak<Self>) {
        let width = self.width();
        if width <= 0.0 {
            return;
        }
        let one_row = width >= ONE_ROW_WIDTH;
        let top = (ROW - CONTROL_HEIGHT) / 2.0;

        let mut x = PAD;
        for button in [self.back_button, self.forward_button, self.up_button] {
            button.set_frame((x, top, BUTTON, BUTTON));
            x += BUTTON + GAP;
        }
        x += PAD - GAP;

        let switches = BUTTON * 2.0 + GAP;
        let right_block = SEARCH_WIDTH + PAD + switches;

        // The right block sits in the first row, or alone in the second.
        let (block_y, path_right) = if one_row {
            (top, width - PAD - right_block - PAD)
        } else {
            (ROW - PAD + top, width - PAD)
        };
        self.path_bar.set_frame((x, top, (path_right - x).max(0.0), CONTROL_HEIGHT));

        let switches_x = width - PAD - switches;
        self.hidden_button.set_frame((switches_x, block_y, BUTTON, BUTTON));
        self.grid_button.set_frame((switches_x + BUTTON + GAP, block_y, BUTTON, BUTTON));

        let search_x = if one_row {
            switches_x - PAD - SEARCH_WIDTH
        } else {
            PAD
        };
        let search_width = switches_x - PAD - search_x;
        self.search_box.set_frame((search_x, block_y, search_width, CONTROL_HEIGHT));
        self.search_icon.set_frame((
            search_x + 7.0,
            block_y + (CONTROL_HEIGHT - SEARCH_ICON) / 2.0,
            SEARCH_ICON,
            SEARCH_ICON,
        ));
        self.search.set_frame((
            search_x + 7.0 + SEARCH_ICON + 2.0,
            block_y + 1.0,
            (search_width - SEARCH_ICON - 12.0).max(0.0),
            CONTROL_HEIGHT - 2.0,
        ));

        self.line.set_frame((0.0, self.height() - 1.0, width, 1.0));
    }

    fn tint_icons(&self) {
        self.search_icon.set_image(Icon::Search.image(DIM_TEXT.resolve()));
    }
}

impl ViewCallbacks for Toolbar {
    fn theme_changed(&mut self) {
        self.tint_icons();
    }
}

impl Setup for Toolbar {
    fn setup(self: Weak<Self>) {
        self.set_color(PANEL);
        self.line.set_color(BORDER);

        self.back_button.set_icon(Icon::Back);
        self.forward_button.set_icon(Icon::Forward);
        self.up_button.set_icon(Icon::Up);
        self.hidden_button.set_icon(Icon::Eye);
        self.set_grid(false);

        self.back_button.tapped.sub(move || self.back.trigger(()));
        self.forward_button.tapped.sub(move || self.forward.trigger(()));
        self.up_button.tapped.sub(move || self.up.trigger(()));
        self.hidden_button.tapped.sub(move || self.hidden_tapped.trigger(()));
        self.grid_button.tapped.sub(move || self.grid_tapped.trigger(()));

        self.search_box
            .set_color(FIELD)
            .set_corner_radius(RADIUS)
            .set_border_width(1)
            .set_border_color(BORDER);

        self.search
            .set_placeholder("Search")
            .set_placeholder_color(FAINT_TEXT)
            .set_text_size(TEXT_SIZE)
            .set_text_color(TEXT)
            .set_selected_color(FIELD);
        let mut search = self.search;
        search.set_alignment(TextAlignment::Left);
        self.search.set_color(FIELD);
        self.search.changed.val(move |text| self.search_changed.trigger(text));

        self.tint_icons();
        self.size_changed().sub(move || self.layout());
    }
}
