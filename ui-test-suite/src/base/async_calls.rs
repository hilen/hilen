// use std::{any::Any, time::Duration};
//
// use anyhow::Result;
// use hilen::{
//     dispatch::{from_main, ok_main, on_back},
//     refs::Weak,
//     hilen::ui::{
//         Anchor::{Size, Top, X},
//         BLUE, Button,  Label, Setup, Spinner, TableData, TableView,
// UI, ViewData, link_button, view,     },
//     ui_test::inject_touches,
// };
// use tokio::time::sleep;
//
// #[view]
// struct AsyncCalls {
//     #[init]
//     label:  Label,
//     button: Button,
//     table:  TableView,
// }
//
// impl AsyncCalls {
//     fn on_tap(mut self: Weak<Self>) {
//         let spin = Spinner::lock();
//         on_back(async move {
//             sleep(Duration::from_millis(200)).await;
//
//             ok_main(move || {
//                 self.label.set_text("A");
//                 drop(spin);
//             })
//         });
//     }
// }
//
// impl Setup for AsyncCalls {
//     fn setup(mut self: Weak<Self>) {
//         self.button.place().tl(20).size(200, 100);
//         self.button.set_color(BLUE);
//         link_button!(self, button, on_tap);
//
//         self.label.place().same([Size, X], self.button).anchor(Top,
// self.button, 10);         self.label.set_text("Sopog");
//
//         self.table.place().below(self.label, 10);
//         self.table.set_data_source(self);
//     }
// }
//
// impl TableData for AsyncCalls {
//     fn setup_cell(self: Weak<Self>, cell: &mut dyn Any, index: usize) {
//         let cell = cell.downcast_mut::<Label>().unwrap();
//         cell.set_text(index);
//     }
// }
//
// pub async fn test_async_calls() -> Result<()> {
//     let view = UI::init_test_view::<AsyncCalls>();
//
//     inject_touches(
//         "
//             130  61   b
//             130  61   e
//         ",
//     )
//     .await;
//
//     sleep(Duration::from_millis(200)).await;
//
//     let text = from_main(move || view.label.text.clone()).await;
//
//     assert_eq!(text, "A");
//
//     Ok(())
// }
