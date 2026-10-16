//! The find bar of a `CodeEditor`, over the top of the editor: the query,
//! the toggles, the count, back and next, and the replace row. It only
//! says what the user asked for with `command`, the editor does it.

use plat::Platform;

use super::{find::FindOptions, style::CodeEditorStyle};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::color::{CLEAR, Color},
    ui::{
        ImageView, Input, Label, Setup, TextAlignment, TextField, UIColor, UIEvents, UIImages, UIManager,
        ViewCallbacks, ViewData, ViewFrame, ViewTouch, view,
    },
    window::{NamedKey, image::Image},
};

/// The height of a row of the bar.
const ROW: f32 = 28.0;
/// The room around the rows and between the groups of a row.
const GAP: f32 = 4.0;
const BUTTON: f32 = 28.0;
/// Room for "999 of 999".
const COUNT_WIDTH: f32 = 84.0;
const REPLACE_WIDTH: f32 = 68.0;
const ALL_WIDTH: f32 = 40.0;
const BUTTON_TEXT: f32 = 13.0;
const ICON: f32 = 16.0;
const CORNER: f32 = 4.0;

/// What the user asked for in the bar.
#[derive(Debug, Clone)]
pub(super) enum FindCommand {
    Query(String),
    Options(FindOptions),
    Next,
    Previous,
    ShowReplace,
    Replace(String),
    ReplaceAll(String),
    /// Escape, or the close button: the bar goes and the editor gets the
    /// keys back.
    Close,
}

#[derive(Debug, Clone, Copy)]
enum BarIcon {
    Up,
    Down,
    Close,
}

impl BarIcon {
    fn image(self, tint: Color) -> Weak<Image> {
        match self {
            Self::Up => UIImages::arrow_up(tint),
            Self::Down => UIImages::arrow_down(tint),
            Self::Close => UIImages::x(tint),
        }
    }
}

/// A button of the find bar, a short text or an icon. A press leaves the
/// keys with the field that has them.
#[view]
pub(super) struct BarButton {
    icon: Option<BarIcon>,
    tint: UIColor,

    tapped: Event,

    #[init]
    label: Label,
    image: ImageView,
}

impl Setup for BarButton {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(CORNER);
        self.label.place().back();
        self.label.set_color(CLEAR);
        self.label.set_text_size(BUTTON_TEXT);
        self.image.place().center().size(ICON, ICON);
        self.image.set_hidden(true);

        self.enable_touch();
        self.set_keeps_selection(true);
        self.touch().up_inside.sub(self, move || self.tapped.trigger(()));
    }
}

impl ViewCallbacks for BarButton {
    /// The icon is an SVG with the color written in, a theme change has
    /// to tint it again.
    fn theme_changed(&mut self) {
        self.tint_icon();
    }
}

impl BarButton {
    fn set_title(self: Weak<Self>, title: &str) {
        self.label.set_text(title);
    }

    fn set_icon(mut self: Weak<Self>, icon: BarIcon) {
        self.icon = Some(icon);
        self.image.set_hidden(false);
        self.tint_icon();
    }

    fn set_tint(mut self: Weak<Self>, tint: UIColor) {
        self.tint = tint;
        self.label.set_text_color(tint);
        self.tint_icon();
    }

    fn tint_icon(&self) {
        if let Some(icon) = self.icon {
            self.image.set_image(icon.image(self.tint.resolve()));
        }
    }
}

/// The bar. The editor makes it after its own touch is on, so a press on
/// a field or a button of the bar reaches that view and not the editor.
#[view]
pub(super) struct FindBar {
    replacing: bool,
    options:   FindOptions,
    /// The editor puts a query in, no `Query` goes back for it.
    syncing:   bool,
    toggle_on: UIColor,

    pub(super) command: Event<FindCommand>,

    #[init]
    query:       TextField,
    case:        BarButton,
    word:        BarButton,
    regex:       BarButton,
    count:       Label,
    previous:    BarButton,
    next:        BarButton,
    close:       BarButton,
    replacement: TextField,
    replace:     BarButton,
    replace_all: BarButton,
}

