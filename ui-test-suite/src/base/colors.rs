use anyhow::Result;
use test_engine::{
    gm::Apply,
    refs::Weak,
    ui::{
        Anchor::{Height, Left, Top, Width, X},
        Container, ImageView, Setup, ViewData, ViewTest, WHITE, view,
    },
    ui_test::check_colors,
};

#[view]
struct Colors {
    #[init]
    image: ImageView,

    _1: Container,
    _2: Container,
    _3: Container,
    _4: Container,
}

impl Setup for Colors {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        self.image.place().tl(20).size(280, 520);
        self.image.set_image("colors.png");

        self._1.set_color((45, 70, 149));
        self._1.place().size(100, 100).t(45).anchor(Left, self.image, 20);

        [self._2, self._3, self._4].apply(|view| {
            view.place().same([Width, Height, X], self._1);
        });

        self._2.set_color((48, 48, 48));
        self._2.place().anchor(Top, self._1, 25);

        self._3.set_color((124, 190, 22));
        self._3.place().anchor(Top, self._2, 25);

        self._4.set_color((172, 71, 212));
        self._4.place().anchor(Top, self._3, 25);
    }
}

impl ViewTest for Colors {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             592    4 - #ffffff
             192   48 - #2d4695
             396   52 - #2d4695
              56   92 - #ffffff
             276   92 - #2d4695
             348  108 - #2d4695
             180  128 - #2d4695
             368  176 - #303030
             240  180 - #303030
             584  196 - #ffffff
              64  228 - #ffffff
             372  244 - #303030
             260  256 - #303030
             184  268 - #303030
             324  296 - #7cbe16
             416  296 - #7cbe16
             216  328 - #7cbe16
             116  348 - #ffffff
             116  352 - #ffffff
             272  368 - #7cbe16
             592  388 - #ffffff
             376  392 - #7cbe16
             192  396 - #7cbe16
               4  444 - #ffffff
             336  448 - #ac47d4
             260  452 - #ac47d4
             180  484 - #ac47d4
             416  484 - #ac47d4
             324  516 - #ac47d4
             240  520 - #ac47d4
               4  592 - #ffffff
             592  592 - #ffffff
        ",
        )?;

        Ok(())
    }
}
