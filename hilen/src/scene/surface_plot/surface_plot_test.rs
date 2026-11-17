use anyhow::Result;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    scene::SurfacePlotView,
    ui::{Label, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, checkpoint, inject_touches, set_record_probe_count},
};

/// The saddle `z = (x² - y²) / 10` as a surface colored by height, blue
/// low, red high, over the floor grid with the 3 axes. A drag turns the
/// camera around it. A function with no finite value leaves no surface.
#[view]
struct SurfacePlot {
    #[init]
    status: Label,
    plot:   SurfacePlotView,
}

impl Setup for SurfacePlot {
    fn setup(mut self: Weak<Self>) {
        self.set_color("#FFFFFF");

        self.status
            .set_text("saddle z = (x^2 - y^2) / 10")
            .set_text_size(20)
            .set_text_color("#1B1F24");
        self.status.place().lrt(0).h(44);

        self.plot.place().t(52).lr(12).b(12);
        self.plot.set_function(|x, y| (x * x - y * y) / 10.0);
    }
}

impl ViewTest for SurfacePlot {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);
        wait_for_next_frame();
        wait_for_next_frame();
        assert!(from_main(move || view.plot.has_surface()));
        check_colors(CHECK_1)?;

        checkpoint("a drag to the left turns the camera around the saddle")?;
        let before = from_main(move || view.plot.camera().position);
        inject_touches("400 300 b\n380 300 m\n340 300 m\n300 300 m\n260 300 m\n250 300 e");
        wait_for_next_frame();
        wait_for_next_frame();
        let after = from_main(move || view.plot.camera().position);
        assert!(after.distance(before) > 1.0, "{before} {after}");
        check_colors(CHECK_2)?;

        from_main(move || {
            view.status.set_text("z = ln(-1) has no value, no surface");
            view.plot.set_function(|_, _| f64::NAN);
        });
        wait_for_next_frame();
        wait_for_next_frame();
        assert!(!from_main(move || view.plot.has_surface()));
        check_colors(CHECK_3)?;

        Ok(())
    }
}

