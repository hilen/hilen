use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, BLUE, Container, GREEN, RED, Rect, Setup, ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE,
        YELLOW, view,
    },
    ui_test::check_colors,
};

#[view]
struct FlowWrap {
    boxes: Vec<Weak<Container>>,

    #[init]
    flow: Container,
}

impl Setup for FlowWrap {
    fn setup(mut self: Weak<Self>) {
        self.flow.set_color(BLACK);
        self.flow.place().tl(20).w(300).all(10).all_wrap();

        for (width, height, color) in [
            (100, 50, RED),
            (120, 40, GREEN),
            (150, 60, BLUE),
            (80, 30, YELLOW),
            (340, 20, WHITE),
        ] {
            let container = self.flow.add_view::<Container>();
            container.set_color(color);
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

impl ViewTest for FlowWrap {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_initial_wrap(view)?;
        check_wrap_after_hide(view)?;
        check_wrap_after_resize(view)?;

        Ok(())
    }
}

fn check_initial_wrap(view: Weak<FlowWrap>) -> Result<()> {
    check_colors(
        r"
              56   24 - #ff0000
             116   24 - #ff0000
             156   24 - #00ff00
             208   24 - #00ff00
             248   28 - #00ff00
             104   56 - #ff0000
             180   56 - #00ff00
             316   60 - #000000
             272   64 - #000000
              72   68 - #ff0000
              24   72 - #000000
             184   84 - #ffff00
             232   84 - #ffff00
             116   92 - #0000e7
             204  100 - #ffff00
              60  108 - #0000e7
             256  108 - #ffff00
             312  116 - #000000
              24  128 - #0000e7
             120  136 - #0000e7
             168  136 - #0000e7
              76  148 - #000000
             356  152 - #ffffff
              36  168 - #ffffff
             144  168 - #ffffff
             224  168 - #ffffff
             288  168 - #ffffff
             300  300 - #597c95
             592  308 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
    )?;

    let (frames, flow) = from_main(move || {
        let frames: Vec<Rect> = view.boxes.iter().map(|b| *b.frame()).collect();
        (frames, *view.flow.frame())
    });

    assert_frame(frames[0], (0.0, 0.0, 100.0, 50.0), "first in row");
    assert_frame(frames[1], (110.0, 0.0, 120.0, 40.0), "second in row");
    assert_frame(frames[2], (0.0, 60.0, 150.0, 60.0), "wrapped to second row");
    assert_frame(frames[3], (160.0, 60.0, 80.0, 30.0), "second row neighbor");
    assert_frame(
        frames[4],
        (0.0, 130.0, 340.0, 20.0),
        "oversized child got own row",
    );
    assert_frame(flow, (20.0, 20.0, 300.0, 150.0), "container sized to content");

    Ok(())
}

fn check_wrap_after_hide(view: Weak<FlowWrap>) -> Result<()> {
    from_main(move || {
        view.boxes[1].set_hidden(true);
    });

    wait_for_next_frame();

    check_colors(
        r"
             592    4 - #597c95
              56   24 - #ff0000
             116   24 - #ff0000
             224   24 - #0000e7
             276   24 - #0000e7
             316   28 - #000000
              24   36 - #ff0000
             112   60 - #ff0000
             196   64 - #0000e7
              80   68 - #ff0000
             316   68 - #000000
              24   72 - #000000
             140   76 - #0000e7
             260   76 - #0000e7
              96   92 - #ffff00
              72   96 - #ffff00
              24  100 - #ffff00
              48  112 - #ffff00
             196  112 - #000000
              92  116 - #ffff00
             240  128 - #000000
             316  128 - #000000
              24  148 - #ffffff
             124  148 - #ffffff
             168  148 - #ffffff
             280  148 - #ffffff
             356  148 - #ffffff
             300  300 - #597c95
             592  308 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
    )?;

    let (frames, flow) = from_main(move || {
        let frames: Vec<Rect> = view.boxes.iter().map(|b| *b.frame()).collect();
        (frames, *view.flow.frame())
    });

    assert_frame(frames[0], (0.0, 0.0, 100.0, 50.0), "first after hide");
    assert_frame(frames[2], (110.0, 0.0, 150.0, 60.0), "moved up after hide");
    assert_frame(frames[3], (0.0, 70.0, 80.0, 30.0), "wrapped after hide");
    assert_frame(frames[4], (0.0, 110.0, 340.0, 20.0), "last row after hide");
    assert_frame(flow, (20.0, 20.0, 300.0, 130.0), "height follows hidden child");

    Ok(())
}

fn check_wrap_after_resize(view: Weak<FlowWrap>) -> Result<()> {
    from_main(move || {
        view.boxes[1].set_hidden(false);
        view.flow.place().w(500);
    });

    wait_for_next_frame();

    check_colors(
        r"
              56   24 - #ff0000
             116   24 - #ff0000
             248   24 - #00ff00
             316   24 - #0000e7
             408   24 - #0000e7
             436   24 - #ffff00
             472   24 - #ffff00
             448   48 - #ffff00
             496   48 - #ffff00
             180   52 - #00ff00
             140   56 - #00ff00
             220   56 - #00ff00
             368   56 - #0000e7
              64   64 - #ff0000
             104   68 - #ff0000
             408   68 - #0000e7
              24   72 - #000000
             272   76 - #0000e7
             328   88 - #000000
             516   92 - #000000
             180  100 - #ffffff
              76  104 - #ffffff
              36  108 - #ffffff
             136  108 - #ffffff
             224  108 - #ffffff
             400  108 - #000000
             472  108 - #000000
             300  300 - #597c95
             592  336 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
    )?;

    let (frames, flow) = from_main(move || {
        let frames: Vec<Rect> = view.boxes.iter().map(|b| *b.frame()).collect();
        (frames, *view.flow.frame())
    });

    assert_frame(frames[0], (0.0, 0.0, 100.0, 50.0), "first after resize");
    assert_frame(frames[1], (110.0, 0.0, 120.0, 40.0), "second after resize");
    assert_frame(frames[2], (240.0, 0.0, 150.0, 60.0), "third fits after resize");
    assert_frame(frames[3], (400.0, 0.0, 80.0, 30.0), "fourth fits after resize");
    assert_frame(
        frames[4],
        (0.0, 70.0, 340.0, 20.0),
        "wide child wrapped after resize",
    );
    assert_frame(flow, (20.0, 20.0, 500.0, 90.0), "container re-wrapped on resize");

    Ok(())
}
