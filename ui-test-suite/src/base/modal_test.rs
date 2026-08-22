use anyhow::Result;
use hilen::{
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
             592  148 - #597c95
             592  152 - #597c95
             592  156 - #597c95
             592  208 - #597c95
             592  212 - #597c95
             196  264 - #000000
             372  288 - #000000
             284  292 - #ffffff
             300  300 - #010101
             260  308 - #dddddd
             364  308 - #000000
             408  308 - #000000
             408  316 - #010101
             372  324 - #000000
             496  448 - #ffffff
             492  460 - #ffffff
             192  496 - #ffffff
             300  496 - #ffffff
             324  496 - #ffffff
             340  496 - #ffffff
             496  496 - #ffffff
              20  592 - #597c95
              44  592 - #597c95
             108  592 - #597c95
             132  592 - #597c95
             160  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
