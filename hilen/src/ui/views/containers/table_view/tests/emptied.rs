use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::Color,
    ui::{CellRegistry, Container, Label, Setup, TableData, TableView, View, ViewData, ViewTest, view},
    ui_test::{check_colors, checkpoint},
};

static ROWS: AtomicUsize = AtomicUsize::new(3);

const BACKDROP: Color = Color::hex("#1b2230");
const PALETTE: [Color; 3] = [
    Color::hex("#4f8cff"),
    Color::hex("#3ecf8e"),
    Color::hex("#f5b942"),
];

// A member list whose last member is removed. The rows of the reload
// before must not stay drawn over the empty table, the backdrop no cell
// uses shows through where they were.
#[view]
struct TableEmptied {
    #[init]
    under: Container,
    table: TableView,
}

impl Setup for TableEmptied {
    fn setup(mut self: Weak<Self>) {
        ROWS.store(3, Ordering::SeqCst);
        self.under.set_color(BACKDROP);
        self.under.place().tl(0).size(400, 400);
        self.table.place().tl(0).size(400, 400);
        self.table.set_data_source(self).register_cell::<Label>();
        self.table.set_cell_spacing(10);
        self.table.set_cell_margins(20, 20);
        self.table.reload_data();
    }
}

impl TableData for TableEmptied {
    fn cell_height(&self, _: usize) -> f32 {
        60.0
    }

    fn number_of_cells(&self) -> usize {
        ROWS.load(Ordering::SeqCst)
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_text(format!("Member {}", index + 1));
        cell.set_color(PALETTE[index % PALETTE.len()]);
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}

fn reload_with(view: Weak<TableEmptied>, rows: usize) -> usize {
    ROWS.store(rows, Ordering::SeqCst);
    from_main(move || {
        let mut table = view.table;
        table.reload_data();
        table.visible_cells().len()
    })
}

impl ViewTest for TableEmptied {
    fn canvas() -> (u32, u32) {
        (400, 400)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        assert_eq!(from_main(move || view.table.visible_cells().len()), 3);
        check_colors(THREE_MEMBERS)?;
        checkpoint("3 members")?;

        assert_eq!(
            reload_with(view, 0),
            0,
            "an emptied table keeps no rows on screen"
        );
        check_colors(EMPTIED)?;
        checkpoint("emptied, only the backdrop")?;

        // The recycled cells come back when rows do.
        assert_eq!(reload_with(view, 2), 2);
        check_colors(TWO_MEMBERS)?;
        checkpoint("2 members again")?;
        Ok(())
    }
}

const THREE_MEMBERS: &str = r"
    40    4 - #4f8cff
   376    4 - #4f8cff
   164   28 - #4f8cff
   228   28 - #4f8cff
   240   32 - #1b2f56
   132   36 - #000000
   208   36 - #4f8cff
   240   36 - #1b2f56
    68   72 - #3ecf8e
   196  100 - #000101
   228  100 - #3ac386
   364  100 - #3ecf8e
   168  104 - #3ecf8e
   212  104 - #3ecf8e
   240  104 - #154630
   132  108 - #000000
   240  108 - #154630
   264  108 - #237550
   268  108 - #237550
    20  128 - #3ecf8e
   264  164 - #f5b942
   136  168 - #000000
   212  172 - #f5b942
   240  172 - #533e16
   164  176 - #f5b942
   196  176 - #010100
   240  176 - #533e16
    60  196 - #f5b942
   376  196 - #f5b942
   140  328 - #1b2230
     4  392 - #1b2230
   276  392 - #1b2230
";

const EMPTIED: &str = r"
     4    4 - #1b2230
   292    4 - #1b2230
   392    4 - #1b2230
   100    8 - #1b2230
   196    8 - #1b2230
   248   56 - #1b2230
     8  100 - #1b2230
   104  104 - #1b2230
   200  104 - #1b2230
   296  104 - #1b2230
   392  104 - #1b2230
   152  152 - #1b2230
   248  152 - #1b2230
   344  152 - #1b2230
     8  196 - #1b2230
   104  200 - #1b2230
   200  200 - #1b2230
   296  200 - #1b2230
   392  200 - #1b2230
    56  248 - #1b2230
   152  248 - #1b2230
   248  248 - #1b2230
     4  292 - #1b2230
   104  296 - #1b2230
   200  296 - #1b2230
   296  296 - #1b2230
   392  296 - #1b2230
     4  392 - #1b2230
   104  392 - #1b2230
   200  392 - #1b2230
   296  392 - #1b2230
   392  392 - #1b2230
";

const TWO_MEMBERS: &str = r"
    40    4 - #4f8cff
   304    4 - #4f8cff
   376    4 - #4f8cff
   264   20 - #000101
   164   28 - #4f8cff
   228   28 - #4f8cff
   212   32 - #4f8cff
   240   32 - #1b2f56
   132   36 - #000000
   196   36 - #000101
   228   36 - #4f8cff
   240   36 - #1b2f56
    64   72 - #3ecf8e
   332   72 - #3ecf8e
   132   92 - #000000
   148   92 - #000000
   164  100 - #3ac386
   212  100 - #3ecf8e
   196  104 - #000101
   208  104 - #3ecf8e
   232  104 - #3ecf8e
   240  104 - #154630
   140  108 - #000000
   240  108 - #154630
   264  108 - #237550
   268  108 - #237550
    20  128 - #3ecf8e
   376  128 - #3ecf8e
   332  264 - #1b2230
   196  312 - #1b2230
     4  392 - #1b2230
   392  392 - #1b2230
";
