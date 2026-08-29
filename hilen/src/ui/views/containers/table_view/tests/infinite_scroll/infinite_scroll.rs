#[cfg(not_wasm)]
use anyhow::Result;

#[cfg(not_wasm)]
use crate::deps::hreads::{from_main, wait_for_next_frame};
use crate::{
    self as hilen,
    deps::{
        hreads::{on_main, sleep, spawn},
        refs::Weak,
    },
    gm::color::BLACK,
    ui::{
        CellRegistry, Setup, Spinner, TableData, TableView, View, ViewData, view,
        views::containers::table_view::tests::infinite_scroll::infinite_cell::InfiniteCell,
    },
};
#[cfg(not_wasm)]
use crate::{
    ui::{ViewTest, views::containers::table_view::tests::infinite_scroll::basic_scroll::test_basic_scroll},
    ui_test::{inject_scroll, inject_touches},
};

#[view]
pub struct InfiniteScrollTest {
    pub page_size: usize,

    pub(super) test_string: String,

    data_size:  usize,
    requesting: bool,

    #[init]
    pub table: TableView,
}

impl Setup for InfiniteScrollTest {
    fn setup(mut self: Weak<Self>) {
        self.page_size = 100;
        self.table.columns = 2;
        self.table
            .set_data_source(self)
            .register_cell::<InfiniteCell>()
            .set_border_color(BLACK)
            .set_border_width(5)
            .place()
            .t(120)
            .b(120)
            .lr(0);

        self.table.bottom_reached().sub(self, move || {
            if self.requesting {
                return;
            }

            self.requesting = true;

            spawn(async move {
                self.on_fetch().await;
            });
        });

        self.data_size = 200;
    }
}

impl InfiniteScrollTest {
    /// Waits until the table holds `expected` cells and no fetch is in
    /// flight. The scroll that presses the bottom lands first, the
    /// `bottom_reached` event fires on a later layout frame, so a flag
    /// check right after the scroll can run before the fetch even starts.
    #[cfg(not_wasm)]
    fn wait_for_cells(view: Weak<Self>, expected: usize) {
        for _ in 0..1200 {
            let settled = from_main(move || view.data_size == expected && !view.requesting);
            if settled {
                return;
            }
            wait_for_next_frame();
        }
        panic!("Pagination did not reach {expected} cells");
    }

    async fn on_fetch(mut self: Weak<Self>) {
        let _spin = Spinner::lock();

        fetch_more_data().await;

        on_main(move || {
            self.data_size += self.page_size;
            self.table.reload_data();
            self.requesting = false;
        });
    }
}

impl TableData for InfiniteScrollTest {
    fn cell_height(&self, _index: usize) -> f32 {
        80.0
    }

    fn number_of_cells(&self) -> usize {
        self.data_size
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        registry.cell::<InfiniteCell>().set_text(index)
    }

    fn cell_selected(&mut self, index: usize) {
        self.test_string.push('|');
        self.test_string.push_str(&index.to_string());
        self.test_string.push('|');
    }
}

// The test paces its scrolls one per frame against the half second fetch
// timer, and a slow browser loses that race. The failed assert then panics,
// and a wasm panic aborts the whole app because there is no unwinding, which
// kills every test after this one. Skipped on wasm until the race is fixed.
#[cfg(not_wasm)]
impl ViewTest for InfiniteScrollTest {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        test_basic_scroll(view)?;

        inject_scroll(-8000);

        for _ in 0..20 {
            inject_scroll(-10);
        }

        Self::wait_for_cells(view, 300);

        for _ in 0..100 {
            inject_scroll(-10);
        }

        from_main(move || {
            view.test_string.clear();
        });

        inject_touches(
            "
                 186  181  b
                 186  181  e
                 398  190  b
                 398  190  e
                 387  264  b
                 387  264  e
                 204  262  b
                 204  262  e
                 196  334  b
                 195  334  e
                 427  362  b
                 427  362  e
                 419  412  b
                 419  412  e
                 176  396  b
                 176  396  e
             ",
        );

        assert_eq!(view.test_string, "|216||217||219||218||220||223||223||222|");

        inject_touches(
            "
              405  44   m
              390  37   m
              379  37   m
              372  20   m
              302  314  m
              266  431  m
              320  422  m
              317  382  m
              306  380  m
              294  395  m
              326  301  m
              330  246  m
              313  240  m
              281  253  m
              267  257  m
              293  240  m
              300  229  m
              305  203  m
              308  199  b
              307  216  m
              305  248  m
              303  270  m
              295  326  m
              288  368  m
              286  408  m
              285  427  m
              285  428  e
              301  325  m
              300  247  m
              299  221  b
              297  239  m
              267  424  m
              258  490  m
              258  496  e
              288  371  m
              329  217  b
              329  227  m
              316  331  m
              305  410  m
              303  432  m
              302  442  e
              323  396  m
          ",
        );

        // The fling after the drag is still running. How many frames it
        // gets before the taps below depends on the machine and the load,
        // so under the full suite the taps once landed four rows off.
        while from_main(move || view.table.scroll.is_scrolling()) {
            wait_for_next_frame();
        }

        from_main(move || {
            view.test_string.clear();
        });

        inject_touches(
            "
             170  177  b
             170  177  e
             388  248  b
             388  248  e
             448  409  b
             448  409  e
         ",
        );

        assert_eq!(view.test_string, "|150||151||155|");

        // crate::ui_test::record_ui_test();

        Ok(())
    }
}

async fn fetch_more_data() {
    sleep(0.5).await;
}
