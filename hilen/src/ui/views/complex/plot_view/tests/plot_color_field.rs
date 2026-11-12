use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::Color,
    ui::{Label, PlotView, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, checkpoint, inject_touches, set_record_probe_count},
};

/// A color field under the grid: blue where x is below 0, red above, with
/// a dark circle of radius 2 around the origin, and a white curve over it.
/// A drag moves the field with the graph, and without the field the plot
/// shows its own background again.
#[view]
struct PlotColorField {
    #[init]
    status: Label,
    plot:   PlotView,
}

fn field(x: f64, y: f64) -> Color {
    if x.hypot(y) < 2.0 {
        Color::hex("#202830")
    } else if x < 0.0 {
        Color::hex("#2C7BB6")
    } else {
        Color::hex("#D7191C")
    }
}

impl Setup for PlotColorField {
    fn setup(self: Weak<Self>) {
        self.set_color("#FFFFFF");

        self.status
            .set_text("blue left of 0, red right, a dark disc in the middle")
            .set_text_size(20)
            .set_text_color("#1B1F24");
        self.status.place().lrt(0).h(44);

        self.plot.set_color("#FFFFFF");
        self.plot.place().t(52).lr(12).b(12);
        self.plot
            .set_x_range(-5.0, 5.0, 0.0)
            .set_color_field(field)
            .add_function(|x| x / 2.0, "#FFFFFF");
    }
}

impl ViewTest for PlotColorField {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        check_colors(CHECK_1)?;

        checkpoint("drag 100 points to the left, the field moves with the graph")?;
        inject_touches("300 300 b\n200 300 m\n200 300 e");
        check_colors(CHECK_2)?;

        from_main(move || {
            view.status.set_text("the field is removed, the white background is back");
            view.plot.remove_color_field();
        });
        check_colors(CHECK_3)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
     136   16 - #858789
     268   16 - #5a5d61
     492   16 - #6b6d70
     116   20 - #c5c6c7
     136   20 - #858789
     212   20 - #1c2025
     252   20 - #65686b
     396   20 - #a9abad
     468   20 - #383b3f
     504   20 - #ffffff
      92   24 - #1b1f24
     136   24 - #858789
     152   24 - #66696c
     176   24 - #1b1f24
     196   24 - #ffffff
     232   24 - #ffffff
     276   24 - #7a7c7f
     316   24 - #ffffff
     328   24 - #1d2126
     360   24 - #ffffff
     396   24 - #a9abad
     432   24 - #d5d6d7
     452   24 - #c1c2c4
     468   24 - #383b3f
     264   28 - #1b1f24
     592   92 - #ffffff
     300  112 - #5b6472
      96  116 - #2c7bb6
     440  120 - #d7191c
      12  156 - #2c7bb6
     324  208 - #4e242b
     104  224 - #2c7bb6
     216  244 - #202830
       4  256 - #ffffff
     408  272 - #d7191c
     408  276 - #d7191c
     300  316 - #5b6472
     464  320 - #5b6472
      72  332 - #4f6a83
     532  332 - #a23940
     192  364 - #2c7bb6
     404  364 - #202830
     192  368 - #2c7bb6
     256  424 - #202830
     344  424 - #202830
     584  428 - #d7191c
       4  448 - #ffffff
     476  460 - #d7191c
      88  508 - #2c7bb6
     300  524 - #5b6472
     592  536 - #ffffff
      12  584 - #2c7bb6
     388  584 - #d7191c
     496  584 - #d7191c
";

const CHECK_2: &str = r"
     136   16 - #858789
     268   16 - #5a5d61
     492   16 - #6b6d70
     136   20 - #858789
     212   20 - #1c2025
     252   20 - #65686b
     396   20 - #a9abad
     468   20 - #383b3f
     504   20 - #ffffff
      92   24 - #1b1f24
     116   24 - #c5c6c7
     136   24 - #858789
     152   24 - #66696c
     176   24 - #1b1f24
     196   24 - #ffffff
     232   24 - #ffffff
     276   24 - #7a7c7f
     316   24 - #ffffff
     328   24 - #1d2126
     360   24 - #ffffff
     396   24 - #a9abad
     432   24 - #d5d6d7
     452   24 - #c1c2c4
     468   24 - #383b3f
     264   28 - #1b1f24
       4   64 - #ffffff
     592   92 - #ffffff
     200  116 - #5b6472
     592  204 - #ffffff
     208  208 - #4d5662
     224  208 - #4e242b
     120  244 - #202830
     308  272 - #d7191c
     308  276 - #d7191c
      12  288 - #2c7bb6
     364  320 - #5b6472
     556  320 - #5b6472
     196  324 - #202830
     432  332 - #a23940
      92  364 - #2c7bb6
     304  364 - #202830
      92  368 - #2c7bb6
     244  424 - #202830
      12  432 - #2c7bb6
     364  432 - #d7191c
     508  460 - #d7191c
     128  484 - #2c7bb6
     200  508 - #5b6472
      12  540 - #2c7bb6
     392  552 - #d7191c
     196  584 - #2c7bb6
     484  584 - #d7191c
     584  584 - #d7191c
     300  592 - #ffffff
";

const CHECK_3: &str = r"
     136   16 - #1b1f24
     148   16 - #3d4045
     312   16 - #1f2328
     480   16 - #1b1f24
     512   16 - #1b1f24
      88   20 - #1b1f24
     156   20 - #c0c2c3
     176   20 - #b6b7b9
     236   20 - #ffffff
     264   20 - #1b1f24
     324   20 - #e0e0e1
     400   20 - #dadbdc
     428   20 - #1b1f24
     104   24 - #fafafa
     116   24 - #d4d5d6
     120   24 - #313539
     136   24 - #1b1f24
     156   24 - #c0c2c3
     176   24 - #b6b7b9
     188   24 - #fafafa
     200   24 - #6e7174
     208   24 - #64676a
     216   24 - #ffffff
     248   24 - #ffffff
     276   24 - #2c3034
     324   24 - #e0e0e1
     352   24 - #ffffff
     368   24 - #a1a3a5
     376   24 - #ffffff
     392   24 - #1b1f24
     400   24 - #dadbdc
     404   24 - #4d5054
     412   24 - #ffffff
     432   24 - #1b1f24
     440   24 - #bebfc0
     448   24 - #ffffff
     464   24 - #ffffff
     484   24 - #ffffff
     508   24 - #ffffff
     592  112 - #ffffff
     200  116 - #5b6472
       4  160 - #ffffff
     348  172 - #ffffff
     496  184 - #ffffff
     208  208 - #828893
      96  220 - #ffffff
     200  280 - #5b6472
      16  320 - #5b6472
     144  320 - #5b6472
     292  320 - #5b6472
     364  320 - #5b6472
     508  320 - #5b6472
     584  320 - #5b6472
     432  332 - #babdc3
     200  372 - #5b6472
     592  440 - #ffffff
     200  452 - #5b6472
     340  452 - #ffffff
      56  460 - #ffffff
     480  480 - #ffffff
     200  584 - #5b6472
       4  592 - #ffffff
     372  592 - #ffffff
     592  592 - #ffffff
";
