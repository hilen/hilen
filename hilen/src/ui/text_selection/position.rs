use crate::{
    deps::refs::Weak,
    ui::{TableView, View, WeakView},
};

/// A place in the text of a scope, kept as data and not as a view, so it
/// stays right when a recycling table gives the cell to another row.
/// `row` is the index of the cell in the data of a table, 0 outside a
/// table. `piece` counts the selectable texts of that cell in reading
/// order. `byte` is a byte of the text of the piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) struct Position {
    pub row:   usize,
    pub piece: usize,
    pub byte:  usize,
}

impl Position {
    /// After the last byte of the last piece of the last row.
    pub const END: Self = Self {
        row:   usize::MAX,
        piece: usize::MAX,
        byte:  usize::MAX,
    };

    pub fn new(row: usize, piece: usize, byte: usize) -> Self {
        Self { row, piece, byte }
    }

    /// The piece without the byte, to tell which piece comes first.
    pub fn piece_key(self) -> (usize, usize) {
        (self.row, self.piece)
    }
}

/// What 1 step of a drag takes: a click drags by characters, a double
/// click by words and a triple click by lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Granularity {
    Char,
    Word,
    Line,
}

/// What stands between a piece and the piece before it in a copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Lead {
    /// 1 line break, 2 texts under each other.
    #[default]
    Line,
    /// An empty line, 2 blocks of a markdown text.
    Block,
    /// A list marker and the text of its item.
    Space,
    /// 2 cells of a row of a markdown table.
    Tab,
}

impl Lead {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Line => "\n",
            Self::Block => "\n\n",
            Self::Space => " ",
            Self::Tab => "\t",
        }
    }
}

/// The views 1 selection goes over.
#[derive(Clone, Copy)]
pub(crate) enum Scope {
    /// A selectable `Label` or `MarkdownView` by itself.
    View(WeakView),
    /// Every cell of a table, also the ones that are not on screen.
    Table(Weak<TableView>),
}

impl Scope {
    pub fn view(&self) -> WeakView {
        match self {
            Self::View(view) => *view,
            Self::Table(table) => table.weak_view(),
        }
    }

    pub fn is_ok(&self) -> bool {
        match self {
            Self::View(view) => view.is_ok(),
            Self::Table(table) => table.is_ok(),
        }
    }

    pub fn same(&self, other: &Self) -> bool {
        self.view().addr() == other.view().addr()
    }
}
