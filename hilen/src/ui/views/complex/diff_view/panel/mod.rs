mod cell;
/// The code of the test is drawn in the bundled mono font.
#[cfg(all(feature = "ui-tests", feature = "font-roboto-mono"))]
mod look_test;
pub(crate) mod selection;

use cell::DiffLineCell;
use filediff::{DiffHunk, DiffTarget, FileDiff, FileDiffBody, hunk_patch, line_patch};
use web_time::Instant;

use super::{
    diff_colors as colors,
    highlight::{self, LineRuns},
    icons,
    image_diff::ImageDiff,
};
use crate::{
    dispatch::{after, on_main},
    refs::{Weak, weak_from_ref},
    ui::{
        Button, CellRegistry, Color, Container, KeyCombo, Label, Point, RingSpinner, Setup, TableData,
        TableView, TextAlignment, Theme, UIColor, UIEvents, UIManager, View, ViewData, ViewFrame,
        ViewSubviews, ViewTooltip, ViewTouch, view,
    },
    window::Font,
};

const HEADER_H: f32 = 32.0;
// The square close button, its icon fills it.
const CLOSE_SIZE: f32 = 22.0;
const LINE_H: f32 = 18.0;
const HUNK_H: f32 = 20.0;
const GUTTER_W: f32 = 40.0;
const MARKER_W: f32 = 16.0;
const DISCARD_W: f32 = 60.0;
// How long a load may run before its spinner shows, so a local diff
// never flashes it.
const SPINNER_DELAY: f32 = 0.15;

// Where the code text starts in a row, after the pinned gutter and
// marker. A split row has 1 gutter per half, a unified row has 2.
fn content_x(split: bool) -> f32 {
    let gutters = if split { GUTTER_W } else { GUTTER_W * 2.0 };
    gutters + MARKER_W + 4.0
}

// The space kept free right of the code text, for the discard button of
// a row that has one.
fn reserved_w(discardable: bool) -> f32 {
    if discardable { DISCARD_W + 8.0 } else { 4.0 }
}

const ADD_BG: Color = Color::rgba(0.063, 0.725, 0.506, 0.10);
const DEL_BG: Color = Color::rgba(0.937, 0.267, 0.267, 0.10);

/// The file a panel shows and what its diff is taken against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffShown {
    pub file:     String,
    pub old_path: Option<String>,
    pub target:   DiffTarget,
}

