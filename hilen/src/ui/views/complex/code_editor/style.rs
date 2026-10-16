use crate::{
    deps::refs::Weak,
    gm::color::Color,
    ui::{DynamicColor, UIColor, selection_drawer::SELECTION_COLOR},
    window::Font,
};

/// The look of a `CodeEditor`, set with `CodeEditor::set_style`. Every
/// default color is a light and dark pair.
#[derive(Clone, Copy)]
pub struct CodeEditorStyle {
    pub text_size:    f32,
    /// The font of the code and the line numbers, `Font::mono()` when
    /// none. Columns line up only with a mono font.
    pub font:         Option<Weak<Font>>,
    /// Room between 2 lines on top of the line pitch of the font.
    pub line_spacing: f32,
    /// Room between the gutter and the code.
    pub padding:      f32,
    pub background:   UIColor,
    /// The text of the code where no color run of the app is.
    pub text:         UIColor,
    /// The column of the line numbers.
    pub gutter:       UIColor,
    pub line_number:  UIColor,
    pub caret:        UIColor,
    pub selection:    UIColor,
    /// The lines of `set_highlight`.
    pub highlight:    UIColor,
    /// The line number and the message of a line with a mark.
    pub mark:         UIColor,
    /// The background of a line with a mark.
    pub mark_line:    UIColor,
    /// The background of every match of the find bar.
    pub find_match:   UIColor,
    /// The background of the current match, stronger than the others.
    pub find_current: UIColor,
    /// The find bar over the top of the editor.
    pub find_bar:     UIColor,
    /// The fields of the find bar.
    pub find_field:   UIColor,
    /// The text of the fields and the buttons of the find bar.
    pub find_text:    UIColor,
    /// A toggle of the find bar that is on, like the one for the case.
    pub find_toggle:  UIColor,
}

impl CodeEditorStyle {
    pub const DEFAULT: Self = Self {
        text_size:    14.0,
        font:         None,
        line_spacing: 4.0,
        padding:      8.0,
        background:   dynamic("#ffffff", "#0d1117"),
        text:         dynamic("#24292f", "#e6edf3"),
        gutter:       dynamic("#f6f8fa", "#161b22"),
        line_number:  dynamic("#8c959f", "#6e7681"),
        caret:        dynamic("#0969da", "#58a6ff"),
        selection:    UIColor::Plain(SELECTION_COLOR),
        highlight:    dynamic("#fff8c5", "#3b2e00"),
        mark:         dynamic("#cf222e", "#ff7b72"),
        mark_line:    dynamic("#ffebe9", "#3d1d1f"),
        find_match:   dynamic("#fff1a8", "#4d3d00"),
        find_current: dynamic("#f9c513", "#9e6a03"),
        find_bar:     dynamic("#eaeef2", "#21262d"),
        find_field:   dynamic("#ffffff", "#0d1117"),
        find_text:    dynamic("#24292f", "#e6edf3"),
        find_toggle:  dynamic("#b6e3ff", "#1f6feb"),
    };

    pub(super) fn regular_font(&self) -> Weak<Font> {
        self.font.unwrap_or_else(Font::mono)
    }
}

impl Default for CodeEditorStyle {
    fn default() -> Self {
        Self::DEFAULT
    }
}

const fn dynamic(light: &str, dark: &str) -> UIColor {
    UIColor::Dynamic(DynamicColor::new(Color::hex(light), Color::hex(dark)))
}