impl Setup for FindBar {
    fn setup(self: Weak<Self>) {
        self.query.set_placeholder("Find");
        self.replacement.set_placeholder("Replace");
        self.case.set_title("Aa");
        self.word.set_title("W");
        self.regex.set_title(".*");
        self.previous.set_icon(BarIcon::Up);
        self.next.set_icon(BarIcon::Down);
        self.close.set_icon(BarIcon::Close);
        self.replace.set_title("Replace");
        self.replace_all.set_title("All");
        self.count.set_text_size(BUTTON_TEXT);
        self.count.set_color(CLEAR);
        self.set_replacing(false);

        // Enter and Escape stay with the bar, the field would end its
        // editing on them.
        self.query.take_keys([NamedKey::Enter, NamedKey::Escape]);
        self.replacement.take_keys([NamedKey::Enter, NamedKey::Escape]);
        self.query.key_taken.val(move |key| self.query_key(key));
        self.replacement.key_taken.val(move |key| self.replacement_key(key));
        self.query.changed.val(move |text| {
            if !self.syncing {
                self.command.trigger(FindCommand::Query(text));
            }
        });

        self.case.tapped.sub(move || {
            self.toggle(|options| options.case_sensitive = !options.case_sensitive);
        });
        self.word
            .tapped
            .sub(move || self.toggle(|options| options.whole_word = !options.whole_word));
        self.regex
            .tapped
            .sub(move || self.toggle(|options| options.regex = !options.regex));
        self.previous.tapped.sub(move || self.command.trigger(FindCommand::Previous));
        self.next.tapped.sub(move || self.command.trigger(FindCommand::Next));
        self.close.tapped.sub(move || self.command.trigger(FindCommand::Close));
        self.replace.tapped.sub(move || {
            let with = self.replacement.text().to_string();
            self.command.trigger(FindCommand::Replace(with));
        });
        self.replace_all.tapped.sub(move || {
            let with = self.replacement.text().to_string();
            self.command.trigger(FindCommand::ReplaceAll(with));
        });

        UIEvents::keyboard_input().val(self, move |ch| self.on_char(ch));
        self.size_changed().sub(move || self.lay_out());
    }
}

impl FindBar {
    /// The height of the bar, 1 row or 2 with the replace row.
    pub(super) fn bar_height(&self) -> f32 {
        let rows = if self.replacing { 2.0 } else { 1.0 };
        GAP + rows * (ROW + GAP)
    }

    pub(super) fn set_look(mut self: Weak<Self>, style: &CodeEditorStyle) {
        self.toggle_on = style.find_toggle;
        self.set_color(style.find_bar);
        for mut field in [self.query, self.replacement] {
            field.set_alignment(TextAlignment::Left);
            field.set_font(style.regular_font());
            field.set_text_size(style.text_size);
            field.set_color(style.find_field);
            field.set_selected_color(style.find_field);
            field.set_text_color(style.find_text);
            field.set_placeholder_color(style.line_number);
            field.set_corner_radius(CORNER);
        }
        self.count.set_text_color(style.line_number);
        let buttons = [
            self.case,
            self.word,
            self.regex,
            self.previous,
            self.next,
            self.close,
            self.replace,
            self.replace_all,
        ];
        for button in buttons {
            button.set_tint(style.find_text);
        }
        self.show_options(self.options);
    }

    /// Puts `query` into the field with no `Query` back.
    pub(super) fn set_query(mut self: Weak<Self>, query: &str) {
        self.syncing = true;
        self.query.set_text(query);
        self.syncing = false;
    }

    pub(super) fn focus_query(&self) {
        self.query.focus();
    }

    pub(super) fn focus_replacement(&self) {
        self.replacement.focus();
    }

    pub(super) fn is_replacing(&self) -> bool {
        self.replacing
    }

    /// Shows or hides the replace row. The editor lays the bar out again
    /// for its new height.
    pub(super) fn set_replacing(mut self: Weak<Self>, on: bool) {
        self.replacing = on;
        self.replacement.set_hidden(!on);
        self.replace.set_hidden(!on);
        self.replace_all.set_hidden(!on);
        if !on && self.replacement.is_editing() {
            self.query.focus();
        }
    }

    pub(super) fn show_status(&self, status: &str) {
        self.count.set_text(status);
    }