const CHECK_1: &str = r"
     228   16 - #c3c4c5
     272   20 - #3e4145
     368   20 - #1c2025
     396   20 - #a0a2a4
     368   24 - #1c2025
     396   24 - #a0a2a4
     256   28 - #494d51
     316   28 - #494d51
     300  136 - #46a758
       4  152 - #ffffff
     192  164 - #bf2c2d
     228  176 - #b33936
     300  184 - #46a758
     152  196 - #cf4744
     180  204 - #d0524b
     252  204 - #ac5c50
     456  204 - #761e1e
     208  208 - #cc5950
     232  216 - #c66659
     468  220 - #842e2b
     144  228 - #d7695e
     176  232 - #d97365
     268  232 - #a9816c
     300  232 - #46a758
     432  232 - #782826
     224  236 - #d47f6e
     456  236 - #853833
     248  252 - #d39780
     416  252 - #993935
     436  252 - #7d3f37
     168  256 - #df8f7b
     200  256 - #e19680
     140  260 - #de8f7b
     452  264 - #925a4e
     276  272 - #c8b193
     352  276 - #f4f5f7
     416  284 - #836051
     308  292 - #f4f5f7
     468  296 - #a8957b
     148  304 - #e8c9a7
     196  304 - #ebd1ae
     396  308 - #867d67
     264  312 - #eee4bd
     304  312 - #dbd9b4
     360  312 - #b68772
     104  316 - #e6d0ad
     416  316 - #94947c
     436  320 - #b0b5b7
     312  324 - #e5484d
     456  324 - #949c8a
     388  328 - #767e71
     280  332 - #3e8ed0
     452  332 - #879489
     408  336 - #75847c
     420  340 - #748581
     192  344 - #e0e5c5
     348  344 - #bc957d
     432  344 - #708584
     152  348 - #d9dfc1
     248  348 - #e1e7c9
     392  348 - #5c7377
     404  348 - #61787c
     380  352 - #4f6973
     420  356 - #597681
     404  360 - #4d6d7c
     320  364 - #d0c7a4
     376  368 - #335973
     396  368 - #3d627a
     372  372 - #2b5271
     448  372 - #e5484d
     124  380 - #afc5bb
     152  380 - #b1c7bd
     204  380 - #3e8ed0
     472  380 - #e5484d
     240  388 - #b6cdc1
     164  404 - #3e8ed0
     276  404 - #b0c2b5
     352  404 - #b89b81
     312  416 - #b9bfa5
     188  424 - #81abba
     220  432 - #81abb9
     352  444 - #f4f5f7
     168  452 - #5c93b5
     300  452 - #9daea3
     220  464 - #6194b3
     252  468 - #6f9aad
     200  488 - #417fb0
     268  492 - #6992a5
     208  500 - #3a79ae
     224  500 - #427eac
     276  504 - #6890a1
     292  504 - #78979e
     244  516 - #437da7
       4  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_2: &str = r"
     228   16 - #c3c4c5
     272   20 - #3e4145
     368   20 - #1c2025
     396   20 - #a0a2a4
     204   24 - #ffffff
     288   24 - #ffffff
     368   24 - #1c2025
     404   24 - #ffffff
     256   28 - #494d51
     316   28 - #494d51
     360   28 - #494d51
       4  104 - #ffffff
     592  120 - #ffffff
     300  132 - #46a758
     292  160 - #c53636
     324  160 - #ba3031
     300  176 - #46a758
     276  184 - #cf514b
     356  184 - #b0433e
     312  192 - #c7534c
     232  204 - #d97265
     264  216 - #dc7d6d
     336  216 - #c16b5d
     380  216 - #aa6456
     300  224 - #46a758
     212  236 - #e1aa8f
     240  240 - #772121
     220  244 - #e4b497
     212  252 - #e5c4a3
     324  252 - #782725
     372  252 - #b28e77
     200  260 - #e7d7b2
     396  260 - #a7957b
     244  280 - #7b3732
     284  280 - #7b3933
     408  280 - #999b83
     332  288 - #7c3e37
     372  288 - #863e38
     192  296 - #7c3d37
     292  312 - #81594b
     124  316 - #3e8ed0
     136  316 - #3e8ed0
     144  316 - #3e8ed0
     224  316 - #805448
     268  320 - #826252
     364  320 - #83594b
     436  320 - #c7ccd4
     188  332 - #815b4d
     308  332 - #f4f5f7
     384  344 - #8e7562
     328  348 - #b8bec8
     416  352 - #9c7b67
     360  360 - #c7ccd4
     452  360 - #a37b68
     140  364 - #836b59
     248  364 - #c7ccd4
     168  372 - #867f68
     208  372 - #72796c
     388  372 - #808473
     200  376 - #747a6c
     392  380 - #757e72
     244  384 - #e5e7eb
     276  384 - #b8bec8
     412  384 - #92957f
     444  384 - #a79c80
     192  392 - #606e6a
     468  392 - #ab9c81
     152  400 - #767b6c
     404  400 - #5e7274
     116  404 - #84836d
     224  404 - #e5e7eb
     428  404 - #869183
     364  408 - #c7ccd4
     444  416 - #879387
     420  424 - #4e6b79
     436  424 - #708483
     176  428 - #305066
     464  428 - #8b978a
     172  432 - #315166
     440  432 - #688083
     168  436 - #325166
     160  440 - #385567
     292  440 - #e5484d
     452  440 - #6b8385
       4  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_3: &str = r"
     568    4 - #ffffff
     216   16 - #1b1f24
     236   16 - #4d5054
     320   16 - #383b3f
     172   20 - #3e4145
     188   20 - #888a8d
     272   20 - #44474c
     320   20 - #383b3f
     332   20 - #1b1f24
     356   20 - #1b1f24
     404   20 - #44474c
     412   20 - #1c2025
     172   24 - #313539
     188   24 - #888a8d
     196   24 - #4c4f53
     216   24 - #1c2025
     280   24 - #909295
     320   24 - #383b3f
     336   24 - #1b1f24
     400   24 - #1b1f24
     404   24 - #44474c
     156   28 - #494d51
     300  132 - #46a758
       4  156 - #ffffff
     592  156 - #ffffff
     300  160 - #46a758
     300  196 - #46a758
     300  228 - #46a758
     300  248 - #46a758
     300  264 - #46a758
     244  268 - #c7ccd4
     288  276 - #e5e7eb
     360  276 - #f4f5f7
     404  276 - #f4f5f7
     200  280 - #f4f5f7
     268  280 - #f4f5f7
     336  280 - #f4f5f7
     300  288 - #46a758
     288  300 - #f4f5f7
     336  308 - #c7ccd4
     404  308 - #c7ccd4
     124  316 - #3e8ed0
     136  316 - #3e8ed0
     200  320 - #f4f5f7
     268  320 - #3e8ed0
     276  320 - #3e8ed0
     288  320 - #3e8ed0
     292  320 - #3e8ed0
     296  320 - #3e8ed0
     436  320 - #c7ccd4
     300  324 - #e5484d
     388  344 - #c7ccd4
     272  348 - #c7ccd4
     328  348 - #b8bec8
     296  356 - #e5484d
     296  360 - #e5484d
     360  360 - #c7ccd4
     220  364 - #c7ccd4
     248  364 - #c7ccd4
     296  368 - #e5484d
     296  376 - #e5484d
     264  384 - #c7ccd4
     148  404 - #e5e7eb
     200  404 - #c7ccd4
     228  404 - #e5e7eb
     348  408 - #e5e7eb
     292  416 - #e5484d
     292  428 - #e5484d
     292  440 - #e5484d
       4  592 - #ffffff
     188  592 - #ffffff
     396  592 - #ffffff
     592  592 - #ffffff
";
