use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::{
        Apply,
        color::{Color, TURQUOISE},
    },
    ui::{Container, ScrollView, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_scroll, inject_touches},
};

#[view]
struct ScrollViewTest {
    #[init]
    scroll: ScrollView,
}

impl Setup for ScrollViewTest {
    fn setup(mut self: Weak<Self>) {
        self.scroll.set_content_size((600, 600));
        self.scroll.place().back();
        add_corners(self.scroll, TURQUOISE);
    }
}

impl ViewTest for ScrollViewTest {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        check_colors(COLORS_1)?;

        assert!(view.scroll.content_offset().abs() < f32::EPSILON);

        inject_scroll(-5);
        assert!(view.scroll.content_offset().abs() < f32::EPSILON);

        inject_scroll(-20);
        assert!(view.scroll.content_offset().abs() < f32::EPSILON);

        inject_scroll(-30);
        assert!(view.scroll.content_offset().abs() < f32::EPSILON);

        check_colors(COLORS_2)?;

        from_main(move || {
            view.scroll.set_content_size((400, 400));
        });

        check_colors(COLORS_3)?;

        from_main(move || {
            view.scroll.set_content_size((600, 800));
        });

        check_colors(COLORS_4)?;

        inject_touches("258  176  b");
        inject_touches("258  176  m");
        inject_touches("258  176  e");

        inject_scroll(-150);
        assert!((view.scroll.get_scroll_content_offset() + 150.0).abs() < f32::EPSILON);

        check_colors(COLORS_5)?;

        inject_scroll(-1500);
        assert!((view.scroll.get_scroll_content_offset() + 200.0).abs() < f32::EPSILON);

        check_colors(COLORS_6)?;

        from_main(move || {
            view.scroll.set_content_offset(-400.0);
        });

        check_colors(COLORS_7)?;

        // crate::ui::ui_test::record_ui_test();

        Ok(())
    }
}

fn add_corners(view: Weak<ScrollView>, color: Color) {
    let v1 = view.add_view::<Container>();
    let v2 = view.add_view::<Container>();
    let v3 = view.add_view::<Container>();
    let v4 = view.add_view::<Container>();

    [v1, v2, v3, v4].apply(|a| {
        a.place().size(100, 100);
        a.set_color(color);
    });

    v1.place().tl(0);
    v2.place().tr(0);
    v3.place().bl(0);
    v4.place().br(0);
}

const COLORS_1: &str = r"
    4    4 - #00ffff
    96    4 - #00ffff
    284    4 - #597c95
    496    4 - #597c95
    588    4 - #00ffff
    52   52 - #00ffff
    504   92 - #00ffff
    4   96 - #00ffff
    96   96 - #00ffff
    592   96 - #00ffff
    240  156 - #597c95
    404  156 - #597c95
    592  212 - #597c95
    104  220 - #597c95
    8  296 - #597c95
    516  296 - #597c95
    300  300 - #597c95
    152  332 - #597c95
    592  380 - #597c95
    412  440 - #597c95
    244  448 - #597c95
    592  496 - #597c95
    8  504 - #00ffff
    96  504 - #00ffff
    504  504 - #00ffff
    52  548 - #00ffff
    592  588 - #00ffff
    4  592 - #00ffff
    96  592 - #00ffff
    184  592 - #597c95
    312  592 - #597c95
    504  592 - #00ffff
";

const COLORS_2: &str = r"
    4    4 - #00ffff
    96    4 - #00ffff
    284    4 - #597c95
    496    4 - #597c95
    588    4 - #00ffff
    52   52 - #00ffff
    504   92 - #00ffff
    4   96 - #00ffff
    96   96 - #00ffff
    592   96 - #00ffff
    240  156 - #597c95
    404  156 - #597c95
    592  212 - #597c95
    104  220 - #597c95
    8  296 - #597c95
    516  296 - #597c95
    300  300 - #597c95
    152  332 - #597c95
    592  380 - #597c95
    412  440 - #597c95
    244  448 - #597c95
    592  496 - #597c95
    8  504 - #00ffff
    96  504 - #00ffff
    504  504 - #00ffff
    52  548 - #00ffff
    592  588 - #00ffff
    4  592 - #00ffff
    96  592 - #00ffff
    184  592 - #597c95
    312  592 - #597c95
    504  592 - #00ffff
";

