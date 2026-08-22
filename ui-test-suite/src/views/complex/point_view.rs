use anyhow::Result;
use hilen::{
    refs::{Own, Weak},
    ui::{Label, Point, PointView, Setup, ViewData, ViewTest, view},
    ui_test::inject_touches,
};
use log::debug;

#[view]
struct PointTestView {
    point: Own<Point>,

    #[init]
    point_view: PointView,
    label:      Label,
}

impl Setup for PointTestView {
    fn setup(mut self: Weak<Self>) {
        self.point_view.place().size(200, 200).tl(100);

        self.point_view.changed.val(move |p| {
            self.label.set_text(p);
            *self.point += p;
        });

        self.label.place().size(400, 50).br(5);
    }
}

impl ViewTest for PointTestView {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches(
            r"
            261  272  b
            261  272  e
            262  269  b
            262  269  e
            261  269  b
            261  269  e
            261  269  b
            261  269  e
            260  269  b
            260  269  e
            259  269  b
            259  269  e
            259  269  b
            259  269  e
            259  269  b
            259  269  e
            143  187  b
            143  187  e
            143  187  b
            143  187  e
            143  187  b
            143  187  e
            142  187  b
            142  187  e
            141  187  b
            141  187  e
            141  187  b
            141  187  e
            141  187  b
            141  187  e
            141  187  b
            141  187  e
            141  187  b
            141  187  e
            141  187  b
            141  187  e
            141  187  b
            141  187  e
            141  187  b
            141  187  e
            141  187  b
            141  187  e
            141  187  b
            141  187  e
            141  187  b
            141  187  e
        ",
        );

        assert_eq!(view.point_view.point(), Point::new(16.0, -7.0));
        assert_eq!(view.point, Point::new(143.0, -133.0));

        debug!("Point view: OK");

        Ok(())
    }
}
