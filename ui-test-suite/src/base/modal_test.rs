use anyhow::Result;
use test_engine::{
    OnceEvent,
    refs::Weak,
    ui::{
        Color, Container, Label, ModalView, Setup, Size, ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE,
        WeakView, ui_test::helpers::check_colors, view,
    },
};

#[view]
struct ShowModally {}

impl Setup for ShowModally {
    fn setup(self: Weak<Self>) {
        let mut view = WeakView::default();

        for _ in 0..200 {
            if view.is_ok() {
                view = view.add_view::<Container>();
                view.set_color(Color::random()).place().all_sides(1);
            } else {
                view = self.add_view::<Container>();
                view.set_color(Color::random()).place().tl(1).size(400, 400);
                assert!((view.z_position() - 0.499_969_87).abs() < f32::EPSILON);
            }
        }

        assert!((view.z_position() - 0.497_977_17).abs() < f32::EPSILON);
    }
}

#[view]
struct Modal {
    event: OnceEvent,

    #[init]
    label: Label,
}

impl Setup for Modal {
    fn setup(self: Weak<Self>) {
        self.label.place().back();
        self.label.set_text_size(100);
        self.label.set_text("Hello");
        self.label.set_color(WHITE);
    }
}

impl ModalView for Modal {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (400, 400).into()
    }
}

impl ViewTest for ShowModally {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        Modal::show_modally_with_input((), |()| {});

        check_colors(
            r"
             592    4 - #597c95
             104  104 - #ffffff
             432  104 - #ffffff
             268  108 - #ffffff
             484  204 - #ffffff
             592  232 - #597c95
             200  260 - #000000
             344  260 - #000000
             272  284 - #000000
             284  284 - #ffffff
             288  284 - #ffffff
             300  284 - #000000
             372  284 - #000000
             400  284 - #010101
             236  288 - #000000
             200  292 - #000000
             404  312 - #010101
             300  316 - #000000
             272  320 - #000000
             296  320 - #000000
             372  320 - #010101
             196  324 - #000000
             324  324 - #000000
             588  380 - #597c95
               4  404 - #597c95
             240  432 - #ffffff
             348  472 - #ffffff
             132  480 - #ffffff
             496  496 - #ffffff
               4  592 - #597c95
             260  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
