use ui_proc::view;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::flat::Point,
    ui::{ImageView, Setup, Touch, UIImages, ViewData, ViewFrame, ViewTouch},
};

#[view]
pub struct StickView {
    pub on_change: Event<Point>,
    pub flaccid:   bool,

    #[init]
    background:      ImageView,
    direction_stick: ImageView,
}

impl StickView {
    fn on_touch_moved(&mut self, touch: Point) {
        let max_length = self.frame().size.height / 2.0;
        let center = self.frame().size.center();

        let vector = (touch - center).trimmed(max_length);

        let frame = *self.frame();

        self.direction_stick.set_center(vector + frame.size.center());

        self.on_change.trigger(vector * 0.1);
    }

    fn on_touch(&mut self, touch: &Touch) {
        if touch.is_ended() {
            if self.flaccid {
                return;
            }
            let frame = *self.frame();
            self.direction_stick.set_center(frame.size.center());
            self.on_change.trigger(Point::default());
        } else {
            self.on_touch_moved(touch.position);
        }
    }
}

impl Setup for StickView {
    fn setup(mut self: Weak<Self>) {
        self.enable_touch();
        self.touch().all.val(move |touch| {
            self.direction_stick.place().clear().relative_size(self, 0.5);
            self.on_touch(&touch);
        });

        self.background.set_image(UIImages::joystick());
        self.direction_stick.set_image(UIImages::handle());

        self.background.place().back();

        let _center = self.frame().size.center();

        self.direction_stick.place().relative_size(self, 0.5).center();

        // self.direction_stick
        //     .set_frame((0, 0, STICK_VIEW_SIZE, STICK_VIEW_SIZE))
        //     .set_center(center);
        //
        // self.chan
    }
}

#[cfg(feature = "ui-tests")]
mod test {
    use anyhow::Result;

    use crate::{
        deps::refs::Weak,
        gm::flat::Point,
        ui::{Setup, StickView, ViewData, ViewTest, view},
        ui_test::inject_touches,
    };

    #[view]
    struct StickViewTest {
        point: Point,

        #[init]
        stick: StickView,
    }

    impl Setup for StickViewTest {
        fn setup(mut self: Weak<Self>) {
            self.stick.place().tl(50).size(280, 280);
            self.stick.on_change.val(move |point| {
                self.point += point;
            });
        }
    }

    impl ViewTest for StickViewTest {
        fn perform_test(view: Weak<Self>) -> Result<()> {
            inject_touches(TOUCHES_1);

            assert_eq!(view.point, Point::new(-0.084_445_12, 49.880_524));

            // record_ui_test();

            Ok(())
        }
    }

    const TOUCHES_1: &str = "
    204  189  m
    167  129  m
    197  193  m
    197  196  m
    197  189  m
    197  189  b
    234  178  m
    219  226  m
    183  243  m
    169  207  m
    200  153  m
    234  188  m
    163  262  m
    108  167  m
    177  128  m
    201  172  m
    165  214  m
    56   175  m
    70   90   m
    126  67   m
    183  99   m
    233  188  m
    199  246  m
    136  246  m
    130  124  m
    217  128  m
    224  143  e
    294  268  m
    344  440  m
    342  438  m
    342  437  b
    341  437  e
    290  423  m
    182  413  m
    180  413  b
    180  413  e
    179  353  m
    112  144  m
    83   96   m
    48   28   m
    43   12   b
    43   12   e
    85   16   m
    272  56   m
    358  81   m
    362  83   b
    363  84   e
    349  88   m
    284  93   m
    270  96   m
    261  104  m
    253  112  b
    253  112  m
    253  112  e
    162  112  m
    132  109  m
    132  109  b
    132  109  e
    119  182  m
    111  200  b
    111  200  m
    111  200  e
    111  267  m
    116  287  m
    116  287  b
    116  288  e
    194  288  m
    231  291  m
    234  291  b
    234  291  e
    237  277  m
    263  221  m
    264  216  b
    264  216  e
    263  212  m
    223  168  m
    215  158  b
    214  158  m
    214  158  e
    199  135  m
    189  129  m
    189  129  b
    189  129  e
    188  167  m
    186  192  m
    185  194  b
    193  184  m
    210  215  m
    12   214  m
    201  163  m
    201  299  m
    224  149  m
    259  297  m
    90   202  m
    234  88   m
    241  260  m
    90   282  m
    39   162  m
    83   87   m
    144  57   m
    202  48   m
    267  59   m
    290  74   m
    333  140  m
    348  259  m
    347  471  m
    290  559  m
    225  558  m
    141  470  m
    82   321  m
    64   206  m
    83   119  m
    138  61   m
    256  47   m
    318  100  m
    363  222  m
    382  302  m
    410  386  m
    336  451  m
    251  447  m
    191  419  m
    159  391  m
    170  378  m
    195  379  m
    195  379  e
    377  378  m
    ";
}
