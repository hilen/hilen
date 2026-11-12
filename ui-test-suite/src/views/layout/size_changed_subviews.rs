use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{BLUE, Container, GREEN, Label, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::checkpoint,
};

/// A `size_changed` handler reads the size of a subview that its `place()`
/// rule sizes. That subview used to keep its old size inside the handler, 0 on
/// the first call, so a page that laid out its content there stayed blank.
#[view]
struct SizeChangedSubviews {
    child: Weak<Container>,
    seen:  Vec<f32>,

    #[init]
    status: Label,
    holder: Container,
}

impl Setup for SizeChangedSubviews {
    fn setup(mut self: Weak<Self>) {
        self.status.set_text_size(20).place().l(20).t(20).r(20).h(40);
        self.status.set_text("holder not sized yet");

        self.holder.set_color(GREEN);
        self.holder.place().l(20).t(80).w(200).h(200);

        self.child = self.holder.add_view();
        self.child.set_color(BLUE);
        self.child.place().all_sides(10);

        self.holder.size_changed().sub(move || {
            let width = self.child.width();
            self.seen.push(width);
            self.status.set_text(format!("blue width in size_changed: {width}"));
        });
    }
}

impl ViewTest for SizeChangedSubviews {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        let seen = from_main(move || view.seen.clone());
        ensure!(seen.first() == Some(&180.0), "first handler saw {seen:?}");
        checkpoint("blue width in size_changed is 180")?;

        from_main(move || {
            view.holder.place().clear().l(20).t(80).w(400).h(200);
        });
        settle();
        let seen = from_main(move || view.seen.clone());
        ensure!(
            seen.last() == Some(&380.0),
            "after the resize the handler saw {seen:?}"
        );
        checkpoint("holder is wider, blue width in size_changed is 380")?;

        Ok(())
    }
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}
