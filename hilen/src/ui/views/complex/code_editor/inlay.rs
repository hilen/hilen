//! Text an app shows inside a line of a `CodeEditor` that is no part of
//! the file, like the type of a name the code does not spell out. The file
//! and every byte place of it stay as they are: the cursor steps over such
//! a text, and a selection and a copy leave it out. `Shown` is the way
//! between a byte of the line and a byte of the line as it is drawn.

use std::{borrow::Cow, ops::Range};

use super::view::CodeEditor;
use crate::deps::refs::weak_from_ref;

/// A text shown inside a line that is no part of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inlay {
    /// The byte of the line the text stands at.
    pub byte:  usize,
    pub text:  String,
    /// The text belongs to the code after it, like the name of a
    /// parameter in front of an argument. The caret of `byte` stands after
    /// the text then, and what the user types there goes between the text
    /// and that code. With `false` it belongs to the code before it, like
    /// the type after a name, and the caret stands before it.
    pub leads: bool,
}

impl Inlay {
    /// A text that belongs to the code before `byte`, like `: i32` after
    /// a name.
    pub fn new(byte: usize, text: impl Into<String>) -> Self {
        Self {
            byte,
            text: text.into(),
            leads: false,
        }
    }

    /// A text that belongs to the code after `byte`, like `count: ` in
    /// front of an argument.
    pub fn leading(byte: usize, text: impl Into<String>) -> Self {
        Self {
            byte,
            text: text.into(),
            leads: true,
        }
    }
}

/// The inlays of 1 line, in the order they stand. At 1 byte the leading
/// ones stand first, the caret is between them and the rest.
#[derive(Debug, Default, Clone)]
pub(super) struct Inlays {
    items: Vec<Inlay>,
    /// The line the items stand in, kept so they can follow a change of
    /// it. Empty while there are no items.
    line:  String,
}

impl Inlays {
    pub fn new(line: &str, items: impl IntoIterator<Item = Inlay>) -> Self {
        let mut items: Vec<Inlay> = items.into_iter().filter(|inlay| !inlay.text.is_empty()).collect();
        for inlay in &mut items {
            inlay.byte = line.floor_char_boundary(inlay.byte.min(line.len()));
        }
        items.sort_by_key(|inlay| (inlay.byte, !inlay.leads));
        let mut inlays = Self {
            items,
            line: String::new(),
        };
        inlays.keep_line(line);
        inlays
    }

    pub fn items(&self) -> &[Inlay] {
        &self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn keep_line(&mut self, line: &str) {
        self.line.clear();
        if !self.items.is_empty() {
            self.line.push_str(line);
        }
    }

    /// The line changed to `line`. The inlays before the changed part
    /// stay, the ones after it move with their code, and the ones inside
    /// it go. What the user types at an inlay goes where the caret
    /// stands: before one that follows its code, after one that leads.
    pub fn follow(&mut self, line: &str) {
        if self.items.is_empty() {
            return;
        }
        let old = self.line.as_str();
        let prefix = common_prefix(old, line);
        let suffix = common_suffix(&old[prefix..], &line[prefix..]);
        let (old_end, new_end) = (old.len() - suffix, line.len() - suffix);
        let only_added = old_end == prefix;
        self.items.retain_mut(|inlay| {
            let before = inlay.byte < prefix || (inlay.byte == prefix && (inlay.leads || !only_added));
            if before {
                return true;
            }
            if inlay.byte < old_end {
                return false;
            }
            inlay.byte = inlay.byte - old_end + new_end;
            true
        });
        self.keep_line(line);
    }
}

/// How many bytes 2 texts start with alike, whole chars only.
fn common_prefix(a: &str, b: &str) -> usize {
    let alike = a.bytes().zip(b.bytes()).take_while(|(a, b)| a == b).count();
    a.floor_char_boundary(alike)
}

/// How many bytes 2 texts end with alike, whole chars only.
fn common_suffix(a: &str, b: &str) -> usize {
    let alike = a.bytes().rev().zip(b.bytes().rev()).take_while(|(a, b)| a == b).count();
    a.len() - a.ceil_char_boundary(a.len() - alike)
}

/// Where 1 inlay stands.
#[derive(Debug, Clone, Copy)]
struct Spot {
    /// Its byte of the line.
    byte:  usize,
    /// Where it starts in the shown text.
    at:    usize,
    len:   usize,
    leads: bool,
}

/// A line as it is drawn, with its inlays put in, and the way between a
/// byte of the line and a byte of the drawn text.
#[derive(Debug)]
pub(super) struct Shown<'a> {
    pub text: Cow<'a, str>,
    spots:    Vec<Spot>,
    /// The length of the line itself.
    len:      usize,
}

