use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Anchor, Label, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view},
    ui_test::{helpers::check_colors, inject_touches},
};

#[view]
pub struct Slider {
    #[init]
    slider: hilen::ui::Slider,
    label:  Label,
}

impl Setup for Slider {
    fn setup(self: Weak<Self>) {
        self.slider.set_color(WHITE).place().size(50, 400).center();
        self.slider.on_change.val(move |a| {
            self.label.set_text(a);
        });

        self.label
            .set_color(WHITE)
            .place()
            .size(100, 50)
            .center_y()
            .anchor(Anchor::Right, self.slider, 40);
    }
}

fn tap_sets_value(view: Weak<Slider>) {
    inject_touches(
        r"
            306  207  b
            306  207  e
    ",
    );

    assert!((view.slider.value() - 0.75).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "0.75");
}

fn drag_outside_does_not_change_value(view: Weak<Slider>) {
    inject_touches(
        r"
            177  137  m
            183  139  m
            195  138  m
            196  138  b
            197  139  m
            270  148  m
            290  148  m
            307  148  m
            304  160  m
            302  184  m
            301  234  m
            299  268  m
            292  315  m
            288  371  m
            290  409  m
            288  417  m
            195  448  m
            173  455  m
            173  455  e
            172  449  m
    ",
    );

    assert!((view.slider.value() - 0.75).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "0.75");
}

fn drag_sets_value(view: Weak<Slider>) {
    inject_touches(
        r"
            317  218  m
            303  208  m
            300  205  m
            300  205  b
            300  205  m
            325  208  m
            362  220  m
            378  240  m
            387  261  m
            381  292  m
            364  309  m
            342  324  m
            320  339  m
            299  357  m
            283  372  m
            269  395  m
            269  400  m
            274  420  m
            288  429  m
            298  429  m
            334  431  m
            359  431  m
            371  432  m
            378  433  e
            379  432  m
            465  391  m
            488  356  m
            455  444  m
            389  459  m
            416  449  m
            482  405  m
    ",
    );

    assert!((view.slider.value() - 0.145_161_3).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "0.15");
}

fn drag_below_bottom_clamps_to_zero(view: Weak<Slider>) {
    inject_touches(
        r"
            322  443  m
            312  438  m
            308  437  m
            308  438  b
            306  446  m
            299  464  m
            289  488  m
            273  512  m
            252  531  m
            248  537  m
            247  536  e
            248  535  m
            323  538  m
    ",
    );

    assert!(view.slider.value().abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "0.00");
}

fn drag_above_top_clamps_to_one(view: Weak<Slider>) {
    inject_touches(
        r"
            337  478  m
            306  475  m
            297  476  m
            298  477  m
            299  477  b
            322  458  m
            363  379  m
            382  230  m
            359  107  m
            316  46   m
            303  35   m
            304  37   e
            304  38   m
            435  184  m
            469  194  m
            477  177  m
    ",
    );

    assert!((view.slider.value() - 1.0).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "1.00");
}

fn set_range_updates_value(mut view: Weak<Slider>) {
    from_main(move || {
        view.slider.set_range(-5, 5);
    });

    assert!((view.slider.value() - 5.0).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "5.00");
}

fn drag_to_bottom_of_range(view: Weak<Slider>) {
    inject_touches(
        "
            301  136  b
            303  186  m
            307  313  m
            306  446  m
            304  507  m
            303  543  m
            303  542  e
        ",
    );

    assert!((view.slider.value() + 5.0).abs() < f32::EPSILON);
    assert_eq!(view.label.text(), "-5.00");
}

fn check_labels_at_indicator_positions(mut view: Weak<Slider>) -> Result<()> {
    for i in -5..=5 {
        from_main(move || {
            view.slider.set_value(i);
            let mut label = view.add_view::<Label>();
            label.set_text(i);
            label.set_color(WHITE);
            label.set_size(50, 20);
            label.set_x(340);
            // A whole pixel row, a fractional top edge blends differently
            // on every GPU.
            label.set_y((view.slider.indicator_position() - 10.0 + view.slider.y()).round());
        });
    }

    check_colors(
        r"
               4    4 - #597c95
             368  104 - #242424
             284  116 - #dfdfdf
             316  116 - #e7e7e7
             284  120 - #e2e2e2
             316  120 - #e9e9e9
             300  128 - #0762be
             360  156 - #585858
             360  196 - #010101
             296  224 - #0a84ff
             368  224 - #010101
             364  256 - #828282
             364  264 - #828282
             364  268 - #828282
             296  280 - #0a84ff
             160  288 - #303030
             212  292 - #010101
             196  300 - #9c9c9c
             348  300 - #ffffff
             360  300 - #4a4a4a
             372  300 - #d2d2d2
             380  336 - #ffffff
             276  348 - #ffffff
             356  376 - #8c8c8c
             340  412 - #ffffff
             276  440 - #ffffff
             372  440 - #000000
             368  452 - #484848
             372  476 - #242424
             356  488 - #8c8c8c
             364  492 - #000000
               4  592 - #597c95
            ",
    )
}

impl ViewTest for Slider {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        tap_sets_value(view);
        drag_outside_does_not_change_value(view);
        drag_sets_value(view);
        drag_below_bottom_clamps_to_zero(view);
        drag_above_top_clamps_to_one(view);
        set_range_updates_value(view);
        drag_to_bottom_of_range(view);
        check_labels_at_indicator_positions(view)?;

        Ok(())
    }
}
