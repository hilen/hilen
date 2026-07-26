use anyhow::Result;
use test_engine::{
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
             452    4 - #597c95
             164   52 - #af8173
             212   52 - #af8173
              76   56 - #0000e7
             128   56 - #af8173
             248   80 - #af8173
              80   88 - #af8173
             160   88 - #0000e7
             120   92 - #af8173
             196   96 - #af8173
             132  148 - #af8173
             228  148 - #0202e7
             168  156 - #0202e7
              64  164 - #af8173
             248  164 - #af8173
             592  168 - #597c95
             108  176 - #0202e7
             200  176 - #0000e7
             148  180 - #af8173
              56  232 - #0000e7
             136  236 - #0000e7
             244  236 - #0000e7
             108  244 - #0000e7
             200  248 - #0000e7
              96  268 - #0202e7
             140  268 - #0000e7
             172  268 - #af8173
             220  276 - #af8173
             480  380 - #597c95
             300  536 - #597c95
               4  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        from_main(|| {
            GLOBAL_STYLE.reset_global::<Button>();
        });

        UITest::reload::<GlobalStyles>();

        check_colors(
            r"
             592    4 - #597c95
              52   52 - #ffffff
             196   52 - #ffffff
             248   52 - #ffffff
             100   68 - #ffffff
             136   72 - #ffffff
             156   76 - #ffffff
              96   96 - #ffffff
             212   96 - #ffffff
             436  116 - #597c95
              52  144 - #ffffff
             100  156 - #ffffff
             248  156 - #ffffff
             136  164 - #ffffff
             156  164 - #ffffff
             592  164 - #597c95
             100  168 - #ffffff
             204  188 - #ffffff
             100  248 - #ffffff
             200  248 - #ffffff
             156  252 - #ffffff
             136  256 - #ffffff
              52  276 - #ffffff
             248  276 - #ffffff
             504  300 - #597c95
             368  360 - #597c95
             128  448 - #597c95
             464  472 - #597c95
             300  532 - #597c95
               4  592 - #597c95
             164  592 - #597c95
             592  592 - #597c95
        ",
        )?;

        Ok(())
    }
}
