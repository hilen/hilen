use std::ops::Deref;

use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::Toggle,
    refs::Weak,
    ui::{CellRegistry, Setup, TableData, TableView, View, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{inject_touches, inject_touches_delayed},
};
use parking_lot::Mutex;

use crate::interface::dev::{MenuEntry, Node, NodeCell};

static DATA: Mutex<String> = Mutex::new(String::new());

#[view]
struct TreeTableTest {
    root: Node<MenuEntry>,

    #[init]
    table: TableView,
}

impl Setup for TreeTableTest {
    fn setup(mut self: Weak<Self>) {
        self.table.set_size(400, 400);
        self.table.set_data_source(self).register_cell::<NodeCell>();

        self.root = Node::new(
            MenuEntry::new("Root"),
            vec![
                Node::new(
                    MenuEntry::new("food"),
                    vec![
                        MenuEntry::new("pizza")
                            .action(|| {
                                *DATA.lock() += "|pizza|";
                            })
                            .into(),
                        MenuEntry::new("borgor")
                            .action(|| {
                                *DATA.lock() += "|borgor|";
                            })
                            .into(),
                        MenuEntry::new("ojje")
                            .action(|| {
                                *DATA.lock() += "|ojje|";
                            })
                            .into(),
                    ],
                ),
                Node::new(
                    MenuEntry::new("dogs"),
                    vec![
                        MenuEntry::new("barker")
                            .action(|| {
                                *DATA.lock() += "|barker|";
                            })
                            .into(),
                        MenuEntry::new("woofer")
                            .action(|| {
                                *DATA.lock() += "|woofer|";
                            })
                            .into(),
                    ],
                ),
            ],
        );
    }
}

impl TableData for TreeTableTest {
    fn cell_height(&self, _: usize) -> f32 {
        40.0
    }

    fn number_of_cells(&self) -> usize {
        self.root.length()
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let node = self.root.val_at_index(index);

        let cell = registry.cell::<NodeCell>();

        let node = node.clone();

        cell.set_node(&node);
        cell.label.set_color(WHITE);

        cell
    }

    fn cell_selected(&mut self, index: usize) {
        let val = self.root.val_at_index(index);

        if val.is_leaf() {
            val.value.run();
            return;
        }

        val.open.toggle();
        self.root.update_indices(0, 0);
        self.table.reload_data();
    }
}

impl ViewTest for TreeTableTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches(
            "
                128  209  b
                128  208  e
                129  173  b
                130  170  e
                133  118  b
                133  118  e
                133  88   b
                133  88   e
                133  58   b
                132  58   e
                142  25   b
                142  25   e
            ",
        );

        let rows = from_main(move || view.number_of_cells());

        assert_eq!(rows, 3);

        inject_touches_delayed(
            "
            185  59   b
            185  59   e
            174  217  b
            174  217  e
        ",
        );

        let rows = from_main(move || view.number_of_cells());

        assert_eq!(rows, 8);

        inject_touches(
            "
            152  99   b
            152  99   e
            123  141  b
            123  141  e
            105  178  b
            105  178  e
            106  251  b
            106  251  e
            115  299  b
            115  298  e
        ",
        );

        assert_eq!(DATA.lock().deref(), "|pizza||borgor||ojje||barker||woofer|");
        DATA.lock().clear();

        // hilen::ui_test::record_ui_test();

        Ok(())
    }
}
