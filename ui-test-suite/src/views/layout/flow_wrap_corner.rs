use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, BLUE, Container, GREEN, Label, RED, Rect, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest,
        WHITE, YELLOW, view,
    },
    ui_test::check_colors,
};

const BOXES: [(f32, f32); 6] = [
    (100.0, 30.0),
    (100.0, 30.0),
    (60.0, 20.0),
    (120.0, 30.0),
    (150.0, 30.0),
    (100.0, 40.0),
];

const TOP_RIGHT: &str = r"
     592    4 - #597c95
      24   20 - #ff0000
      68   20 - #ff0000
     116   20 - #ff0000
     192   20 - #00ff00
     280   20 - #ffffff
     316   20 - #ffffff
     240   24 - #ffffff
     152   32 - #00ff00
     276   52 - #000000
     296   52 - #ffffff
      44   60 - #0000e7
     176   64 - #ffff00
     124   72 - #ffff00
      20   76 - #0000e7
      56   76 - #0000e7
      76   76 - #0000e7
     240   76 - #ffffff
     152   88 - #ffff00
     204   88 - #ffff00
     288  100 - #000000
     100  120 - #ff0000
     248  120 - #00ff00
      60  132 - #000000
     140  132 - #000000
      20  136 - #000000
     180  136 - #00ff00
     316  136 - #000000
     592  280 - #597c95
     344  432 - #597c95
     100  592 - #597c95
     592  592 - #597c95
";

const TOP_LEFT: &str = r"
     592    4 - #597c95
      68   20 - #ffffff
     108   20 - #000000
     160   20 - #ff0000
     208   20 - #ff0000
     264   20 - #00ff00
     316   20 - #00ff00
      20   24 - #ffffff
      56   52 - #000000
      76   52 - #ffffff
     128   60 - #0000e7
     240   60 - #ffff00
     284   60 - #ffff00
     148   68 - #0000e7
      20   76 - #ffffff
      84   76 - #ffffff
     112   76 - #0000e7
     168   76 - #0000e7
     204   80 - #ffff00
     264   88 - #ffff00
     316   92 - #000000
      68   96 - #000000
     144  112 - #ff0000
     248  120 - #00ff00
     100  128 - #ff0000
      20  136 - #000000
     180  136 - #00ff00
     316  136 - #000000
     592  280 - #597c95
     296  456 - #597c95
       4  592 - #597c95
     592  592 - #597c95
";

const WIDE_CORNER: &str = r"
      72   20 - #ffffff
     256   20 - #ffffff
     316   20 - #ffffff
     176   52 - #ffffff
     184   52 - #ffffff
     192   52 - #c1c1c1
     196   52 - #343434
     208   52 - #ffffff
      80   76 - #ffffff
      20   84 - #000000
     136   92 - #00ff00
     268   92 - #0000e7
     240  108 - #0000e7
     296  108 - #0000e7
      48  132 - #ffff00
      92  132 - #ffff00
     176  132 - #ff0000
     268  132 - #ff0000
     236  144 - #ff0000
      64  156 - #ffff00
     136  156 - #ffff00
     296  156 - #ff0000
      20  168 - #000000
      88  176 - #00ff00
     212  204 - #000000
      60  208 - #00ff00
     116  208 - #00ff00
     272  208 - #000000
     592  272 - #597c95
     300  500 - #597c95
       4  592 - #597c95
     592  592 - #597c95
";

const ONE_BOX: &str = r"
     556    4 - #597c95
      52   20 - #ff0000
      92   20 - #ff0000
     116   20 - #ff0000
     184   20 - #000000
     280   20 - #ffffff
     316   20 - #ffffff
     240   24 - #ffffff
     152   36 - #000000
      20   40 - #ff0000
      84   40 - #ff0000
      56   48 - #ff0000
     112   48 - #ff0000
     180   48 - #000000
     208   52 - #000000
     248   52 - #ffffff
     276   52 - #000000
     296   52 - #ffffff
     132   64 - #000000
      20   76 - #000000
      84   76 - #000000
     176   76 - #000000
     240   76 - #ffffff
     288   76 - #ffffff
     316   76 - #ffffff
     592  240 - #597c95
       4  340 - #597c95
     316  392 - #597c95
     508  416 - #597c95
      72  592 - #597c95
     380  592 - #597c95
     592  592 - #597c95
