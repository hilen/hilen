use test_engine::{
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
             188    4 - #5e67de
             304    4 - #00daff
             436    4 - #6d5ae0
             592    8 - #00daff
               4   32 - #00daff
             536   88 - #795ce7
             108   96 - #6271e0
             312  116 - #7e6cf1
             456  144 - #785ee9
             592  168 - #8447e2
             336  204 - #0096e6
             260  212 - #0096e6
              32  216 - #ffffff
              76  224 - #ffffff
              36  228 - #ffffff
              52  228 - #ffffff
             284  268 - #ffffff
             344  280 - #0096e6
             492  288 - #743bde
             204  304 - #865dee
             244  304 - #865dee
             320  332 - #ffffff
             244  344 - #865dee
             300  372 - #ffffff
             120  396 - #593cca
             592  396 - #7b1fd7
               4  444 - #8266f2
             376  504 - #781ad5
             584  516 - #6700c7
              92  592 - #6e30d1
             264  592 - #00daff
             488  592 - #8010da
            ",
        )?;

        // test_engine::ui_test::record_ui_test();

        Ok(())
    }
}