    /// The toggles show `options`.
    pub(super) fn show_options(mut self: Weak<Self>, options: FindOptions) {
        self.options = options;
        let toggles = [
            (self.case, options.case_sensitive),
            (self.word, options.whole_word),
            (self.regex, options.regex),
        ];
        for (button, on) in toggles {
            let color = if on { self.toggle_on } else { CLEAR.into() };
            button.set_color(color);
        }
    }

    /// A field of the bar has the keys.
    pub(super) fn has_keys(&self) -> bool {
        self.query.is_editing() || self.replacement.is_editing()
    }

    /// Ends the editing of a field of the bar, the bar is closing.
    pub(super) fn drop_keys(&self) {
        if self.has_keys() {
            UIManager::unselect_view();
        }
    }

    fn toggle(self: Weak<Self>, flip: fn(&mut FindOptions)) {
        let mut options = self.options;
        flip(&mut options);
        self.command.trigger(FindCommand::Options(options));
    }

    fn query_key(self: Weak<Self>, key: NamedKey) {
        let command = match key {
            NamedKey::Enter if Input::modifiers().shift_key() => FindCommand::Previous,
            NamedKey::Enter => FindCommand::Next,
            NamedKey::Escape => FindCommand::Close,
            _ => return,
        };
        self.command.trigger(command);
    }

    fn replacement_key(self: Weak<Self>, key: NamedKey) {
        let with = self.replacement.text().to_string();
        let command = match key {
            NamedKey::Enter if Input::command_held() => FindCommand::ReplaceAll(with),
            NamedKey::Enter => FindCommand::Replace(with),
            NamedKey::Escape => FindCommand::Close,
            _ => return,
        };
        self.command.trigger(command);
    }

    /// Cmd on a Mac, Ctrl elsewhere, with a letter while a field of the
    /// bar has the keys. The field does nothing with these.
    fn on_char(self: Weak<Self>, ch: char) {
        if self.is_null() || !self.has_keys() || !Input::command_held() {
            return;
        }
        let modifiers = Input::modifiers();
        let command = match Input::shortcut_char(ch).to_ascii_lowercase() {
            'f' if modifiers.alt_key() && Platform::MAC => FindCommand::ShowReplace,
            'h' if !Platform::MAC => FindCommand::ShowReplace,
            'f' => {
                self.focus_query();
                return;
            }
            'g' if modifiers.shift_key() => FindCommand::Previous,
            'g' => FindCommand::Next,
            _ => return,
        };
        self.command.trigger(command);
    }

    /// The query fills the first row up to the toggles, the replacement
    /// the second row up to its buttons. The buttons stand at the right.
    fn lay_out(&self) {
        let width = self.width();
        let top = GAP;
        let close_x = width - GAP - BUTTON;
        let next_x = close_x - BUTTON;
        let previous_x = next_x - BUTTON;
        let count_x = previous_x - GAP - COUNT_WIDTH;
        let regex_x = count_x - GAP - BUTTON;
        let word_x = regex_x - BUTTON;
        let case_x = word_x - BUTTON;
        self.close.set_frame((close_x, top, BUTTON, ROW));
        self.next.set_frame((next_x, top, BUTTON, ROW));
        self.previous.set_frame((previous_x, top, BUTTON, ROW));
        self.count.set_frame((count_x, top, COUNT_WIDTH, ROW));
        self.regex.set_frame((regex_x, top, BUTTON, ROW));
        self.word.set_frame((word_x, top, BUTTON, ROW));
        self.case.set_frame((case_x, top, BUTTON, ROW));
        self.query.set_frame((GAP, top, (case_x - 2.0 * GAP).max(0.0), ROW));

        let second = top + ROW + GAP;
        let all_x = width - GAP - ALL_WIDTH;
        let replace_x = all_x - GAP - REPLACE_WIDTH;
        self.replace_all.set_frame((all_x, second, ALL_WIDTH, ROW));
        self.replace.set_frame((replace_x, second, REPLACE_WIDTH, ROW));
        self.replacement.set_frame((GAP, second, (replace_x - 2.0 * GAP).max(0.0), ROW));
    }
}
