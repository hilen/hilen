//! `Graph view look`: 2 curves of 1 block in a still graph, in the light
//! theme and in the dark one.

use anyhow::{Result, ensure};

use super::{GraphView, graphs};
use crate::{
    self as hilen,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{BLACK, Label, Setup, TextAlignment, Theme, ThemeMode, ViewData, ViewTest, WHITE, view},
    ui_test::{check_colors, set_record_probe_count},
};

/// The block a chat would hold: 2 curves and the range they start at.
const BLOCK: &str = "y = sin(x)\ny = x^2/10\nx = -6..6";

#[view]
struct GraphViewLook {
    #[init]
    graph:  GraphView,
    status: Label,
}

impl Setup for GraphViewLook {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.graph.place().t(10).lr(10).h(400);
        self.status.set_text_size(16).set_multiline(true);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().t(420).lr(10).b(10);
    }
}

impl ViewTest for GraphViewLook {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        // A graph holds its formulas in an `Rc`, it is made on the main
        // thread where it stays.
        let texts = from_main(move || {
            let mut found = graphs(BLOCK);
            let Some(Ok(graph)) = (found.len() == 1).then(|| found.remove(0)) else {
                return Err(format!("the 2 curves are not 1 graph: {found:?}"));
            };
            let texts = graph.texts().to_vec();
            view.graph.set_graph(graph, false);
            view.status.set_text("light: sin(x) red and x^2/10 blue, from -6 to 6");
            Ok(texts)
        })
        .map_err(anyhow::Error::msg)?;
        ensure!(texts == ["y = sin(x)", "y = x^2/10"], "the graph holds {texts:?}");
        wait_for_next_frame();
        wait_for_next_frame();
        check_colors(LIGHT)?;

        from_main(move || {
            Theme::set_mode(ThemeMode::Dark);
            view.set_color(BLACK);
            view.status.set_text_color(WHITE);
            view.status.set_text("dark: the same curves in their dark colors");
        });
        wait_for_next_frame();
        wait_for_next_frame();
        check_colors(DARK)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`. The 11 probes on the flat tops of the
/// curves are left out, 1 sample draws a row there all curve or all ground.
const LIGHT: &str = r"
             164    4 - #ffffff
             300   12 - #5b6472
              20   48 - #1f5fbf
             576   52 - #1f5fbf
             300   64 - #5b6472
             304  112 - #f8f9fa
             516  112 - #1f5fbf
              96  124 - #1f5fbf
             488  136 - #1f5fbf
             300  156 - #5b6472
             172  176 - #1f5fbf
             336  176 - #c1121f
              28  180 - #c1121f
             116  180 - #c1121f
             424  184 - #c1121f
             220  196 - #1f5fbf
             368  200 - #1f5fbf
             324  208 - #1f5fbf
             588  224 - #c1121f
             176  236 - #c1121f
             272  236 - #c1121f
             480  236 - #c1121f
             564  244 - #c1121f
             300  288 - #5b6472
               4  340 - #ffffff
             300  368 - #5b6472
             492  436 - #ffffff
              28  500 - #545454
              44  500 - #000000
             128  500 - #d4d4d4
             160  500 - #383838
             220  500 - #747474
             228  500 - #383838
              28  504 - #545454
              32  504 - #6e6e6e
              40  504 - #000000
             200  504 - #5a5a5a
             212  504 - #000000
             228  504 - #383838
             232  504 - #000000
             320  504 - #ffffff
              32  508 - #6e6e6e
              44  508 - #000000
              84  508 - #737373
             100  508 - #ffffff
             188  508 - #ffffff
             200  508 - #5a5a5a
             228  508 - #383838
             244  508 - #ffffff
             276  508 - #636363
             288  508 - #c1c1c1
             440  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`, without the 10 probes on the flat tops
/// of the curves, for the reason above.
const DARK: &str = r"
             176   16 - #20242d
             328   16 - #20242d
             444   16 - #20242d
              20   48 - #6ea8fe
             576   52 - #6ea8fe
             300  104 - #9aa3b2
             516  112 - #6ea8fe
              96  116 - #000000
             308  116 - #191b1d
             504  116 - #000000
             128  148 - #6ea8fe
             460  156 - #6ea8fe
             164  172 - #6ea8fe
             336  176 - #ff6b6b
             400  176 - #000000
              28  180 - #ff6b6b
             116  180 - #ff6b6b
             424  184 - #ff6b6b
             220  196 - #6ea8fe
             384  200 - #000000
             396  204 - #20242d
             268  208 - #6ea8fe
             324  208 - #6ea8fe
             396  216 - #20242d
             584  228 - #ff6b6b
             176  236 - #ff6b6b
             272  236 - #ff6b6b
             480  236 - #ff6b6b
             540  256 - #ff6b6b
             300  280 - #9aa3b2
             396  320 - #20242d
              80  336 - #000000
             300  356 - #9aa3b2
             300  408 - #9aa3b2
             480  432 - #000000
              32  500 - #919191
              72  500 - #d7d7d7
             264  500 - #5a5a5a
             292  500 - #616161
              72  504 - #ffffff
             144  504 - #878787
             188  504 - #dedede
             192  504 - #ffffff
              44  508 - #323232
              72  508 - #d7d7d7
              76  508 - #666666
             116  508 - #555555
             120  508 - #e7e7e7
             188  508 - #dedede
             192  508 - #a4a4a4
             196  508 - #999999
             292  508 - #616161
             432  588 - #000000
             592  592 - #000000
";
