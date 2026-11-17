use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    ui::{Container, Label, PlotView, ScrollView, Setup, ViewData, ViewSubviews, ViewTest, ViewTouch, view},
    ui_test::{check_colors, checkpoint, inject_scroll, inject_touches, set_record_probe_count},
};

/// A plot with its input off, in a cell of a list. The wheel over it
/// scrolls the list and a tap on it is a tap of the cell, the graph does
/// not move. With its input on again the wheel zooms the graph and the
/// list stays.
#[view]
struct PlotInList {
    taps: u32,
    plot: Weak<PlotView>,

    #[init]
    status: Label,
    scroll: ScrollView,
}

impl Setup for PlotInList {
    fn setup(mut self: Weak<Self>) {
        self.set_color("#FFFFFF");

        self.status.set_text_size(18).set_text_color("#1B1F24");
        self.status.place().lrt(0).h(44);

        self.scroll.set_color("#F4F5F7");
        self.scroll.place().t(52).lr(12).b(12);
        self.scroll.set_content_height(1200);

        let cell = self.scroll.add_view::<Container>();
        cell.set_color("#DCE3EC").set_corner_radius(8);
        cell.place().l(12).t(12).size(420, 280);
        cell.enable_touch();
        cell.touch().up_inside.sub(self, move || {
            self.taps += 1;
            self.show_status();
        });

        let name = cell.add_view::<Label>();
        name.set_text("cell, a tap counts").set_text_size(14).set_text_color("#1B1F24");
        name.place().lrt(8).h(24);

        let plot = cell.add_view::<PlotView>();
        plot.set_color("#FFFFFF");
        plot.place().lr(8).t(36).b(8);
        plot.set_x_range(-5.0, 5.0, 0.0).add_function(f64::sin, "#1F5FBF");
        self.plot = plot.set_interactive(false);

        let below = self.scroll.add_view::<Label>();
        below
            .set_text("list content below the cell")
            .set_text_size(14)
            .set_text_color("#5B6472");
        below.place().l(12).t(320).size(300, 24);

        self.show_status();
    }
}

impl PlotInList {
    fn show_status(self: Weak<Self>) {
        let (left, right) = self.plot.x_range();
        self.status.set_text(format!(
            "cell taps {}   plot x {left:.2} to {right:.2}",
            self.taps
        ));
    }
}

fn range_is(range: (f64, f64), left: f64, right: f64) -> bool {
    (range.0 - left).abs() < 1e-6 && (range.1 - right).abs() < 1e-6
}

