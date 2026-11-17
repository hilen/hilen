use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    ui::{Label, PlotView, Setup, View, ViewData, ViewFrame, ViewTest, view},
    ui_test::{check_colors, checkpoint, inject_scroll, inject_touches, set_record_probe_count},
};

/// A plot with a blue parabola, a red sine and a green point list over
/// the grid, the axes and the numbers. A drag pans the graph and the
/// wheel zooms at the cursor, each time the functions are sampled again.
#[view]
struct PlotPanZoom {
    #[init]
    status: Label,
    plot:   PlotView,
}

impl Setup for PlotPanZoom {
    fn setup(self: Weak<Self>) {
        self.set_color("#FFFFFF");

        self.status.set_text_size(20).set_text_color("#1B1F24");
        self.status.place().lrt(0).h(44);

        self.plot.set_color("#FFFFFF").set_border_color("#9AA4B2").set_border_width(2);
        self.plot.place().t(52).lr(12).b(12);
        self.plot
            .set_x_range(-5.0, 5.0, 0.0)
            .add_function(|x| x * x / 2.0 - 3.0, "#1F5FBF")
            .add_function(f64::sin, "#D7263D")
            .add_points([(-4.0, 3.0), (-2.0, 4.0), (1.0, 2.5), (3.0, 4.5)], "#2D8A3E");

        self.plot.on_change.sub(move || self.show_status());
        self.show_status();
    }
}