";

// A flow of boxes that wraps around a view in its corner. Only the rows
// beside the corner view are shorter, the rows under it are full width.
#[view]
struct FlowWrapCorner {
    boxes:  Vec<Weak<Container>>,
    corner: Weak<Label>,

    #[init]
    flow: Container,
}

impl Setup for FlowWrapCorner {
    fn setup(mut self: Weak<Self>) {
        self.flow.set_color(BLACK);
        self.flow.place().tl(20).w(300).all(10).all_wrap();

        self.corner = self.flow.add_view::<Label>();
        self.corner.set_text("corner");
        self.corner.set_text_size(16);
        self.corner.set_color(WHITE);
        self.corner.place().tr(0).size(80, 60);
        self.flow.place().wrap_around(self.corner);

        for (index, (width, height)) in BOXES.into_iter().enumerate() {
            let container = self.flow.add_view::<Container>();
            container.set_color([RED, GREEN, BLUE, YELLOW][index % 4]);
            container.place().size(width, height);
            self.boxes.push(container);
        }
    }
}

fn assert_frame(frame: Rect, expected: (f32, f32, f32, f32), name: &str) {
    let (x, y, width, height) = expected;
    assert!(
        (frame.x() - x).abs() < 0.1
            && (frame.y() - y).abs() < 0.1
            && (frame.width() - width).abs() < 0.1
            && (frame.height() - height).abs() < 0.1,
        "{name}: expected {expected:?}, got {frame:?}"
    );
}

/// The frame of every box by the origin it must have, then the flow.
fn assert_layout(view: Weak<FlowWrapCorner>, origins: [(f32, f32); 6], flow: (f32, f32, f32, f32)) {
    let (frames, flow_frame) = from_main(move || {
        let frames: Vec<Rect> = view.boxes.iter().map(|b| *b.frame()).collect();
        (frames, *view.flow.frame())
    });

    for (index, ((x, y), (width, height))) in origins.into_iter().zip(BOXES).enumerate() {
        assert_frame(frames[index], (x, y, width, height), &format!("box {index}"));
    }
    assert_frame(flow_frame, flow, "flow");
}

impl ViewTest for FlowWrapCorner {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The corner view is top right. The 2 rows beside it end before
        // it, the third row is under it and takes the full width.
        check_colors(TOP_RIGHT)?;
        assert_layout(
            view,
            [
                (0.0, 0.0),
                (110.0, 0.0),
                (0.0, 40.0),
                (70.0, 40.0),
                (0.0, 80.0),
                (160.0, 80.0),
            ],
            (20.0, 20.0, 300.0, 120.0),
        );

        // The corner view is top left, the rows beside it start after it.
        from_main(move || {
            view.corner.place().clear().tl(0).size(80, 60);
        });
        wait_for_next_frame();
        check_colors(TOP_LEFT)?;
        assert_layout(
            view,
            [
                (90.0, 0.0),
                (200.0, 0.0),
                (90.0, 40.0),
                (160.0, 40.0),
                (0.0, 80.0),
                (160.0, 80.0),
            ],
            (20.0, 20.0, 300.0, 120.0),
        );

        // A wide corner view leaves room for no box beside it, every row
        // is under it.
        from_main(move || {
            view.corner.place().clear().tr(0).size(250, 60);
        });
        wait_for_next_frame();
        check_colors(WIDE_CORNER)?;
        assert_layout(
            view,
            [
                (0.0, 70.0),
                (110.0, 70.0),
                (220.0, 70.0),
                (0.0, 110.0),
                (130.0, 110.0),
                (0.0, 150.0),
            ],
            (20.0, 20.0, 300.0, 190.0),
        );

        // With 1 box left the flow still ends at the bottom of the corner
        // view.
        from_main(move || {
            view.corner.place().clear().tr(0).size(80, 60);
            for hidden in &view.boxes[1..] {
                hidden.set_hidden(true);
            }
        });
        wait_for_next_frame();
        check_colors(ONE_BOX)?;
        let flow = from_main(move || *view.flow.frame());
        assert_frame(flow, (20.0, 20.0, 300.0, 60.0), "flow as tall as the corner view");

        Ok(())
    }
}
