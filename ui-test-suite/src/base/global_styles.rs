use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Anchor::{Top, X},
        BLUE, Button, Setup, Style, ViewData, ViewSubviews, ViewTest, view,
    },
    ui_test::{UITest, check_colors},
};

const GLOBAL_STYLE: Style = Style::new(|view| {
    view.set_color((175, 129, 115));
    view.set_corner_radius(20);

    if let Some(view) = view.downcast_view::<Button>() {
        view.set_text_color(BLUE);
        view.set_text_size(55);
    }
});

#[view]
struct GlobalStyles {
    #[init]
    button_1: Button,
    button_2: Button,
    button_3: Button,
}

impl Setup for GlobalStyles {
    fn setup(self: Weak<Self>) {
        self.button_1.set_text("Button 1").place().t(50).l(50).size(200, 50);

        self.button_2.set_text("Button 2");
        self.button_2
            .place()
            .anchor(Top, self.button_1, 40)
            .same_size(self.button_1)
            .same([X], self.button_1);

        self.button_3.set_text("Button 3");
        self.button_3
            .place()
            .anchor(Top, self.button_2, 40)
            .same_size(self.button_1)
            .same([X], self.button_1);
    }
}

impl ViewTest for GlobalStyles {
    fn before_start() {
        from_main(|| {
            GLOBAL_STYLE.apply_globally::<Button>();
        });
    }

    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(
            r"
             592    4 - #597c95
              60   60 - #af8173
             104   64 - #826091
             184   64 - #826091
             124   68 - #4e3ab3
              84   80 - #89658c
             248   80 - #af8173
              60   84 - #af8173
             160   92 - #0101e7
             176  140 - #af8173
             124  148 - #af8173
             244  148 - #0000e7
              84  156 - #89658c
              52  160 - #0101e7
              84  172 - #89658c
             120  180 - #0000e7
             204  180 - #1510d9
             248  180 - #0000e7
             152  184 - #af8173
             232  236 - #0101e7
              56  240 - #4e3ab3
             104  244 - #826091
             184  244 - #826091
             132  248 - #4e3ab3
              56  260 - #4e3ab3
              84  260 - #89658c
             164  260 - #af8173
             248  264 - #0101e7
             208  276 - #af8173
             544  300 - #597c95
             112  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        from_main(|| {
            GLOBAL_STYLE.reset_global::<Button>();
        });

        UITest::reload::<GlobalStyles>();

        check_colors(
            r"
             248   52 - #ffffff
              96   68 - #ffffff
             180   72 - #000000
             136   76 - #ffffff
             168   76 - #9e9e9e
              92   80 - #9e9e9e
             144   80 - #dadada
             160   80 - #ffffff
             200   80 - #8a8a8a
             116   84 - #000000
              52   96 - #ffffff
             220  140 - #ffffff
             100  156 - #ffffff
             144  156 - #dadada
              92  164 - #9e9e9e
             168  164 - #9e9e9e
             144  168 - #dadada
              92  172 - #9e9e9e
             124  172 - #000000
             248  188 - #ffffff
             592  212 - #597c95
              96  244 - #000000
             132  248 - #000000
             180  252 - #000000
             144  256 - #dadada
              92  260 - #9e9e9e
             168  260 - #9e9e9e
             116  264 - #000000
             200  264 - #000000
             300  544 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