impl<'a> Shown<'a> {
    /// `inlays` in the order `Inlays` keeps them.
    pub fn of(line: &'a str, inlays: &[Inlay]) -> Self {
        if inlays.is_empty() {
            return Self {
                text:  Cow::Borrowed(line),
                spots: Vec::new(),
                len:   line.len(),
            };
        }
        let more: usize = inlays.iter().map(|inlay| inlay.text.len()).sum();
        let mut text = String::with_capacity(line.len() + more);
        let mut spots = Vec::with_capacity(inlays.len());
        let mut done = 0;
        for inlay in inlays {
            text.push_str(&line[done..inlay.byte]);
            spots.push(Spot {
                byte:  inlay.byte,
                at:    text.len(),
                len:   inlay.text.len(),
                leads: inlay.leads,
            });
            text.push_str(&inlay.text);
            done = inlay.byte;
        }
        text.push_str(&line[done..]);
        Self {
            text: Cow::Owned(text),
            spots,
            len: line.len(),
        }
    }

    /// The bytes of the inlays that stand before a byte of the line, as
    /// `counts` picks them by their own byte and whether they lead.
    fn added(&self, counts: impl Fn(usize, bool) -> bool) -> usize {
        self.spots
            .iter()
            .filter(|spot| counts(spot.byte, spot.leads))
            .map(|spot| spot.len)
            .sum()
    }

    /// Where the caret of `byte` of the line stands in the drawn text:
    /// after a leading inlay at that byte and before any other one.
    pub fn caret(&self, byte: usize) -> usize {
        let byte = byte.min(self.len);
        byte + self.added(|at, leads| at < byte || (at == byte && leads))
    }

    /// A range of the line in the drawn text: from its first char to its
    /// last, with the inlays inside it and none at its 2 ends.
    pub fn span(&self, range: &Range<usize>) -> Range<usize> {
        let (from, to) = (range.start.min(self.len), range.end.min(self.len));
        let start = from + self.added(|at, _| at <= from);
        let end = to + self.added(|at, _| at < to);
        start..end.max(start)
    }

    /// The byte of the line for a byte of the drawn text, and whether
    /// that one is inside an inlay. Inside one it is the byte the inlay
    /// stands at.
    pub fn byte_of(&self, shown: usize) -> (usize, bool) {
        let mut added = 0;
        for spot in &self.spots {
            if shown <= spot.at {
                break;
            }
            if shown < spot.at + spot.len {
                return (spot.byte, true);
            }
            added += spot.len;
        }
        (shown.min(self.text.len()) - added, false)
    }

    /// Where the inlays are in the drawn text.
    pub fn inlay_ranges(&self) -> impl Iterator<Item = Range<usize>> + '_ {
        self.spots.iter().map(|spot| spot.at..spot.at + spot.len)
    }
}

impl CodeEditor {
    /// Shows texts inside `line` that are no part of the file, like the
    /// type of a name, in the `inlay` colors of the style. They take the
    /// place of the ones the line had. The text of the editor, the cursor
    /// and every byte place stay as they are: the caret steps over an
    /// inlay, a press on one puts the caret at its byte, and a selection
    /// and a copy leave it out. When the user changes the line, its inlays
    /// move with the code around them until the app sets new ones, and
    /// the ones in the changed part go. A change over more lines than 1
    /// drops the inlays of those lines.
    pub fn set_inlays(&self, line: usize, inlays: impl IntoIterator<Item = Inlay>) {
        let mut this = weak_from_ref(self);
        let Some(text) = this.text.lines().get(line) else {
            return;
        };
        let inlays = Inlays::new(text, inlays);
        if let Some(decor) = this.decor.get_mut(line) {
            decor.inlays = inlays;
            this.width_changed(line);
        }
    }

    /// The inlays of `line` in the order they stand, with the bytes they
    /// have now.
    pub fn inlays(&self, line: usize) -> &[Inlay] {
        self.decor.get(line).map_or(&[][..], |decor| decor.inlays.items())
    }

    /// Takes the inlays off every line.
    pub fn clear_inlays(&mut self) {
        for line in 0..self.decor.len() {
            if !self.decor[line].inlays.is_empty() {
                self.decor[line].inlays = Inlays::default();
                self.width_changed(line);
            }
        }
    }
}
