use hilen::{
    refs::Weak,
    ui::{
        Container, LIGHT_BLUE, NoImage, NumberView, Setup, Style, UIManager, ViewData, ViewSubviews,
        ViewTest, view,
    },
    ui_test::check_colors,
};

use crate::interface::HAS_BACK_BUTTON;

const CORNER_STYLE: Style = Style::new(|v| {
    v.set_color(LIGHT_BLUE).place().size(80, 80);
});

#[view]
pub struct RootLayoutView {
    #[init]
    scale: NumberView,
}

impl Setup for RootLayoutView {
    fn setup(self: Weak<Self>) {
        UIManager::enable_debug_frames();
        UIManager::root_view().set_image("square.png");

        self.apply_style(HAS_BACK_BUTTON);

        self.add_view::<Container>().apply_style(CORNER_STYLE).place().tl(0);
        self.add_view::<Container>().apply_style(CORNER_STYLE).place().tr(0);
        self.add_view::<Container>().apply_style(CORNER_STYLE).place().br(0);
        self.add_view::<Container>().apply_style(CORNER_STYLE).place().bl(0);

        self.add_view::<Container>().apply_style(CORNER_STYLE).place().t(0).center_x();
        self.add_view::<Container>().apply_style(CORNER_STYLE).place().l(0).center_y();
        self.add_view::<Container>().apply_style(CORNER_STYLE).place().r(0).center_y();
        self.add_view::<Container>().apply_style(CORNER_STYLE).place().b(0).center_x();

        self.scale
            .set_min(0.2)
            .set_step(0.1)
            .set_value(1)
            .place()
            .center()
            .size(100, 200);
        self.scale.on_change(|scale| {
            UIManager::set_scale(scale);
        });
    }
}

impl Drop for RootLayoutView {
    fn drop(&mut self) {
        UIManager::disable_debug_frames();
        UIManager::root_view().set_image(NoImage);
    }
}

impl ViewTest for RootLayoutView {
    fn perform_test(_view: Weak<Self>) -> anyhow::Result<()> {
        check_colors(
            r"
             100    4 - #4958b7
             448    4 - #715ee8
               4   76 - #00daff
             592   80 - #6842da
             184   84 - #5a61d9
             300   88 - #655fd2
             500  172 - #5526b8
             212  196 - #6c5ce6
               4  200 - #5d5fc7
              56  220 - #010101
              28  224 - #000000
              76  228 - #ffffff
              28  232 - #010101
              52  232 - #ffffff
             292  236 - #ffffff
             344  240 - #0096e6
             252  276 - #0096e6
             508  300 - #5e19b0
             324  332 - #ffffff
             248  368 - #7a4bed
             484  404 - #832adb
             592  432 - #6501b7
               4  448 - #876afa
             140  448 - #7b4fe7
             416  488 - #7926da
             592  508 - #6700c7
             520  512 - #6c08c7
             352  560 - #6300c1
              80  588 - #6e30d1
             236  592 - #7325cc
             444  592 - #6b01cb
             592  592 - #00daff
            ",
        )?;

        // hilen::ui_test::record_ui_test();

        Ok(())
    }
}