/// What a diff panel asks of the app it lives in. Every callback runs
/// on the main thread.
pub trait DiffHost {
    /// Loads the diff of one file, off the main thread when it is slow.
    fn load(&self, shown: &DiffShown, done: Box<dyn FnOnce(Result<FileDiff, String>) + Send>);
    /// Takes a patch back out of the working tree.
    fn discard(&self, patch: String, done: Box<dyn FnOnce(Result<(), String>) + Send>);
    /// Asks the user before a discard, `yes` runs on a yes.
    fn confirm(&self, question: String, action: &'static str, yes: Box<dyn FnOnce() + Send>);
    /// Tells the user how a discard went.
    fn report(&self, result: Result<(), String>, done_text: &'static str);
    /// The close button was pressed.
    fn close(&self);
    /// The font of the code.
    fn mono(&self) -> Weak<Font>;
    /// The split or unified toggle was pressed, the panel already shows
    /// the new mode.
    fn mode_changed(&self, split: bool);
}

// One side of a side-by-side row: removals and context on the left,
// additions and context on the right.
#[derive(Clone)]
struct Half {
    no:         Option<u32>,
    kind:       char,
    text:       String,
    line_index: usize,
    global:     usize,
}

#[derive(Clone)]
enum DiffRow {
    Hunk {
        title:      String,
        hunk_index: usize,
    },
    Line {
        old_no:     Option<u32>,
        new_no:     Option<u32>,
        kind:       char,
        text:       String,
        hunk_index: usize,
        line_index: usize,
        global:     usize,
    },
    Split {
        left:       Option<Half>,
        right:      Option<Half>,
        hunk_index: usize,
    },
    Note(String),
}

/// The flags of the text selection, see selection.rs: whether it is on
/// the right half of a split row, and whether a drag moves its caret.
#[derive(Default)]
struct SelFlags {
    right:    bool,
    dragging: bool,
}

#[view]
pub struct DiffPanel {
    host:           Option<Box<dyn DiffHost>>,
    split:          bool,
    rows:           Vec<DiffRow>,
    hunks:          Vec<DiffHunk>,
    can_discard:    bool,
    toggle_hovered: bool,
    shown:          Option<DiffShown>,
    shown_version:  u64,
    // Counts the loads, an answer to an older one is dropped.
    load_seq:       u64,
    loaded:         Option<Result<FileDiff, String>>,
    runs:           Vec<LineRuns>,
    // The width of the widest code line of the shown diff, every code
    // label is this wide so the rows scroll sideways together.
    text_width:     f32,

    // The text selection over the diff rows, see selection.rs. Anchor
    // and caret are row and byte, the caret being the moving end.
    sel_anchor: Option<(usize, usize)>,
    sel_caret:  Option<(usize, usize)>,
    sel:        SelFlags,
    last_click: Option<(Instant, Point, u8)>,

    #[init]
    header:      Container,
    title:       Label,
    subtitle:    Label,
    spinner:     RingSpinner,
    mode_toggle: Button,
    close:       Button,
    table:       TableView,
    image:       ImageDiff,
    // Never shown, measures code lines in the font of the cells.
    measure:     Label,
}

impl Setup for DiffPanel {
    fn setup(mut self: Weak<Self>) {
        self.set_color(colors::BG);

        // Touch dispatch checks the latest registration first. The panel
        // registers before everything else in this setup, so the header
        // and discard buttons and the low priority table tap all stay in
        // front, and the panel gets the drags nothing else claimed, for
        // the text selection in selection.rs.
        self.enable_touch();
        self.touch().began.val(move |touch| self.sel_began(touch.position));
        self.touch().moved.val(move |touch| self.sel_moved(touch.position));
        self.touch().all.val(move |touch| {
            if touch.is_ended() {
                self.sel_ended();
            }
        });
        // A click outside the diff clears the selection, the way the
        // browser selection of the original cleared.
        UIEvents::touch_began().val(self, move |touch| {
            if !self.table.absolute_frame().contains(touch.position) {
                self.sel_clear();
            }
        });
        UIManager::keymap().add(self, KeyCombo::cmd('c'), move || self.copy_selection());

        self.header.set_color(colors::BG_BAR);
        self.header.place().t(0).lr(0).h(HEADER_H);
        let border = self.header.add_view::<Container>();
        border.set_color(colors::BORDER);
        border.place().b(0).lr(0).h(1);

        self.title
            .set_text_size(12)
            .set_text_color(colors::FG)
            .set_alignment(TextAlignment::Left)
            .set_ellipsize(true);
        self.title.place().l(4).r(160).t(0).h(HEADER_H);

        // The NEW FILE and DELETED tag of the original header, upper
        // case at opacity-60 before the mode toggle.
        self.subtitle
            .set_text_size(10)
            .set_text_color(colors::FG_60)
            .set_alignment(TextAlignment::Right);
        self.subtitle.place().r(60).t(0).w(90).h(HEADER_H);

        self.spinner.set_ring_color(colors::MUTED).set_line_width(2);
        self.spinner.place().r(155).t((HEADER_H - 12.0) / 2.0).size(12, 12);
        self.spinner.set_hidden(true);

        self.mode_toggle.set_color(colors::BG_BAR);
        self.mode_toggle.place().r(36).t(7).size(18, 18);
        // p-1 rounded hover:bg with the muted icon going fg.
        self.mode_toggle.set_corner_radius(3);
        self.mode_toggle.enable_hover();
        self.mode_toggle.touch().hovered.val(self, move |hovered| {
            self.toggle_hovered = hovered;
            let bg: UIColor = if hovered {
                colors::HOVER.into()
            } else {
                colors::BG_BAR.into()
            };
            self.mode_toggle.set_color(bg);
            self.update_mode_toggle();
        });
        self.mode_toggle.on_tap(move || {
            let split = !self.split;
            self.set_split(split);
            if let Some(host) = &self.host {
                host.mode_changed(split);
            }
        });

        self.close.set_image(icons::x(CLOSE_SIZE, colors::MUTED.resolve()));
        self.close.set_color(colors::BG_BAR);
        self.close.place().r(8).t(5).size(CLOSE_SIZE, CLOSE_SIZE);
        self.close.set_corner_radius(3);
        self.close.enable_hover();
        self.close.touch().hovered.val(self, move |hovered| {
            let bg: UIColor = if hovered {
                colors::HOVER.into()
            } else {
                colors::BG_BAR.into()
            };
            self.close.set_color(bg);
            let tint = if hovered {
                colors::FG.resolve()
            } else {
                colors::MUTED.resolve()
            };
            self.close.set_image(icons::x(CLOSE_SIZE, tint));
        });
        self.close.on_tap(move || {
            if let Some(host) = &self.host {
                host.close();
            }
        });

        self.table.place().lbr(0).t(HEADER_H);
        self.table.set_data_source(self).register_cell::<DiffLineCell>();
        self.table.set_variable_heights(true);
        // Hunk headers pin to the top of the viewport like the sticky
        // headers in the original.
        self.table.set_sticky_rows(true);
        // A split row scrolls inside its half, so the range follows the
        // width of the panel, which the table fills.
        self.size_changed().sub(move || self.update_content_width());

        self.measure.set_text_size(12);
        self.measure.set_hidden(true);

        self.image.place().lbr(0).t(HEADER_H);
        self.image.set_hidden(true);

        // A theme switch swaps the syntect theme, like the isDark watch.
        UIEvents::theme_changed().val(self, move |_: Theme| self.start_highlight());

        self.update_mode_toggle();
    }
}

impl DiffPanel {
    /// Hands the panel to the app it lives in. Call it once, before the
    /// first `show`.
    pub fn set_host(mut self: Weak<Self>, host: impl DiffHost + 'static) {
        let mono = host.mono();
        self.title.set_font(mono);
        self.measure.set_font(mono);
        self.host = Some(Box::new(host));
    }

    /// Shows the diff of one file, or nothing with None. The diff loads
    /// again when the file or the target changed, or when `version` did:
    /// the app bumps it when the working tree changed under a working
    /// tree diff. `can_discard` gives the rows their discard buttons.
    pub fn show(mut self: Weak<Self>, shown: Option<DiffShown>, version: u64, can_discard: bool) {
        let version = match &shown {
            Some(sel) if sel.target.against_working_tree() => version,
            _ => 0,
        };
        if shown == self.shown && version == self.shown_version && can_discard == self.can_discard {
            return;
        }
        let same_file = shown == self.shown && version == self.shown_version;
        self.can_discard = can_discard;
        if same_file {
            self.rebuild_rows();
            return;
        }
        self.shown.clone_from(&shown);
        self.shown_version = version;
        self.load_seq += 1;
        let Some(shown) = shown else {
            self.loaded = None;
            self.runs.clear();
            self.spinner.set_hidden(true);
            self.rebuild_rows();
            return;
        };
        self.title.set_text(match &shown.old_path {
            Some(old) => format!("{old} \u{2192} {}", shown.file),
            None => shown.file.clone(),
        });
        self.subtitle.set_text("");
        self.loaded = None;
        let Some(host) = &self.host else {
            log::warn!("diff panel: no host, {} not loaded", shown.file);
            return;
        };
        let seq = self.load_seq;
        after(SPINNER_DELAY, move || {
            if self.is_ok() && self.load_seq == seq && self.loaded_seq_pending(seq) {
                self.spinner.set_hidden(false);
            }
        });
        host.load(
            &shown,
            Box::new(move |diff| {
                if !self.is_ok() || self.load_seq != seq {
                    log::debug!("diff panel: dropped the answer of an older load");
                    return;
                }
                self.spinner.set_hidden(true);
                self.apply_loaded(diff);
            }),
        );
    }

    // True while the load `seq` has no answer yet.
    fn loaded_seq_pending(&self, seq: u64) -> bool {
        self.load_seq == seq && self.loaded.is_none()
    }

    /// Shows the rows side by side or in 1 column.
    pub fn set_split(mut self: Weak<Self>, split: bool) {
        if self.split == split {
            return;
        }
        self.split = split;
        self.update_mode_toggle();
        self.rebuild_rows();
    }

    fn apply_loaded(mut self: Weak<Self>, diff: Result<FileDiff, String>) {
        if let Ok(d) = &diff {
            self.subtitle.set_text(if d.is_new {
                "NEW FILE"
            } else if d.is_deleted {
                "DELETED"
            } else {
                ""
            });
        }
        self.loaded = Some(diff);
        self.runs.clear();
        self.rebuild_rows();
        self.start_highlight();
    }

    fn update_mode_toggle(self: Weak<Self>) {
        let muted = if self.toggle_hovered {
            colors::FG.resolve()
        } else {
            colors::MUTED.resolve()
        };
        if self.split {
            self.mode_toggle.set_image(icons::split(muted));
            self.mode_toggle.set_tooltip("Switch to unified diff");
        } else {
            self.mode_toggle.set_image(icons::unified(muted));
            self.mode_toggle.set_tooltip("Switch to side-by-side diff");
        }
    }

    fn mono(&self) -> Weak<Font> {
        self.host.as_ref().map_or_else(Font::default, |host| host.mono())
    }

    fn rebuild_rows(mut self: Weak<Self>) {
        self.sel_clear();
        self.rows.clear();
        self.hunks.clear();
        let mut show_image = None;
        let loaded = self.loaded.clone();
        match &loaded {
            Some(Ok(diff)) => match &diff.body {
                FileDiffBody::Text { hunks } => {
                    let split = self.split;
                    let mut global = 0;
                    for (hunk_index, hunk) in hunks.iter().enumerate() {
                        self.rows.push(DiffRow::Hunk {
                            title: format!(
                                "@@ -{},{} +{},{} @@",
                                hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines
                            ),
                            hunk_index,
                        });
                        if split {
                            let rows = split_rows(hunk, hunk_index, global);
                            self.rows.extend(rows);
                        } else {
                            for (line_index, line) in hunk.lines.iter().enumerate() {
                                self.rows.push(DiffRow::Line {
                                    old_no: line.old_no,
                                    new_no: line.new_no,
                                    kind: kind_char(&line.kind),
                                    text: line.text.clone(),
                                    hunk_index,
                                    line_index,
                                    global: global + line_index,
                                });
                            }
                        }
                        global += hunk.lines.len();
                    }
                    self.hunks.clone_from(hunks);
                    if self.rows.is_empty() {
                        self.rows.push(DiffRow::Note("No changes to show.".to_string()));
                    }
                }
                FileDiffBody::Binary => {
                    self.rows.push(DiffRow::Note("Binary file".to_string()));
                }
                FileDiffBody::Image { old, new } => {
                    if old.is_none() && new.is_none() {
                        self.rows.push(DiffRow::Note("Image content unavailable.".to_string()));
                    } else {
                        show_image = Some((old.clone(), new.clone()));
                    }
                }
                FileDiffBody::Empty => {
                    self.rows.push(DiffRow::Note("No changes to show.".to_string()));
                }
            },
            Some(Err(e)) => self.rows.push(DiffRow::Note(e.clone())),
            None => {}
        }
        if let Some((old, new)) = show_image {
            self.image.set_hidden(false);
            self.table.set_hidden(true);
            self.image.set_blobs(old.as_ref(), new.as_ref());
        } else {
            self.image.set_hidden(true);
            self.table.set_hidden(false);
        }
        self.measure_text_width();
        self.update_content_width();
        self.table.set_content_offset_x(0);
        self.table.reload_data();
    }

    // Measures the line with the most characters, the font is monospace.
    fn measure_text_width(mut self: Weak<Self>) {
        let longest = self
            .rows
            .iter()
            .flat_map(|row| match row {
                DiffRow::Line { text, .. } => [Some(text.as_str()), None],
                DiffRow::Split { left, right, .. } => [
                    left.as_ref().map(|half| half.text.as_str()),
                    right.as_ref().map(|half| half.text.as_str()),
                ],
                _ => [None, None],
            })
            .flatten()
            .max_by_key(|text| text.chars().count())
            .map(str::to_string);
        self.text_width = match longest {
            Some(text) if !text.is_empty() => self.measure.text_layout_for(&text).x_on_line(0, text.len()),
            _ => 0.0,
        };
    }

    // The width of a row with nothing cut off: the pinned parts plus the
    // widest line. The table scrolls sideways by what does not fit.
    fn update_content_width(mut self: Weak<Self>) {
        let side = if self.split { self.width() / 2.0 } else { 0.0 };
        let width = side + content_x(self.split) + reserved_w(self.can_discard) + self.text_width;
        self.table.set_content_width(width);
    }

    fn highlight_key(&self) -> String {
        let file = self.shown.as_ref().map_or("", |s| s.file.as_str());
        let dark = Theme::current() == Theme::Dark;
        format!("{file}|{dark}")
    }

    // Tokenizes the whole diff on a background thread, the runs come
    // back when the shown file and the theme are still the same.
    fn start_highlight(self: Weak<Self>) {
        let Some(shown) = self.shown.clone() else {
            return;
        };
        let Some(Ok(diff)) = &self.loaded else {
            return;
        };
        let FileDiffBody::Text { hunks } = &diff.body else {
            return;
        };
        let lines: Vec<String> = hunks.iter().flat_map(|h| h.lines.iter().map(|l| l.text.clone())).collect();
        if lines.is_empty() {
            return;
        }
        let dark = Theme::current() == Theme::Dark;
        let key = self.highlight_key();
        let work = move || {
            let Some(runs) = highlight::highlight(&shown.file, &lines, dark) else {
                return;
            };
            on_main(move || {
                let mut panel = self;
                if !panel.is_ok() || panel.highlight_key() != key {
                    return;
                }
                panel.runs = runs;
                panel.table.reload_data();
            });
        };
        // A page has 1 thread, there the colors come in this turn. The
        // lines past `highlight::MAX_LINES` stay plain, so it stays short.
        #[cfg(not_wasm)]
        std::thread::spawn(work);
        #[cfg(wasm)]
        work();
    }

    // Drops the loaded state so the diff loads again, the working tree
    // just changed under it.
    fn reload_current(mut self: Weak<Self>) {
        let shown = self.shown.take();
        let (version, can_discard) = (self.shown_version, self.can_discard);
        self.show(shown, version, can_discard);
    }

    fn discard_hunk(self: Weak<Self>, hunk_index: usize) {
        let Some(sel) = self.shown.clone() else {
            return;
        };
        let Some(hunk) = self.hunks.get(hunk_index).cloned() else {
            return;
        };
        let Some(host) = &self.host else {
            return;
        };
        host.confirm(
            format!(
                r"Discard this hunk in '{}'?

The selected changes will be reverted in the working tree. This cannot be undone.",
                sel.file
            ),
            "Discard hunk",
            Box::new(move || {
                self.discard(hunk_patch(&sel.file, &hunk), "Hunk discarded");
            }),
        );
    }

    fn discard_line(self: Weak<Self>, hunk_index: usize, line_index: usize) {
        let Some(sel) = self.shown.clone() else {
            return;
        };
        let Some(hunk) = self.hunks.get(hunk_index).cloned() else {
            return;
        };
        let Some(host) = &self.host else {
            return;
        };
        host.confirm(
            format!("Discard this line in '{}'?\n\nThis cannot be undone.", sel.file),
            "Discard line",
            Box::new(move || {
                let Some(patch) = line_patch(&sel.file, &hunk, line_index) else {
                    return;
                };
                self.discard(patch, "Line discarded");
            }),
        );
    }

    fn discard(self: Weak<Self>, patch: String, done_text: &'static str) {
        if !self.is_ok() {
            return;
        }
        let Some(host) = &self.host else {
            return;
        };
        host.discard(
            patch,
            Box::new(move |result| {
                if !self.is_ok() {
                    return;
                }
                if let Some(host) = &self.host {
                    host.report(result, done_text);
                }
                self.reload_current();
            }),
        );
    }
}

fn kind_char(kind: &str) -> char {
    match kind {
        "add" => '+',
        "rem" => '-',
        _ => ' ',
    }
}

// The pairSplitRows pairing: removals and additions of one run align
// side by side, context lines share the row.
fn split_rows(hunk: &DiffHunk, hunk_index: usize, global_start: usize) -> Vec<DiffRow> {
    fn flush(rows: &mut Vec<DiffRow>, rems: &mut Vec<Half>, adds: &mut Vec<Half>, hunk_index: usize) {
        let count = rems.len().max(adds.len());
        for i in 0..count {
            rows.push(DiffRow::Split {
                left: rems.get(i).cloned(),
                right: adds.get(i).cloned(),
                hunk_index,
            });
        }
        rems.clear();
        adds.clear();
    }

    let mut rows = Vec::new();
    let mut rems: Vec<Half> = Vec::new();
    let mut adds: Vec<Half> = Vec::new();
    for (line_index, line) in hunk.lines.iter().enumerate() {
        let global = global_start + line_index;
        match line.kind.as_str() {
            "rem" => rems.push(Half {
                no: line.old_no,
                kind: '-',
                text: line.text.clone(),
                line_index,
                global,
            }),
            "add" => adds.push(Half {
                no: line.new_no,
                kind: '+',
                text: line.text.clone(),
                line_index,
                global,
            }),
            _ => {
                flush(&mut rows, &mut rems, &mut adds, hunk_index);
                let left = Half {
                    no: line.old_no,
                    kind: ' ',
                    text: line.text.clone(),
                    line_index,
                    global,
                };
                let right = Half {
                    no: line.new_no,
                    ..left.clone()
                };
                rows.push(DiffRow::Split {
                    left: Some(left),
                    right: Some(right),
                    hunk_index,
                });
            }
        }
    }
    flush(&mut rows, &mut rems, &mut adds, hunk_index);
    rows
}

impl TableData for DiffPanel {
    fn number_of_cells(&self) -> usize {
        self.rows.len()
    }

    fn cell_height(&self, index: usize) -> f32 {
        match self.rows.get(index) {
            Some(DiffRow::Hunk { .. }) => HUNK_H,
            _ => LINE_H,
        }
    }

    fn is_sticky(&self, index: usize) -> bool {
        matches!(self.rows.get(index), Some(DiffRow::Hunk { .. }))
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let panel = weak_from_ref(self);
        let cell = registry
            .cell::<DiffLineCell>()
            .set_row(self.rows[index].clone(), self.can_discard, panel);
        // A recycled cell may carry another row's highlight.
        cell.set_selection(self.row_selection(index));
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}

#[cfg(test)]
mod tests {
    use filediff::{DiffHunk, DiffLine};

    use super::{DiffRow, split_rows};

    fn line(kind: &str, text: &str) -> DiffLine {
        DiffLine {
            kind:   kind.to_string(),
            text:   text.to_string(),
            old_no: Some(1),
            new_no: Some(1),
        }
    }

    fn hunk(lines: Vec<DiffLine>) -> DiffHunk {
        DiffHunk {
            old_start: 1,
            old_lines: 1,
            new_start: 1,
            new_lines: 1,
            lines,
        }
    }

    #[test]
    fn pairs_removals_with_additions() {
        let rows = split_rows(
            &hunk(vec![
                line("ctx", "a"),
                line("rem", "old1"),
                line("rem", "old2"),
                line("add", "new1"),
                line("ctx", "b"),
            ]),
            0,
            0,
        );
        assert_eq!(rows.len(), 4);
        let DiffRow::Split { left, right, .. } = &rows[1] else {
            panic!("expected a split row");
        };
        assert_eq!(left.as_ref().unwrap().text, "old1");
        assert_eq!(right.as_ref().unwrap().text, "new1");
        let DiffRow::Split { left, right, .. } = &rows[2] else {
            panic!("expected a split row");
        };
        assert_eq!(left.as_ref().unwrap().text, "old2");
        assert!(right.is_none(), "unpaired removal leaves the right empty");
    }

    #[test]
    fn context_shares_the_row_and_keeps_globals() {
        let rows = split_rows(&hunk(vec![line("ctx", "same")]), 2, 10);
        let DiffRow::Split {
            left,
            right,
            hunk_index,
        } = &rows[0]
        else {
            panic!("expected a split row");
        };
        assert_eq!(*hunk_index, 2);
        let left = left.as_ref().unwrap();
        let right = right.as_ref().unwrap();
        assert_eq!(left.text, "same");
        assert_eq!(right.text, "same");
        assert_eq!(left.global, 10);
        assert_eq!(right.global, 10);
    }
}
