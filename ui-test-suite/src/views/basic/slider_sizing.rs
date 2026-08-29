use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Setup, Slider, ViewData, ViewTest, view},
    ui_test::{helpers::check_colors, human_checkpoint, inject_touches},
};

const TRACK: f32 = 16.0;
const THUMB: f32 = 22.0;

/// A horizontal and a vertical slider that start with the default
/// look, then get a thick track and then a big thumb, one step at a
/// time. The thumb radius also moves the value 0 and 1 stops, so a
/// touch at the thumb edge must land on the end after the change.
#[view]
pub struct SliderSizing {
    #[init]
    horizontal: Slider,
    vertical:   Slider,
}

impl Setup for SliderSizing {
    fn setup(mut self: Weak<Self>) {
        self.horizontal.set_horizontal().place().size(400, 60).center_x().t(100);
        self.horizontal.set_value(0.5);

        self.vertical.place().size(60, 300).center_x().t(220);
        self.vertical.set_value(0.5);
    }
}

fn default_look() -> Result<()> {
    human_checkpoint("default look, 8 point track, 14 point thumb");
    check_colors(
        r"
             592    4 - #597c95
             308  120 - #ffffff
             288  124 - #ffffff
             116  128 - #0a84ff
             340  128 - #d1d1d6
             480  128 - #d1d1d6
             160  132 - #0a84ff
             204  132 - #0a84ff
             248  132 - #0a84ff
             312  132 - #ffffff
             372  132 - #d1d1d6
             428  132 - #d1d1d6
             288  140 - #49657a
             292  144 - #49657a
             296  144 - #445e71
             300  144 - #425c6f
             304  144 - #456073
             300  264 - #d1d1d6
             296  312 - #d1d1d6
               8  348 - #597c95
             588  348 - #597c95
             308  360 - #ffffff
             292  364 - #ffffff
             304  372 - #ffffff
             288  380 - #49657a
             292  384 - #49657a
             300  384 - #0762be
             296  444 - #0a84ff
             468  472 - #597c95
             300  504 - #0a84ff
               4  592 - #597c95
             592  592 - #597c95
            ",
    )
}

fn thick_track(mut view: Weak<SliderSizing>) -> Result<()> {
    from_main(move || {
        view.horizontal.set_track_thickness(TRACK);
        view.vertical.set_track_thickness(TRACK);
    });
    human_checkpoint("track thickness 16");
    check_colors(
        r"
             592    4 - #597c95
             308  120 - #ffffff
             204  124 - #0a84ff
             356  124 - #d1d1d6
             452  124 - #d1d1d6
             116  132 - #0a84ff
             288  132 - #ffffff
             484  132 - #d1d1d6
             160  136 - #0a84ff
             244  136 - #0a84ff
             420  136 - #d1d1d6
             288  140 - #49657a
             292  144 - #49657a
             296  144 - #445e71
             300  144 - #425c6f
             304  144 - #456073
             300  252 - #d1d1d6
             292  308 - #d1d1d6
               4  348 - #597c95
             592  348 - #597c95
             308  360 - #ffffff
             292  364 - #ffffff
             312  372 - #ffffff
             288  380 - #49657a
             296  384 - #0865c2
             300  384 - #0762be
             304  384 - #0866c5
             292  444 - #0a84ff
             468  468 - #597c95
             300  504 - #0a84ff
               4  592 - #597c95
             592  592 - #597c95
            ",
    )
}

fn big_thumb(mut view: Weak<SliderSizing>) -> Result<()> {
    from_main(move || {
        view.horizontal.set_thumb_radius(THUMB);
        view.vertical.set_thumb_radius(THUMB);
        assert!((view.horizontal.indicator_position() - (THUMB + 356.0 * 0.5)).abs() < 0.01);
        assert!((view.vertical.indicator_position() - (THUMB + 256.0 * 0.5)).abs() < 0.01);
    });
    human_checkpoint("thumb radius 22");
    check_colors(
        r"
               4    4 - #597c95
             288  112 - #ffffff
             320  124 - #ffffff
             472  124 - #d1d1d6
             124  128 - #0a84ff
             172  132 - #0a84ff
             300  132 - #ffffff
             356  132 - #d1d1d6
             396  132 - #d1d1d6
             220  136 - #0a84ff
             436  136 - #d1d1d6
             320  140 - #49667a
             284  148 - #49667b
             292  152 - #466276
             300  152 - #425c6f
             304  152 - #445e71
             300  264 - #d1d1d6
             296  308 - #d1d1d6
               4  344 - #597c95
             588  344 - #597c95
             288  352 - #ffffff
             312  356 - #ffffff
             320  380 - #49667a
             284  388 - #49667b
             296  392 - #0864c1
             300  392 - #0762be
             304  392 - #0864c2
             292  444 - #0a84ff
             468  468 - #597c95
             300  496 - #0a84ff
               4  592 - #597c95
             592  592 - #597c95
            ",
    )
}

fn touch_at_thumb_edge_hits_the_end(view: Weak<SliderSizing>) {
    // The horizontal slider spans x 100 to 500, its thumb stops sit THUMB in.
    inject_touches(
        r"
            478  130  b
            478  130  e
    ",
    );
    assert!((view.horizontal.value() - 1.0).abs() < f32::EPSILON);

    inject_touches(
        r"
            122  130  b
            122  130  e
    ",
    );
    assert!(view.horizontal.value().abs() < f32::EPSILON);

    // The vertical slider spans y 220 to 520.
    inject_touches(
        r"
            300  498  b
            300  498  e
    ",
    );
    assert!(view.vertical.value().abs() < f32::EPSILON);

    inject_touches(
        r"
            300  242  b
            300  242  e
    ",
    );
    assert!((view.vertical.value() - 1.0).abs() < f32::EPSILON);
}

impl ViewTest for SliderSizing {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        default_look()?;
        thick_track(view)?;
        big_thumb(view)?;
        touch_at_thumb_edge_hits_the_end(view);
        Ok(())
    }
}