const COLORS_3: &str = r"
    4    4 - #00ffff
    96    4 - #00ffff
    304    4 - #00ffff
    396    4 - #00ffff
    592   32 - #597c95
    348   48 - #00ffff
    200   52 - #597c95
    392   92 - #00ffff
    4   96 - #00ffff
    96   96 - #00ffff
    304   96 - #00ffff
    452  156 - #597c95
    592  232 - #597c95
    156  240 - #597c95
    300  296 - #597c95
    12  304 - #00ffff
    392  304 - #00ffff
    96  316 - #00ffff
    508  316 - #597c95
    4  368 - #00ffff
    304  388 - #00ffff
    60  396 - #00ffff
    396  396 - #00ffff
    576  412 - #597c95
    160  424 - #597c95
    480  472 - #597c95
    324  504 - #597c95
    128  548 - #597c95
    420  568 - #597c95
    4  592 - #597c95
    248  592 - #597c95
    592  592 - #597c95
";

const COLORS_4: &str = r"
    4    4 - #00ffff
    96    4 - #00ffff
    312    4 - #597c95
    592    4 - #00ffff
    48    8 - #00ffff
    504    8 - #00ffff
    8   48 - #00ffff
    52   52 - #00ffff
    96   52 - #00ffff
    504   52 - #00ffff
    548   52 - #00ffff
    4   96 - #00ffff
    52   96 - #00ffff
    96   96 - #00ffff
    504   96 - #00ffff
    548   96 - #00ffff
    592   96 - #00ffff
    224  140 - #597c95
    360  156 - #597c95
    144  252 - #597c95
    444  276 - #597c95
    300  300 - #597c95
    592  300 - #597c95
    4  336 - #597c95
    296  444 - #597c95
    444  444 - #597c95
    588  448 - #597c95
    152  464 - #597c95
    448  588 - #597c95
    4  592 - #597c95
    300  592 - #597c95
    592  592 - #597c95
";

const COLORS_5: &str = r"
    4    4 - #597c95
    444    4 - #597c95
    592    4 - #597c95
    296    8 - #597c95
    148   12 - #597c95
    24  144 - #597c95
    576  144 - #597c95
    436  152 - #597c95
    164  156 - #597c95
    300  160 - #597c95
    12  284 - #597c95
    588  284 - #597c95
    300  300 - #597c95
    152  388 - #597c95
    448  388 - #597c95
    300  508 - #597c95
    4  552 - #00ffff
    40  552 - #00ffff
    96  552 - #00ffff
    504  552 - #00ffff
    556  552 - #00ffff
    592  552 - #00ffff
    68  560 - #00ffff
    528  564 - #00ffff
    504  580 - #00ffff
    36  584 - #00ffff
    96  584 - #00ffff
    560  584 - #00ffff
    4  592 - #00ffff
    68  592 - #00ffff
    528  592 - #00ffff
    592  592 - #00ffff
";

const COLORS_6: &str = r"
    4    4 - #597c95
    444    4 - #597c95
    592    4 - #597c95
    296    8 - #597c95
    148   12 - #597c95
    556  132 - #597c95
    36  136 - #597c95
    424  152 - #597c95
    168  156 - #597c95
    584  260 - #597c95
    4  264 - #597c95
    300  300 - #597c95
    144  348 - #597c95
    452  352 - #597c95
    256  448 - #597c95
    8  504 - #00ffff
    52  504 - #00ffff
    96  504 - #00ffff
    504  504 - #00ffff
    548  504 - #00ffff
    592  504 - #00ffff
    8  548 - #00ffff
    52  548 - #00ffff
    96  548 - #00ffff
    504  548 - #00ffff
    548  548 - #00ffff
    592  548 - #00ffff
    4  592 - #00ffff
    96  592 - #00ffff
    300  592 - #597c95
    504  592 - #00ffff
    592  592 - #00ffff
";

const COLORS_7: &str = r"
    4    4 - #597c95
    444    4 - #597c95
    592    4 - #597c95
    296    8 - #597c95
    148   12 - #597c95
    556  132 - #597c95
    36  136 - #597c95
    424  152 - #597c95
    168  156 - #597c95
    584  260 - #597c95
    4  264 - #597c95
    300  300 - #597c95
    144  348 - #597c95
    452  352 - #597c95
    256  448 - #597c95
    8  504 - #00ffff
    52  504 - #00ffff
    96  504 - #00ffff
    504  504 - #00ffff
    548  504 - #00ffff
    592  504 - #00ffff
    8  548 - #00ffff
    52  548 - #00ffff
    96  548 - #00ffff
    504  548 - #00ffff
    548  548 - #00ffff
    592  548 - #00ffff
    4  592 - #00ffff
    96  592 - #00ffff
    300  592 - #597c95
    504  592 - #00ffff
    592  592 - #00ffff
";
