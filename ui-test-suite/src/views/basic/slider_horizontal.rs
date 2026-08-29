use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Anchor, Label, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view},
    ui_test::{helpers::check_colors, inject_touches},
};

/// The vertical slider laid on its side, the minimum at the left.
#[view]
pub struct SliderHorizontal {
    #[init]
    slider: hilen::ui::Slider,
    label:  Label,
}

impl Setup for SliderHorizontal {
    fn setup(mut self: Weak<Self>) {
        self.slider.set_horizontal().set_color(WHITE).place().size(400, 50).center();
        self.slider.on_change.val(move |a| {
            self.label.set_text(a);
        });

        self.label
            .set_color(WHITE)
            .place()
            .size(100, 50)
            .center_x()
            .anchor(Anchor::Top, self.slider, 40);
    }
}

fn tap_sets_value(view: Weak<SliderHorizontal>) {
    inject_touches(
        r"
            393  300  b
            393  300  e
    ",
    );

    assert!((view.slider.value() - 0.75).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "0.75");
}

fn drag_sets_value(view: Weak<SliderHorizontal>) {
    inject_touches(
        r"
            393  300  b
            350  302  m
            260  305  m
            207  301  m
            207  301  e
    ",
    );

    assert!((view.slider.value() - 0.25).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "0.25");
}

fn drag_past_left_clamps_to_zero(view: Weak<SliderHorizontal>) {
    inject_touches(
        r"
            207  301  b
            150  300  m
             60  300  m
             10  300  m
             10  300  e
    ",
    );

    assert!(view.slider.value().abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "0.00");
}

fn drag_past_right_clamps_to_one(view: Weak<SliderHorizontal>) {
    inject_touches(
        r"
            125  300  b
            300  300  m
            520  300  m
            590  300  m
            590  300  e
    ",
    );

    assert!((view.slider.value() - 1.0).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "1.00");
}

fn set_range_updates_value(mut view: Weak<SliderHorizontal>) {
    from_main(move || {
        view.slider.set_range(-5, 5);
    });

    assert!((view.slider.value() - 5.0).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "5.00");
}

fn drag_to_left_of_range(view: Weak<SliderHorizontal>) {
    inject_touches(
        r"
            470  300  b
            300  300  m
            130  300  m
            110  300  m
            110  300  e
    ",
    );

    assert!((view.slider.value() + 5.0).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "-5.00");
}

fn check_labels_at_indicator_positions(mut view: Weak<SliderHorizontal>) -> Result<()> {
    for i in -5..=5 {
        from_main(move || {
            view.slider.set_value(i);
            let mut label = view.add_view::<Label>();
            label.set_text(i);
            label.set_color(WHITE);
            label.set_size(30, 20);
            label.set_x(view.slider.indicator_position() - 15.0 + view.slider.x());
            label.set_y(240);
        });
    }

    check_colors(
        r"
             120  240 - #242424
             196  240 - #010101
             488  240 - #242424
             160  244 - #474747
             240  244 - #8ba4b5
             268  244 - #010101
             336  244 - #b6b6b6
             452  244 - #000000
             136  248 - #bdcbd5
             160  248 - #474747
             240  248 - #8ba4b5
             292  248 - #cecece
             304  248 - #464646
             364  248 - #ffffff
             396  248 - #7a96aa
             104  252 - #8c8c8c
             180  252 - #8c8c8c
             216  252 - #8c8c8c
             492  252 - #000000
             240  256 - #8ba4b5
             292  256 - #ffffff
             336  256 - #b6b6b6
             348  300 - #0a84ff
             424  300 - #0a84ff
             484  316 - #cfcfcf
             128  324 - #ffffff
             204  324 - #ffffff
             280  380 - #242424
             324  392 - #ffffff
             304  400 - #010101
               4  592 - #597c95
             592  592 - #597c95
            ",
    )
}

impl ViewTest for SliderHorizontal {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        tap_sets_value(view);
        drag_sets_value(view);
        drag_past_left_clamps_to_zero(view);
        drag_past_right_clamps_to_one(view);
        set_range_updates_value(view);
        drag_to_left_of_range(view);
        check_labels_at_indicator_positions(view)?;

        Ok(())
    }
}
