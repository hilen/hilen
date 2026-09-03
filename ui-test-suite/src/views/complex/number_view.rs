use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Anchor::{Bot, Height, Left, Right, Width, X, Y},
        Color, Label, NumberView, Setup, Shadow, TextAlignment, ViewData, ViewTest, view,
    },
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

const BOX_TOP: Color = Color::rgb(1.0, 1.0, 1.0);
const BOX_BOTTOM: Color = Color::rgb(0.88, 0.91, 0.95);
const BORDER: Color = Color::rgb(0.80, 0.84, 0.90);
const TEXT: Color = Color::rgb(0.10, 0.12, 0.17);

#[view]
struct NumberTestView {
    #[init]
    float: NumberView,
    uint:  NumberView,
    int:   NumberView,

    float_label: Label,
    uint_label:  Label,
    int_label:   Label,
}

impl Setup for NumberTestView {
    fn setup(self: Weak<Self>) {
        fn attach_label(label: Weak<Label>, view: Weak<NumberView>) {
            // The stepper with chevrons in a bordered rounded box with a
            // light gradient and a shadow for volume, the line between
            // the halves in the border color, the value in a label under it.
            view.set_chevrons().set_separator_color(BORDER).set_bevel(BOX_TOP, BOX_BOTTOM);
            view.set_corner_radius(12)
                .set_border_width(1)
                .set_border_color(BORDER)
                .set_shadow(Shadow {
                    offset: (0, 2).into(),
                    radius: 6.0,
                    color:  Color::rgb(0.0, 0.0, 0.0).with_alpha(0.25),
                });
            label
                .set_text_color(TEXT)
                .set_text_size(22)
                .set_alignment(TextAlignment::Center);
            label.place().same([Width, X], view).h(50).anchor(Bot, view, 20);
            view.on_change(move |num| {
                label.set_text(num);
            });
        }

        self.float.place().tl(200).size(100, 200);
        attach_label(self.float_label, self.float);

        self.uint
            .place()
            .same([Width, Height, Y], self.float)
            .anchor(Left, self.float, 20);
        attach_label(self.uint_label, self.uint);

        self.int
            .place()
            .same([Width, Height, Y], self.float)
            .anchor(Right, self.float, 20);
        attach_label(self.int_label, self.int);
    }
}

const INITIAL_TAPS: &str = "
            379  244  b
            378  243  e
            378  243  b
            378  243  e
            378  243  b
            378  243  e
            378  243  b
            378  243  e
            378  243  b
            378  243  e
            378  243  b
            378  243  e
            378  243  b
            378  243  e
            248  371  b
            248  371  e
            248  371  b
            248  371  e
            248  371  b
            248  371  e
            248  371  b
            248  371  e
            248  371  b
            248  371  e
            248  371  b
            248  371  e
            248  371  b
            248  371  e
            112  234  b
            112  234  e
            112  234  b
            112  234  e
            112  234  b
            112  234  e
            112  234  b
            112  234  e
            112  234  b
            112  234  e
            112  234  b
            112  234  e
            112  234  b
            112  234  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            124  352  b
            124  352  e
            356  334  b
            355  334  e
            355  334  b
            355  334  e
            355  334  b
            355  334  e
            355  334  b
            355  334  e
            355  334  b
            355  334  e
            355  334  b
            355  334  e
            355  334  b
            355  334  e
            355  334  b
            355  334  e
            355  334  b
            355  334  e
            355  334  b
            355  334  e
            355  334  b
            355  334  e
            361  247  b
            361  247  e
            361  247  b
            361  247  e
            361  247  b
            361  247  e
            361  247  b
            361  247  e
            361  247  b
            361  247  e
            361  247  b
            361  247  e
            361  247  b
            361  247  e
            361  325  b
            361  325  e

        ";

fn tap_buttons_then_clamp_to_min(view: Weak<NumberTestView>) {
    inject_touches(INITIAL_TAPS);

    from_main(move || {
        assert!((view.float.value() + 6.0).abs() < f32::EPSILON);
        assert!((view.uint.value() - 3.0).abs() < f32::EPSILON);
        assert!((view.int.value() + 6.0).abs() < f32::EPSILON);

        view.float.set_min(-10.0);
        view.uint.set_min(2);
        view.int.set_min(-10);

        assert!((view.float.value() + 10.0).abs() < f32::EPSILON);
        assert!((view.uint.value() - 2.0).abs() < f32::EPSILON);
        assert!((view.int.value() + 10.0).abs() < f32::EPSILON);
    });
}

fn tap_up_to_five(view: Weak<NumberTestView>) {
    inject_touches(
        "
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            160  266  b
            160  266  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            230  255  b
            230  255  e
            285  345  b
            285  345  e
            363  234  b
            363  234  e
            363  234  b
            363  234  e
            363  234  b
            363  234  e

        ",
    );

    assert!((view.float.value() - 5.0).abs() < f32::EPSILON);
    assert!((view.uint.value() - 5.0).abs() < f32::EPSILON);
    assert!((view.int.value() - 5.0).abs() < f32::EPSILON);
}

const TAPS_BELOW_MIN: &str = "
            126  344  b
            126  344  e
            126  344  b
            126  344  e
            126  344  b
            126  344  e
            126  344  b
            126  344  e
            126  344  b
            126  344  e
            126  349  b
            126  349  e
            126  349  b
            126  349  e
            126  349  b
            126  349  e
            126  349  b
            126  349  e
            126  349  b
            126  349  e
            126  344  b
            126  344  e
            126  344  b
            126  344  e
            126  344  b
            126  344  e
            126  344  b
            126  344  e
            126  342  b
            126  342  e
            126  342  b
            126  342  e
            126  342  b
            126  342  e
            126  342  b
            126  342  e
            126  342  b
            126  342  e
            126  342  b
            126  342  e
            126  340  b
            126  340  e
            126  340  b
            126  340  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            126  339  b
            126  339  e
            257  374  b
            257  374  e
            257  374  b
            257  374  e
            257  374  b
            257  374  e
            257  374  b
            257  374  e
            257  374  b
            257  374  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  369  b
            257  369  e
            257  368  b
            257  368  e
            257  368  b
            257  368  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            257  366  b
            257  366  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e
            386  353  b
            386  353  e

        ";

fn tap_down_clamps_to_min(view: Weak<NumberTestView>) {
    inject_touches(TAPS_BELOW_MIN);

    assert!((view.float.value() + 10.0).abs() < f32::EPSILON);
    assert!((view.uint.value() - 2.0).abs() < f32::EPSILON);
    assert!((view.int.value() + 10.0).abs() < f32::EPSILON);
}

impl ViewTest for NumberTestView {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);
        tap_buttons_then_clamp_to_min(view);
        tap_up_to_five(view);
        tap_down_clamps_to_min(view);

        check_colors("")?;

        Ok(())
    }
}