impl ViewTest for PlotInList {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);
        check_colors(CHECK_1)?;

        checkpoint("the wheel over the plot scrolls the list, the graph stays")?;
        inject_touches("200 220 m");
        inject_scroll(-60);
        let offset = from_main(move || view.scroll.get_scroll_content_offset());
        assert!(offset.abs() > 1.0, "the list did not scroll, offset {offset}");
        let range = from_main(move || view.plot.x_range());
        assert!(range_is(range, -5.0, 5.0), "{range:?}");
        check_colors(CHECK_2)?;

        checkpoint("a tap on the plot is a tap of the cell")?;
        inject_touches("200 200 b\n200 200 e");
        assert_eq!(from_main(move || view.taps), 1);
        let range = from_main(move || view.plot.x_range());
        assert!(range_is(range, -5.0, 5.0), "{range:?}");
        check_colors(CHECK_3)?;

        checkpoint("input on again, the wheel zooms the graph and the list stays")?;
        from_main(move || {
            view.plot.set_interactive(true);
            view.show_status();
        });
        inject_touches("200 200 m");
        let offset = from_main(move || view.scroll.get_scroll_content_offset());
        inject_scroll(60);
        let after = from_main(move || view.scroll.get_scroll_content_offset());
        assert!((after - offset).abs() < 0.5, "the list moved {offset} {after}");
        let range = from_main(move || view.plot.x_range());
        assert!(range.1 - range.0 < 9.0, "the graph did not zoom {range:?}");
        from_main(move || view.show_status());
        check_colors(CHECK_4)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
     204   16 - #a3a5a7
     284   16 - #cbcccd
     332   16 - #303439
     392   16 - #303439
     204   20 - #a3a5a7
     256   20 - #ffffff
     284   20 - #cbcccd
     300   20 - #1b1f24
     348   20 - #ffffff
     408   20 - #ffffff
     192   24 - #ffffff
     204   24 - #a3a5a7
     212   24 - #373a3f
     224   24 - #66696c
     228   24 - #1b1f24
     284   24 - #cbcccd
     292   24 - #ffffff
     300   24 - #1b1f24
     356   24 - #ffffff
     368   24 - #4d5054
     380   24 - #1b1f24
     416   24 - #ffffff
     584   56 - #9f9fa1
      32   64 - #dce3ec
     200   80 - #afb6be
     184   84 - #dce3ec
     192   84 - #6d7379
     200   84 - #afb6be
     236   84 - #dce3ec
     248   84 - #dce3ec
     260   84 - #353a3f
     268   84 - #1b1f24
     276   84 - #32363c
     408   84 - #dce3ec
     220   88 - #dce3ec
     584   92 - #9f9fa1
     104   96 - #dce3ec
     348   96 - #dce3ec
     584  128 - #9f9fa1
     440  132 - #dce3ec
     240  136 - #f8f9fa
     584  168 - #9f9fa1
      60  180 - #1f5fbf
     280  180 - #1f5fbf
     332  192 - #1f5fbf
      84  196 - #1f5fbf
     256  196 - #1f5fbf
     440  196 - #dce3ec
      88  200 - #1f5fbf
     252  200 - #1f5fbf
      84  208 - #ffffff
     584  208 - #9f9fa1
      72  220 - #e3e6ec
      76  228 - #dadcdf
     384  228 - #ffffff
     584  228 - #9f9fa1
     380  236 - #1f5fbf
     384  240 - #1f5fbf
     136  244 - #1f5fbf
     204  244 - #1f5fbf
     396  248 - #1f5fbf
     584  248 - #9f9fa1
     160  256 - #1f5fbf
     184  256 - #1f5fbf
     412  256 - #1f5fbf
     584  268 - #9f9fa1
     584  288 - #9f9fa1
      24  312 - #dce3ec
     440  336 - #dce3ec
     344  340 - #dce3ec
      96  380 - #5b6472
     168  380 - #cfd2d7
     184  380 - #656e7b
     248  380 - #838a95
      96  384 - #5b6472
     100  384 - #a2a7b0
     120  384 - #f4f5f7
     128  384 - #f4f5f7
     148  384 - #9ca2ab
     168  384 - #cfd2d7
     184  384 - #656e7b
     212  384 - #f4f5f7
     224  384 - #9ca2ab
     240  384 - #5b6472
     248  384 - #838a95
     592  400 - #ffffff
     492  444 - #f4f5f7
     364  464 - #f4f5f7
       4  468 - #ffffff
     180  564 - #f4f5f7
     472  564 - #f4f5f7
       4  592 - #ffffff
     356  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_2: &str = r"
     520    4 - #ffffff
     204   16 - #a3a5a7
     284   16 - #cbcccd
     332   16 - #303439
     392   16 - #303439
     200   20 - #e8e8e9
     204   20 - #a3a5a7
     256   20 - #ffffff
     284   20 - #cbcccd
     300   20 - #1b1f24
     348   20 - #ffffff
     416   20 - #ffffff
     184   24 - #ffffff
     200   24 - #e8e8e9
     204   24 - #a3a5a7
     212   24 - #373a3f
     224   24 - #66696c
     228   24 - #1b1f24
     284   24 - #cbcccd
     292   24 - #ffffff
     300   24 - #1b1f24
     332   24 - #ffffff
     356   24 - #ffffff
     368   24 - #4d5054
     380   24 - #1b1f24
     392   24 - #ffffff
      28   52 - #dce3ec
     240   76 - #f8f9fa
     584   84 - #9f9fa1
     440  108 - #dce3ec
     584  116 - #9f9fa1
      60  120 - #1f5fbf
     280  120 - #1f5fbf
     320  124 - #1f5fbf
     332  132 - #1f5fbf
      84  136 - #1f5fbf
     256  136 - #1f5fbf
      88  140 - #1f5fbf
     252  140 - #1f5fbf
     584  144 - #9f9fa1
      84  148 - #ffffff
      72  160 - #e3e6ec
      72  164 - #e3e6ec
      76  168 - #dadcdf
     384  168 - #ffffff
     584  168 - #9f9fa1
     380  176 - #1f5fbf
     384  180 - #1f5fbf
     136  184 - #1f5fbf
     204  184 - #1f5fbf
     396  188 - #1f5fbf
      24  192 - #dce3ec
     584  192 - #9f9fa1
     156  196 - #1f5fbf
     180  196 - #1f5fbf
     412  196 - #1f5fbf
     584  216 - #9f9fa1
      72  224 - #e3e6ec
     584  240 - #9f9fa1
      24  252 - #dce3ec
     584  264 - #9f9fa1
      72  276 - #dce3ec
     140  276 - #dce3ec
     284  276 - #dce3ec
     440  276 - #dce3ec
     344  280 - #dce3ec
     584  288 - #9f9fa1
     584  312 - #9f9fa1
      96  320 - #5b6472
     168  320 - #cfd2d7
     184  320 - #656e7b
     248  320 - #838a95
      96  324 - #5b6472
     100  324 - #a2a7b0
     120  324 - #f4f5f7
     128  324 - #f4f5f7
     148  324 - #9ca2ab
     168  324 - #cfd2d7
     172  324 - #f4f5f7
     184  324 - #656e7b
     212  324 - #f4f5f7
     224  324 - #9ca2ab
     236  324 - #f4f5f7
     240  324 - #5b6472
     248  324 - #838a95
       8  444 - #ffffff
     444  448 - #f4f5f7
     304  452 - #f4f5f7
     588  452 - #ffffff
     160  504 - #f4f5f7
     452  584 - #f4f5f7
       4  592 - #ffffff
     316  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_3: &str = r"
     524    4 - #ffffff
     204   16 - #a3a5a7
     284   16 - #cbcccd
     332   16 - #303439
     392   16 - #303439
     200   20 - #e8e8e9
     204   20 - #a3a5a7
     256   20 - #d1d1d2
     284   20 - #cbcccd
     300   20 - #1b1f24
     348   20 - #ffffff
     408   20 - #ffffff
     184   24 - #ffffff
     200   24 - #e8e8e9
     204   24 - #a3a5a7
     212   24 - #373a3f
     224   24 - #66696c
     228   24 - #1b1f24
     256   24 - #d1d1d2
     284   24 - #cbcccd
     292   24 - #ffffff
     300   24 - #1b1f24
     332   24 - #ffffff
     356   24 - #ffffff
     368   24 - #4d5054
     380   24 - #1b1f24
     392   24 - #ffffff
     416   24 - #ffffff
      28   52 - #dce3ec
     240   76 - #f8f9fa
     584   96 - #9f9fa1
     440  104 - #dce3ec
      60  120 - #1f5fbf
     280  120 - #1f5fbf
     320  124 - #1f5fbf
     584  124 - #9f9fa1
     332  132 - #1f5fbf
      84  136 - #1f5fbf
     256  136 - #1f5fbf
      88  140 - #1f5fbf
     252  140 - #1f5fbf
      84  148 - #ffffff
     584  152 - #9f9fa1
      72  160 - #e3e6ec
      72  164 - #e3e6ec
      76  168 - #dadcdf
     384  168 - #ffffff
     380  176 - #1f5fbf
     384  180 - #1f5fbf
     584  180 - #9f9fa1
     136  184 - #1f5fbf
     204  184 - #1f5fbf
      24  188 - #dce3ec
     396  188 - #1f5fbf
     156  196 - #1f5fbf
     180  196 - #1f5fbf
     412  196 - #1f5fbf
     584  204 - #9f9fa1
     440  220 - #dce3ec
     584  232 - #9f9fa1
      24  244 - #dce3ec
     584  260 - #9f9fa1
      72  272 - #e3e6ec
     284  276 - #dce3ec
     440  276 - #dce3ec
     344  280 - #dce3ec
     584  288 - #9f9fa1
     584  312 - #9f9fa1
      96  320 - #5b6472
     168  320 - #cfd2d7
     184  320 - #656e7b
     248  320 - #838a95
      96  324 - #5b6472
     100  324 - #a2a7b0
     120  324 - #f4f5f7
     128  324 - #f4f5f7
     148  324 - #9ca2ab
     168  324 - #cfd2d7
     172  324 - #f4f5f7
     184  324 - #656e7b
     212  324 - #f4f5f7
     224  324 - #9ca2ab
     236  324 - #f4f5f7
     240  324 - #5b6472
     248  324 - #838a95
       8  444 - #ffffff
     444  448 - #f4f5f7
     304  452 - #f4f5f7
     588  452 - #ffffff
     160  504 - #f4f5f7
     452  584 - #f4f5f7
       4  592 - #ffffff
     316  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_4: &str = r"
     204   16 - #a3a5a7
     284   16 - #cbcccd
     348   16 - #303439
     200   20 - #e8e8e9
     204   20 - #a3a5a7
     256   20 - #d1d1d2
     284   20 - #cbcccd
     300   20 - #1b1f24
     352   20 - #ffffff
     404   20 - #ffffff
     184   24 - #ffffff
     192   24 - #ffffff
     200   24 - #e8e8e9
     204   24 - #a3a5a7
     212   24 - #373a3f
     224   24 - #66696c
     228   24 - #1b1f24
     256   24 - #d1d1d2
     284   24 - #cbcccd
     292   24 - #ffffff
     300   24 - #1b1f24
     332   24 - #1b1f24
     348   24 - #ffffff
     356   24 - #ffffff
     368   24 - #4d5054
     380   24 - #1b1f24
     392   24 - #1b1f24
     416   24 - #1b1f24
      28   52 - #dce3ec
     244   60 - #ffffff
     584   96 - #9f9fa1
     440  104 - #dce3ec
      36  108 - #1f5fbf
     320  108 - #1f5fbf
     280  116 - #1f5fbf
     340  116 - #1f5fbf
      64  124 - #1f5fbf
     268  124 - #1f5fbf
     584  124 - #9f9fa1
     356  128 - #1f5fbf
     360  132 - #1f5fbf
     364  136 - #1f5fbf
     368  140 - #1f5fbf
      36  152 - #5b6472
      80  152 - #5b6472
     124  152 - #5b6472
     172  152 - #5b6472
     244  152 - #5b6472
     284  152 - #5b6472
     316  152 - #5b6472
     432  152 - #5b6472
     584  152 - #9f9fa1
     116  172 - #1f5fbf
     212  176 - #1f5fbf
     124  180 - #1f5fbf
     208  180 - #1f5fbf
     416  184 - #1f5fbf
     152  196 - #1f5fbf
     176  196 - #1f5fbf
     584  204 - #9f9fa1
     440  232 - #dce3ec
     584  232 - #9f9fa1
     248  248 - #dfe1e3
      24  252 - #dce3ec
     584  260 - #9f9fa1
      72  276 - #dce3ec
     292  280 - #dce3ec
     348  280 - #dce3ec
     408  280 - #dce3ec
     584  288 - #9f9fa1
     584  312 - #9f9fa1
      96  320 - #5b6472
     168  320 - #cfd2d7
     184  320 - #656e7b
     248  320 - #838a95
      96  324 - #5b6472
     100  324 - #a2a7b0
     120  324 - #f4f5f7
     128  324 - #f4f5f7
     148  324 - #9ca2ab
     168  324 - #cfd2d7
     172  324 - #f4f5f7
     184  324 - #656e7b
     212  324 - #f4f5f7
     224  324 - #9ca2ab
     236  324 - #f4f5f7
     240  324 - #5b6472
     248  324 - #838a95
       8  444 - #ffffff
     444  448 - #f4f5f7
     304  452 - #f4f5f7
     588  452 - #ffffff
     160  504 - #f4f5f7
       4  592 - #ffffff
     316  592 - #ffffff
     592  592 - #ffffff
";