impl PlotPanZoom {
    fn show_status(self: Weak<Self>) {
        let (left, right) = self.plot.x_range();
        self.status.set_text(format!(
            "blue parabola, red sine, green points   x {left:.2} to {right:.2}"
        ));
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

impl ViewTest for PlotPanZoom {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        let range = from_main(move || view.plot.x_range());
        assert!(close(range.0, -5.0) && close(range.1, 5.0), "{range:?}");
        check_colors(CHECK_1)?;

        checkpoint("drag 100 points to the right, the graph follows the finger")?;
        let width = from_main(move || f64::from(view.plot.width()));
        inject_touches("300 300 b\n400 300 m\n400 300 e");
        let shift = 100.0 * 10.0 / width;
        let range = from_main(move || view.plot.x_range());
        assert!(
            close(range.0, -5.0 - shift) && close(range.1, 5.0 - shift),
            "{range:?}"
        );
        check_colors(CHECK_2)?;

        checkpoint("wheel at the cursor zooms in, the point under it stays")?;
        inject_touches("300 300 m");
        let cursor = from_main(move || view.plot.__base_view().local_point((300, 300).into()));
        let before = from_main(move || view.plot.graph_point(cursor));
        inject_scroll(60);
        let after = from_main(move || view.plot.graph_point(cursor));
        assert!(
            close(before.0, after.0) && close(before.1, after.1),
            "{before:?} {after:?}"
        );
        let range = from_main(move || view.plot.x_range());
        assert!(close(range.1 - range.0, 10.0 / 0.12_f64.exp()), "{range:?}");
        check_colors(CHECK_3)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
      72   16 - #1b1f24
      84   16 - #6d7073
     176   16 - #4e5155
     224   16 - #adaeb0
     432   16 - #c6c7c8
      84   20 - #6d7073
     136   20 - #44474c
     176   20 - #4e5155
     200   20 - #cdcecf
     244   20 - #a9abad
     264   20 - #ffffff
     288   20 - #adaeb0
     336   20 - #1b1f24
     428   20 - #1f2328
     456   20 - #ffffff
     496   20 - #1b1f24
     520   20 - #1c2025
      76   24 - #ffffff
      84   24 - #6d7073
     116   24 - #ffffff
     136   24 - #44474c
     156   24 - #ffffff
     176   24 - #4e5155
     188   24 - #1b1f24
     200   24 - #cdcecf
     212   24 - #fafafa
     244   24 - #a9abad
     256   24 - #1b1f24
     280   24 - #1b1f24
     288   24 - #adaeb0
     312   24 - #fafafa
     348   24 - #ffffff
     364   24 - #bfc0c1
     380   24 - #ffffff
     476   24 - #1b1f24
     500   24 - #1b1f24
     192   28 - #1b1f24
      32   52 - #9aa4b2
     528   52 - #9aa4b2
     584   52 - #9aa4b2
      76   56 - #1f5fbf
     472   60 - #2d8a3e
     300   68 - #5b6472
     456   76 - #2d8a3e
      80   88 - #ffffff
     444   88 - #2d8a3e
     520   88 - #ffffff
      12  100 - #9aa4b2
     432  100 - #2d8a3e
     300  112 - #5b6472
     420  112 - #2d8a3e
     412  120 - #2d8a3e
     400  132 - #2d8a3e
      12  144 - #9aa4b2
     388  144 - #2d8a3e
     500  144 - #1f5fbf
     100  148 - #1f5fbf
     376  156 - #2d8a3e
     300  160 - #5b6472
     212  164 - #ffffff
     364  168 - #2d8a3e
     592  172 - #ffffff
      12  184 - #9aa4b2
     116  200 - #1f5fbf
     308  208 - #828893
     480  208 - #1f5fbf
      12  224 - #9aa4b2
     552  244 - #ffffff
     132  248 - #1f5fbf
     468  248 - #1f5fbf
     300  256 - #5b6472
      12  264 - #d7263d
     408  264 - #d7263d
      56  268 - #d7263d
     364  268 - #d7263d
     348  276 - #d7263d
     436  280 - #d7263d
      80  284 - #d7263d
     148  292 - #1f5fbf
     328  292 - #d7263d
     452  292 - #d7263d
     320  300 - #d7263d
     460  300 - #d7263d
      16  320 - #5b6472
     204  320 - #5b6472
     380  320 - #5b6472
     584  320 - #5b6472
      72  332 - #858c96
     532  332 - #babdc3
     140  340 - #d7263d
     260  356 - #d7263d
     424  356 - #1f5fbf
     520  356 - #d7263d
     164  360 - #d7263d
     548  372 - #d7263d
     196  376 - #d7263d
     212  376 - #d7263d
     224  376 - #d7263d
     564  376 - #d7263d
     584  376 - #d7263d
      12  384 - #9aa4b2
      96  408 - #ffffff
     204  412 - #1f5fbf
     396  412 - #1f5fbf
     300  416 - #5b6472
     228  448 - #1f5fbf
      12  452 - #9aa4b2
     232  452 - #1f5fbf
     368  452 - #1f5fbf
     364  456 - #1f5fbf
     356  464 - #1f5fbf
     504  468 - #ffffff
     348  472 - #1f5fbf
     256  476 - #1f5fbf
     120  484 - #ffffff
     288  492 - #1f5fbf
     300  492 - #1f5fbf
     308  492 - #1f5fbf
     592  496 - #ffffff
      12  516 - #9aa4b2
     524  548 - #ffffff
     448  576 - #ffffff
      12  584 - #9aa4b2
     300  584 - #5b6472
     156  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_2: &str = r"
      72   16 - #1b1f24
      84   16 - #6d7073
     176   16 - #4e5155
     224   16 - #adaeb0
     460   16 - #474a4e
      84   20 - #6d7073
     136   20 - #44474c
     176   20 - #4e5155
     200   20 - #cdcecf
     244   20 - #a9abad
     288   20 - #adaeb0
     336   20 - #1b1f24
     460   20 - #474a4e
      72   24 - #1b1f24
      84   24 - #6d7073
     104   24 - #fafafa
     132   24 - #1c2025
     136   24 - #44474c
     156   24 - #ffffff
     176   24 - #4e5155
     188   24 - #1b1f24
     200   24 - #cdcecf
     212   24 - #fafafa
     244   24 - #a9abad
     256   24 - #1b1f24
     280   24 - #1b1f24
     288   24 - #adaeb0
     312   24 - #fafafa
     348   24 - #ffffff
     364   24 - #bfc0c1
     380   24 - #ffffff
     428   24 - #ffffff
     456   24 - #1f2327
     476   24 - #1b1f24
     524   24 - #ffffff
     192   28 - #1b1f24
     512   28 - #494d51
     516   28 - #494d51
     572   60 - #2d8a3e
      12   72 - #9aa4b2
     556   76 - #2d8a3e
     180   88 - #ffffff
     544   88 - #2d8a3e
     532  100 - #2d8a3e
     400  108 - #5b6472
      12  112 - #9aa4b2
     520  112 - #2d8a3e
     508  124 - #2d8a3e
     496  136 - #2d8a3e
     200  148 - #1f5fbf
      12  152 - #9aa4b2
     476  156 - #2d8a3e
     308  160 - #ffffff
     400  160 - #5b6472
     468  164 - #2d8a3e
     108  172 - #ffffff
     460  172 - #2d8a3e
     212  188 - #1f5fbf
      12  196 - #9aa4b2
     584  196 - #1f5fbf
     408  208 - #828893
     224  224 - #1f5fbf
      12  236 - #9aa4b2
     568  244 - #1f5fbf
     116  264 - #d7263d
     508  264 - #d7263d
     100  268 - #d7263d
     156  268 - #d7263d
     464  268 - #d7263d
     240  272 - #1f5fbf
     448  276 - #d7263d
     536  280 - #d7263d
      12  284 - #9aa4b2
      76  284 - #d7263d
     180  284 - #d7263d
     428  292 - #d7263d
     552  292 - #d7263d
     420  300 - #d7263d
     560  300 - #d7263d
     256  312 - #1f5fbf
      52  316 - #ffffff
     116  320 - #5b6472
     304  320 - #5b6472
     356  320 - #5b6472
     492  320 - #5b6472
      24  324 - #ffffff
      12  332 - #9aa4b2
     172  332 - #858c96
     536  332 - #1f5fbf
     240  340 - #d7263d
      12  344 - #d7263d
     248  348 - #d7263d
     360  356 - #d7263d
     264  360 - #d7263d
     400  360 - #5b6472
     296  376 - #d7263d
     308  376 - #d7263d
     320  376 - #d7263d
     516  376 - #1f5fbf
      12  392 - #9aa4b2
     496  412 - #1f5fbf
     308  420 - #1f5fbf
     400  420 - #5b6472
     328  448 - #1f5fbf
     332  452 - #1f5fbf
     468  452 - #1f5fbf
      12  456 - #9aa4b2
     464  456 - #1f5fbf
     460  460 - #1f5fbf
     148  468 - #ffffff
     448  472 - #1f5fbf
     592  472 - #ffffff
     356  476 - #1f5fbf
     388  492 - #1f5fbf
     396  492 - #1f5fbf
     408  492 - #1f5fbf
     244  500 - #ffffff
     400  508 - #5b6472
      12  520 - #9aa4b2
     400  556 - #5b6472
     108  560 - #ffffff
      12  584 - #9aa4b2
     316  584 - #ffffff
     204  592 - #ffffff
     432  592 - #ffffff
     560  592 - #ffffff
";

const CHECK_3: &str = r"
      72   16 - #1b1f24
      84   16 - #6d7073
     176   16 - #4e5155
     224   16 - #adaeb0
     456   16 - #f1f1f1
      84   20 - #6d7073
     136   20 - #44474c
     176   20 - #4e5155
     200   20 - #cdcecf
     244   20 - #a9abad
     288   20 - #adaeb0
     292   20 - #1b1f24
     524   20 - #ffffff
      76   24 - #ffffff
      84   24 - #6d7073
     136   24 - #44474c
     156   24 - #ffffff
     176   24 - #4e5155
     188   24 - #1b1f24
     200   24 - #cdcecf
     256   24 - #1b1f24
     280   24 - #1b1f24
     288   24 - #adaeb0
     292   24 - #1f2328
     312   24 - #fafafa
     336   24 - #1b1f24
     364   24 - #bfc0c1
     380   24 - #ffffff
     428   24 - #ffffff
     476   24 - #1b1f24
     192   28 - #1b1f24
     496   28 - #494d51
      20   52 - #9aa4b2
     112   52 - #9aa4b2
     160   52 - #9aa4b2
     580   52 - #9aa4b2
      12   60 - #9aa4b2
     272   68 - #2d8a3e
     412   68 - #5b6472
     256   76 - #2d8a3e
     240   84 - #2d8a3e
     224   92 - #2d8a3e
     208  100 - #2d8a3e
     200  104 - #2d8a3e
     192  108 - #2d8a3e
     184  112 - #2d8a3e
     412  112 - #5b6472
      12  120 - #9aa4b2
     160  124 - #2d8a3e
     592  140 - #ffffff
     412  156 - #5b6472
     504  160 - #ffffff
     200  172 - #1f5fbf
     308  172 - #ffffff
      12  180 - #9aa4b2
     420  196 - #727985
     412  204 - #5b6472
     212  208 - #1f5fbf
      12  240 - #9aa4b2
     224  244 - #1f5fbf
     420  260 - #757c88
     328  264 - #ffffff
      68  268 - #d7263d
     144  268 - #d7263d
     476  268 - #d7263d
     552  268 - #d7263d
     168  284 - #d7263d
      40  288 - #d7263d
     172  288 - #d7263d
     240  288 - #1f5fbf
     448  288 - #d7263d
     580  288 - #d7263d
      36  292 - #d7263d
     444  292 - #d7263d
     412  296 - #5b6472
     252  320 - #1f5fbf
     576  320 - #ffffff
     412  328 - #5b6472
      20  332 - #ffffff
     240  352 - #d7263d
     244  356 - #d7263d
     376  356 - #d7263d
     248  360 - #d7263d
     372  360 - #d7263d
     116  364 - #ffffff
      12  368 - #9aa4b2
     552  368 - #1f5fbf
     348  376 - #d7263d
     280  380 - #d7263d
     424  384 - #67707d
     424  388 - #67707d
      12  408 - #9aa4b2
     296  412 - #1f5fbf
     412  420 - #5b6472
     524  420 - #1f5fbf
     312  440 - #1f5fbf
      32  452 - #e3e6ec
      80  452 - #e3e6ec
     124  452 - #e3e6ec
     164  452 - #e3e6ec
     208  452 - #e3e6ec
     252  452 - #e3e6ec
     380  452 - #e3e6ec
     544  452 - #e3e6ec
     584  452 - #e3e6ec
     424  456 - #6c7480
     332  468 - #1f5fbf
     492  468 - #1f5fbf
     336  472 - #1f5fbf
     340  476 - #1f5fbf
     412  484 - #5b6472
      12  496 - #9aa4b2
     360  496 - #1f5fbf
     464  496 - #1f5fbf
     400  516 - #1f5fbf
     408  516 - #1f5fbf
     420  516 - #1f5fbf
      12  540 - #9aa4b2
     592  548 - #ffffff
     124  560 - #ffffff
     412  560 - #5b6472
      12  584 - #9aa4b2
     236  592 - #ffffff
     508  592 - #ffffff
";
