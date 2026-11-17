use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::from_main,
        refs::{Weak, weak_from_ref},
    },
    gm::color::Color,
    ui::{CellRegistry, Label, Setup, TreeData, TreeTable, View, ViewData, ViewTest, view},
    ui_test::{check_colors, checkpoint, inject_touches},
};

const ROW: f32 = 40.0;

/// A level, a key and a name per row, in the order of the tree.
const TREE: [(usize, u64, &str); 8] = [
    (0, 1, "epic A"),
    (1, 2, "task A1"),
    (2, 3, "subtask A1a"),
    (2, 4, "subtask A1b"),
    (1, 5, "task A2"),
    (0, 6, "epic B"),
    (1, 7, "task B1"),
    (0, 8, "epic C"),
];

/// A `TreeTable` of epics, tasks and subtasks. A tap on an arrow folds the
/// rows under its row, a fold inside a folded row stays when the outer row
/// opens again, a reload with a new row keeps the folds, and a tap on the
/// text of a row selects that row of the data.
#[view]
struct TreeTableFolds {
    rows:     Vec<(usize, u64, String)>,
    folds:    Vec<(u64, bool)>,
    selected: Vec<usize>,

    #[init]
    tree:   TreeTable,
    status: Label,
}

impl Setup for TreeTableFolds {
    fn setup(mut self: Weak<Self>) {
        self.set_color(Color::rgb(0.2, 0.2, 0.25));
        self.rows = TREE
            .iter()
            .map(|(level, key, name)| (*level, *key, (*name).to_string()))
            .collect();
        self.tree.place().tl(0).size(400, 520);
        self.tree.set_data_source(self).register_cell::<Label>();
        self.tree.folded.val(move |fold| self.folds.push(fold));
        self.tree.reload_data();
        self.status.set_text_size(16).set_color(Color::rgb(0.95, 0.9, 0.7));
        self.status.place().lrb(0).h(70);
        self.status.set_text("no row selected");
    }
}

impl TreeData for TreeTableFolds {
    fn number_of_rows(&self) -> usize {
        self.rows.len()
    }

    fn level(&self, row: usize) -> usize {
        self.rows[row].0
    }

    fn row_key(&self, row: usize) -> u64 {
        self.rows[row].1
    }

    fn row_height(&self, _: usize) -> f32 {
        ROW
    }

    fn setup_row(&mut self, row: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let (level, _, name) = &self.rows[row];
        let cell = registry.cell::<Label>();
        cell.set_text(name);
        cell.set_text_inset(weak_from_ref(self).tree.content_left(*level));
        let shade = [0.95, 0.85, 0.75][*level];
        cell.set_color(Color::rgb(shade, shade, 1.0));
        cell
    }

    fn row_selected(&mut self, row: usize) {
        let mut this = weak_from_ref(self);
        this.selected.push(row);
        let text = format!("selected {}", this.rows[row].2);
        this.status.set_text(text);
    }
}

/// The names of the rows in sight, top to bottom.
fn shown(view: Weak<TreeTableFolds>) -> Vec<String> {
    from_main(move || {
        (0..view.tree.shown_count())
            .filter_map(|index| view.tree.shown_row(index))
            .map(|row| view.rows[row].2.clone())
            .collect()
    })
}

/// A tap on the arrow of line `line` of a row of `level`.
fn tap_arrow(line: u32, level: u32) {
    let (x, y) = (level * 20 + 14, line * 40 + 20);
    inject_touches(format!("{x} {y} b\n{x} {y} e"));
}

impl ViewTest for TreeTableFolds {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        ensure!(
            shown(view).len() == 8,
            "every row shows at first, got {:?}",
            shown(view)
        );
        check_colors(ALL_OPEN)?;
        checkpoint("all open: arrows on epic A, task A1 and epic B")?;

        tap_arrow(1, 1);
        ensure!(
            shown(view) == ["epic A", "task A1", "task A2", "epic B", "task B1", "epic C"],
            "folding task A1 hides its 2 subtasks, got {:?}",
            shown(view)
        );
        tap_arrow(0, 0);
        ensure!(
            shown(view) == ["epic A", "epic B", "task B1", "epic C"],
            "folding epic A hides its tasks, got {:?}",
            shown(view)
        );
        check_colors(EPIC_FOLDED)?;
        checkpoint("epic A folded, its arrow points right")?;

