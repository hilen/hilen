use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Point, PositionView, Setup, ViewTest, view},
    ui_test::inject_touches,
};
use log::debug;

#[view]
struct PositionViewTest {
    #[init]
    pos: PositionView,
}

impl Setup for PositionViewTest {
    fn setup(self: Weak<Self>) {}
}

impl ViewTest for PositionViewTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches(
            "
            174  35   b
            402  369  m
            402  369  e
        ",
        );

        assert_eq!(view.pos.position, Point::new(228.0, 334.0));

        inject_touches(
            "
            350  374  b
            460  162  m
            460  162  e

        ",
        );

        assert_eq!(view.pos.position, Point::new(338.0, 122.0));

        inject_touches(
            "
            512  146  b
            144  540  m
            144  540  e
        ",
        );

        assert_eq!(view.pos.position, Point::new(-30.0, 516.0));

        debug!("Position view: OK");

        Ok(())
    }
}
