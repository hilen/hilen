use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::GRAY,
    ui::{
        CellRegistry, Container, Label, Setup, TableData, TableView, UIManager, View, ViewData, ViewSubviews,
        ViewTest, view,
    },
    ui_test::inject_touches,
};

static N_CELLS: AtomicUsize = AtomicUsize::new(2_000_000);
static INDEX: AtomicUsize = AtomicUsize::new(0);

#[view]
struct LongTableTest {
    #[init]
    table: TableView,
}

impl Setup for LongTableTest {
    fn setup(self: Weak<Self>) {
        self.table.place().lr(100).tb(0);
        self.table.set_data_source(self).register_cell::<Label>();
    }
}

impl TableData for LongTableTest {
    fn cell_height(&self, _: usize) -> f32 {
        40.0
    }

    fn number_of_cells(&self) -> usize {
        N_CELLS.load(Ordering::Relaxed)
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let label = registry.cell::<Label>();
        if self.table.columns == 1 {
            label.set_text(format!("Cell number: {}", index + 1));
        } else {
            label.set_text(format!("Cell: {}", index + 1));
        }

        label.add_view::<Container>().set_color(GRAY).place().w(4).sides("tlb", 0);
        label.add_view::<Container>().set_color(GRAY).place().h(4).sides("ltr", 0);

        label
    }

    fn cell_selected(&mut self, index: usize) {
        INDEX.store(index, Ordering::Relaxed);
    }
}

impl ViewTest for LongTableTest {
    // This test drives finger drag gestures, which scroll only with
    // drag scrolling on, the touch platform default.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    /// The scroll drag runs the full height, and the table needs to stay wide
    /// enough for the taps to land in it.
    fn canvas() -> (u32, u32) {
        (640, 1000)
    }

    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        N_CELLS.store(2_000_000, Ordering::Relaxed);

        inject_touches(TOUCHES_1);

        from_main(move || {
            N_CELLS.store(2_000_000 - 5, Ordering::Relaxed);
            view.table.reload_data();
        });

        inject_touches(TOUCHES_2);

        inject_touches(TOUCHES_3);

        assert_eq!(INDEX.load(Ordering::Relaxed), 25);

        from_main(move || {
            view.table.set_columns(2);
        });

        // record_ui_test();

        Ok(())
    }
}

const TOUCHES_1: &str = "
    685  126  m
    697  111  m
    565  74   m
    500  41   m
    511  28   m
    516  28   m
    519  27   m
    519  27   b
    518  36   m
    517  66   m
    517  123  m
    514  186  m
    513  228  m
    512  272  m
    512  323  m
    513  384  m
    516  422  m
    521  473  m
    527  536  m
    530  653  m
    527  736  m
    518  783  m
    511  820  m
    505  868  m
    498  925  m
    496  959  m
    495  978  m
    494  994  m
    494  1003 m
    492  994  e
    489  983  m
    498  896  m
";

const TOUCHES_2: &str = "
    524  979  b
    523  976  m
    520  966  m
    518  952  m
    514  939  m
    513  925  m
    512  913  m
    512  899  m
    512  883  m
    512  868  m
    512  849  m
    512  830  m
    513  814  m
    513  797  m
    513  782  m
    513  763  m
    515  747  m
    515  732  m
    515  718  m
    517  705  m
    517  692  m
    517  676  m
    519  661  m
    519  642  m
    519  623  m
    519  600  m
    520  579  m
    520  559  m
    520  531  m
    520  504  m
    520  475  m
    520  451  m
    519  427  m
    519  401  m
    519  377  m
    517  353  m
    517  329  m
    517  300  m
    517  272  m
    515  245  m
    513  223  m
    511  202  m
    507  187  m
    505  177  m
    501  187  m
    497  218  m
    495  256  m
    491  302  m
    487  344  m
    485  386  m
    483  415  m
    481  441  m
    481  463  m
    480  484  m
    478  503  m
    478  518  m
    479  514  m
    482  493  m
    484  474  m
    488  450  m
    490  430  m
    492  408  m
    496  382  m
    498  358  m
    498  329  m
    498  302  m
    498  274  m
    498  245  m
    498  221  m
    498  199  m
    498  184  m
    498  168  m
    498  152  m
    498  152  m
    498  172  m
    498  198  m
    500  231  m
    500  265  m
    500  302  m
    502  334  m
    502  369  m
    502  396  m
    502  422  m
    502  446  m
    502  462  m
    500  478  m
    498  466  m
    498  452  m
    497  438  m
    497  420  m
    497  399  m
    497  381  m
    497  362  m
    497  347  m
    497  340  m
    497  336  e
    495  336  m
    483  336  m
    467  336  m
    448  336  m
    420  336  m
    388  336  m
    344  332  m
    294  328  m
    232  320  m
    172  314  m
    128  306  m
    90   300  m
    66   296  m
    48   293  m
    48   297  m
    56   308  m
    63   318  m
    70   327  m
    75   335  m
    77   346  m
    81   355  m
    87   364  m
    92   366  m
    98   371  m
    108  375  m
    123  378  m
    135  381  m
    141  384  m
    144  377  m
    142  369  m
    141  375  m
    142  384  m
    145  399  m
    147  418  m
    149  445  m
    151  478  m
    157  503  m
    163  537  m
    169  570  m
    174  600  m
    178  621  m
    182  637  m
    183  649  m
    183  659  m
    183  669  m
    182  682  m
    182  693  m
    182  704  m
    181  714  m
    180  716  m
    179  705  m
    179  692  m
    179  673  m
    181  647  m
    181  618  m
    183  584  m
    185  553  m
    185  522  m
    187  491  m
    187  458  m
    187  420  m
    187  386  m
    187  349  m
    185  311  m
    183  287  m
    181  263  m
    179  246  m
    177  230  m
    174  218  m
    172  208  m
    172  197  m
    170  188  m
    169  177  m
    168  168  m
    168  157  m
    169  157  m
    170  166  m
    170  176  m
    170  187  m
    170  197  m
    170  207  m
    172  215  m
    169  223  m
    166  234  m
    163  244  m
    160  245  m
    157  238  m
    160  237  m
    154  237  b
    154  237  e
";

const TOUCHES_3: &str = "
    157  351  b
    157  351  e
";