        tap_arrow(0, 0);
        ensure!(
            shown(view) == ["epic A", "task A1", "task A2", "epic B", "task B1", "epic C"],
            "epic A opened, task A1 stays folded, got {:?}",
            shown(view)
        );
        let folds = from_main(move || view.folds.clone());
        ensure!(
            folds == [(2, true), (1, true), (1, false)],
            "the fold events were {folds:?}"
        );

        from_main(move || {
            let mut this = view;
            this.rows.insert(7, (2, 9, "subtask B1a".to_string()));
            this.tree.reload_data();
        });
        ensure!(
            shown(view)
                == [
                    "epic A",
                    "task A1",
                    "task A2",
                    "epic B",
                    "task B1",
                    "subtask B1a",
                    "epic C"
                ],
            "a reload with a new row keeps the folds, got {:?}",
            shown(view)
        );
        check_colors(NEW_ROW)?;
        checkpoint("a new subtask under task B1, task B1 got an arrow, task A1 stays folded")?;

        inject_touches("200 260 b\n200 260 e");
        let selected = from_main(move || view.selected.clone());
        ensure!(
            selected == [8],
            "a tap on the text of epic C selects row 8 of the data, got {selected:?}"
        );
        checkpoint("a tap on the text of epic C selected it")
    }
}

/// Every row open.
const ALL_OPEN: &str = r"
             288    4 - #f2f2ff
             580    4 - #333340
             104   12 - #010101
              36   24 - #f2f2ff
              60   60 - #d9d9ff
             132   64 - #000000
             396   80 - #bfbfff
             300   84 - #bfbfff
              80  100 - #bfbfff
             200  100 - #bfbfff
             108  140 - #bfbfff
             152  144 - #bfbfff
             240  148 - #010101
             336  156 - #bfbfff
              52  172 - #000000
             592  200 - #333340
             100  212 - #010101
              36  224 - #f2f2ff
             312  256 - #d9d9ff
              72  260 - #000000
             112  292 - #010101
              36  304 - #f2f2ff
             228  316 - #f2f2ff
             396  316 - #f2f2ff
             560  364 - #333340
             592  528 - #333340
             208  532 - #f2e5b2
             404  532 - #f2e5b2
              72  564 - #f2e5b2
              72  568 - #f2e5b2
             336  592 - #f2e5b2
             472  592 - #f2e5b2
";

/// Epic A folded.
const EPIC_FOLDED: &str = r"
             248    4 - #f2f2ff
             396    4 - #f2f2ff
              60   20 - #000000
             104   20 - #f2f2ff
              36   24 - #f2f2ff
             104   52 - #f2f2ff
              48   64 - #000000
             100   68 - #000000
              96   88 - #000000
              52  100 - #000000
              72  100 - #000000
             128  104 - #d9d9ff
              96  108 - #000000
             396  112 - #d9d9ff
             100  132 - #010101
              60  140 - #000000
             296  144 - #f2f2ff
              72  148 - #f2f2ff
              48  152 - #000000
             592  152 - #333340
             204  156 - #f2f2ff
             156  292 - #333340
             320  312 - #333340
               4  360 - #333340
             592  396 - #333340
             224  532 - #f2e5b2
             468  532 - #f2e5b2
              72  564 - #f2e5b2
              72  568 - #f2e5b2
             160  592 - #f2e5b2
             368  592 - #f2e5b2
             568  592 - #f2e5b2
";

/// A new subtask under task B1, task A1 still folded.
const NEW_ROW: &str = r"
             396    4 - #f2f2ff
             104   20 - #f2f2ff
             252   20 - #f2f2ff
              36   24 - #f2f2ff
             128   60 - #d9d9ff
              72   68 - #010101
             592   88 - #333340
             132  104 - #000000
             312  120 - #f2f2ff
             220  124 - #f2f2ff
              60  140 - #000000
             120  172 - #000000
              52  184 - #000000
               4  200 - #bfbfff
             396  200 - #bfbfff
             200  212 - #bfbfff
              80  220 - #bfbfff
             160  220 - #bfbfff
             128  224 - #bfbfff
             240  224 - #010101
             320  236 - #bfbfff
             112  252 - #010101
              36  264 - #f2f2ff
             592  304 - #333340
             420  404 - #333340
             592  528 - #333340
             208  532 - #f2e5b2
             404  532 - #f2e5b2
              72  564 - #f2e5b2
              72  568 - #f2e5b2
             336  592 - #f2e5b2
             472  592 - #f2e5b2
";
