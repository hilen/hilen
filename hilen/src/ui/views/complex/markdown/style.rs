use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    gm::color::Color,
    ui::{DynamicColor, UIColor, code_highlighter::Token},
    window::Font,
};

static STYLE: MainLock<Option<MarkdownStyle>> = MainLock::new();

/// The look of every `MarkdownView`. Each default color is a light and
/// dark pair. An app with its own palette sets one once at startup with
/// `apply_globally`, before any markdown is shown.
#[derive(Clone, Copy)]
pub struct MarkdownStyle {
    /// The text size of a paragraph, a list and a table.
    pub text_size:         f32,
    /// The text size of a code block.
    pub code_size:         f32,
    /// How much bigger than `text_size` the text of a heading is, from
    /// level 1 to level 6. All at 1 draws a heading as bold text of the
    /// body size, the way a terminal does. The line pitch of a heading
    /// follows its size.
    pub heading_scales:    [f32; 6],
    /// The space between 2 blocks of a text, a paragraph and the next one.
    pub block_gap:         f32,
    /// The space between 2 items of a list, and between 2 blocks inside
    /// an item.
    pub item_gap:          f32,
    /// How the markers of a list stand, in 1 column by default.
    pub list_markers:      MarkdownListMarkers,
    /// The points from one line of the text to the next, the pitch of
    /// the font when not set. It is given for `text_size`: a paragraph,
    /// a list and a table cell take it as it is, a heading takes it
    /// bigger by as much as its text is bigger. A code block keeps the
    /// pitch of its font.
    pub line_height:       Option<f32>,
    /// A line break inside a paragraph stays a line break, the way a
    /// terminal shows a text. Markdown itself reads it as a space, and that
    /// is the default. A view reads it when it gets its text.
    pub keeps_line_breaks: bool,
    /// The text, unless the view has its own color.
    pub text:              UIColor,
    /// The text of a blockquote and the markers of a list.
    pub dim_text:          UIColor,
    /// A link, and the box of a done task.
    pub link:              UIColor,
    /// Code inside a line of text.
    pub inline_code:       UIColor,
    /// Behind a code block.
    pub code_background:   UIColor,
    /// Behind the head row of a table.
    pub table_head:        UIColor,
    /// The lines of a table, a horizontal rule and the bar of a quote.
    pub line:              UIColor,
    pub comment:           UIColor,
    pub string:            UIColor,
    /// Numbers and other constants.
    pub number:            UIColor,
    pub keyword:           UIColor,
    pub function:          UIColor,
    /// The name of a type.
    pub type_name:         UIColor,
    /// A line a diff adds.
    pub inserted:          UIColor,
    /// A line a diff removes.
    pub deleted:           UIColor,
    /// The fonts, the ones the engine brings when not set.
    pub fonts:             Option<MarkdownFonts>,
}

/// How the markers of a list stand, the bullets, the numbers and the
/// boxes of tasks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkdownListMarkers {
    /// Every marker takes the width of the widest one of its list and
    /// stands at the right edge of it. The texts of all items start in 1
    /// column.
    #[default]
    Column,
    /// Every marker starts at the left edge of its list, and the text of
    /// an item starts 1 space of the font after its own marker, the way a
    /// terminal shows a list. The text of item 10 starts 1 char to the
    /// right of the text of item 9.
    Inline,
}

/// The 6 fonts a markdown text is drawn with.
#[derive(Clone, Copy)]
pub struct MarkdownFonts {
    pub regular:     Weak<Font>,
    pub bold:        Weak<Font>,
    pub italic:      Weak<Font>,
    pub bold_italic: Weak<Font>,
    pub mono:        Weak<Font>,
    pub mono_bold:   Weak<Font>,
}

impl MarkdownFonts {
    /// The default font of the app for plain text, and the Roboto files
    /// of the engine for the rest.
    pub fn bundled() -> Self {
        Self {
            regular:     Font::default(),
            bold:        Font::bold(),
            italic:      Font::italic(),
            bold_italic: Font::bold_italic(),
            mono:        Font::mono(),
            mono_bold:   Font::mono_bold(),
        }
    }
}

impl MarkdownStyle {
    pub const DEFAULT: Self = Self {
        text_size:         14.0,
        code_size:         13.0,
        heading_scales:    [1.57, 1.36, 1.14, 1.07, 1.0, 1.0],
        block_gap:         10.0,
        item_gap:          4.0,
        list_markers:      MarkdownListMarkers::Column,
        line_height:       None,
        keeps_line_breaks: false,
        text:              dynamic("#1a1d24", "#eceff4"),
        dim_text:          dynamic("#6b7280", "#9aa3b2"),
        link:              dynamic("#2f7df6", "#3d8bff"),
        inline_code:       dynamic("#b4491f", "#f0a070"),
        code_background:   dynamic("#eef1f5", "#0f1319"),
        table_head:        dynamic("#eaedf1", "#212731"),
        line:              dynamic("#dce0e6", "#2c3440"),
        comment:           dynamic("#6b7280", "#7c8696"),
        string:            dynamic("#2e7d32", "#98c379"),
        number:            dynamic("#b45309", "#d19a66"),
        keyword:           dynamic("#8250df", "#c678dd"),
        function:          dynamic("#0969da", "#61afef"),
        type_name:         dynamic("#0e7490", "#56b6c2"),
        inserted:          dynamic("#16a34a", "#22c55e"),
        deleted:           dynamic("#dc2626", "#ef4444"),
        fonts:             None,
    };

    pub fn apply_globally(self) {
        *STYLE.get_mut() = Some(self);
    }

    /// The style a markdown view draws with now.
    pub fn current() -> Self {
        STYLE.unwrap_or(Self::DEFAULT)
    }

    /// The test harness gives every test the default look and hands the
    /// app's own style back after the run.
    pub(crate) fn take_global() -> Option<Self> {
        STYLE.get_mut().take()
    }

    pub(crate) fn restore_global(style: Option<&Self>) {
        *STYLE.get_mut() = style.copied();
    }

    pub(crate) fn fonts(&self) -> MarkdownFonts {
        self.fonts.unwrap_or_else(MarkdownFonts::bundled)
    }

    /// The text size of a heading of a level, 1 to 6.
    pub(crate) fn heading_size(&self, level: u8) -> f32 {
        let scale = self.heading_scales[usize::from(level.saturating_sub(1)).min(5)];
        (self.text_size * scale).round()
    }

    /// The line pitch of a text of a size, none when the font sets it.
    pub(crate) fn line_height_for(&self, size: f32) -> Option<f32> {
        self.line_height.map(|height| height * size / self.text_size)
    }

    /// The room 1 line of body text takes: the height of a list marker
    /// and the least height of a list item and of a table row.
    pub(crate) fn body_line(&self) -> f32 {
        self.line_height.unwrap_or_else(|| (self.text_size * 1.3).ceil())
    }

    pub(crate) fn syntax(&self, token: Token) -> UIColor {
        match token {
            Token::Comment => self.comment,
            Token::String => self.string,
            Token::Number | Token::Constant => self.number,
            Token::Keyword => self.keyword,
            Token::Function => self.function,
            Token::Type => self.type_name,
            Token::Inserted => self.inserted,
            Token::Deleted => self.deleted,
        }
    }
}

impl Default for MarkdownStyle {
    fn default() -> Self {
        Self::DEFAULT
    }
}

const fn dynamic(light: &str, dark: &str) -> UIColor {
    UIColor::Dynamic(DynamicColor::new(Color::hex(light), Color::hex(dark)))
}
