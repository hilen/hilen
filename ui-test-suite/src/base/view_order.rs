use anyhow::Result;
use test_engine::{
    refs::Weak,
    ui::{BLACK, BLUE, Container, GREEN, RED, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::helpers::check_colors,
};

#[view]
pub struct ViewOrder {
    #[init]
    view_1: Container,
    view_2: Container,
    view_3: Container,
    view_4: Container,
}

impl Setup for ViewOrder {
    fn setup(self: Weak<Self>) {
        self.view_1.set_color(RED).place().size(200, 200);
        self.view_2.set_color(GREEN).place().size(200, 200).tl(100);
        self.view_3.set_color(BLUE).place().size(200, 200).tl(200);
        self.view_4.set_color(BLACK).place().size(200, 200).tl(300);
    }
}

impl ViewTest for ViewOrder {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        assert_eq!(
            view.dump_subviews(),
            vec![
                "ViewOrder.view_1: Container".to_string(),
                "ViewOrder.view_2: Container".to_string(),
                "ViewOrder.view_3: Container".to_string(),
                "ViewOrder.view_4: Container".to_string()
            ]
        );

        assert_eq!(view.view_1.view_label(), "ViewOrder.view_1: Container");
        assert_eq!(view.view_2.view_label(), "ViewOrder.view_2: Container");
        assert_eq!(view.view_3.view_label(), "ViewOrder.view_3: Container");
        assert_eq!(view.view_4.view_label(), "ViewOrder.view_4: Container");

        assert_eq!(view.subviews()[0].label(), view.view_1.view_label());
        assert_eq!(view.subviews()[1].label(), view.view_2.view_label());
        assert_eq!(view.subviews()[2].label(), view.view_3.view_label());
        assert_eq!(view.subviews()[3].label(), view.view_4.view_label());

        check_colors(
            r"
               4    4 - #ff0000
             392    4 - #597c95
             592    4 - #597c95
             196   32 - #ff0000
              96   48 - #ff0000
             156   92 - #ff0000
              12  100 - #ff0000
             296  104 - #00ff00
             212  140 - #00ff00
             104  148 - #00ff00
             592  180 - #597c95
               4  196 - #ff0000
             240  196 - #00ff00
             304  196 - #597c95
             396  212 - #0000e7
             160  220 - #00ff00
             224  288 - #0000e7
             104  296 - #00ff00
             356  296 - #0000e7
             468  304 - #000000
             288  324 - #0000e7
             204  368 - #0000e7
             496  388 - #000000
             280  396 - #0000e7
             408  396 - #000000
               4  428 - #597c95
             344  428 - #000000
             432  464 - #000000
             364  496 - #000000
             496  496 - #000000
               4  592 - #597c95
             196  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
