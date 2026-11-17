use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak, vents::Event},
    gm::{color::Color, flat::Point},
    ui::{
        Button, CanvasView, Container, DrawingView, Label, Setup, StrokeStyle, VectorPath, ViewData,
        ViewFrame, ViewSubviews, ViewTest, ViewTouch, view,
    },
    ui_test::{check_colors, checkpoint, inject_pinch, inject_touches},
};

/// 2 fingers over a canvas are a pinch: they zoom around the middle
/// between them and pan with it, and their release is no tap. A pinch on
/// a trackpad zooms at the cursor. 2 fingers on 2 buttons over the canvas
/// stay 2 taps. A button over the canvas covers the map that passes under
/// it and takes the tap there.
#[view]
struct CanvasPinch {
    scene:      Weak<PinchScene>,
    left_taps:  usize,
    right_taps: usize,
    blue_taps:  usize,

    #[init]
    status: Label,
    canvas: CanvasView,
    left:   Button,
    right:  Button,
}

impl Setup for CanvasPinch {
    fn setup(mut self: Weak<Self>) {
        self.set_color("#FFFFFF");

        self.status.set_text_size(20).set_text_color("#1B1F24");
        self.status.place().lrt(0).h(44);

        self.canvas
            .set_color("#EEF1F5")
            .set_border_color("#9AA4B2")
            .set_border_width(2)
            .set_corner_radius(8);
        self.canvas.place().t(52).lr(12).b(12);

        self.scene = self.canvas.add_view::<PinchScene>();
        self.scene.place().back();

        // Over the canvas, not inside it, and with no call to bring them
        // to the front. The map passes under the right one once it is
        // zoomed. The middle between the 2 is on the canvas.
        self.left.set_text("left").set_text_size(18).set_text_color("#FFFFFF");
        self.left.set_color("#7048E8").set_corner_radius(10);
        self.left.place().l(24).b(24).size(120, 60);
        self.left.on_tap(move || {
            self.left_taps += 1;
            self.show_status();
        });

        self.right.set_text("right").set_text_size(18).set_text_color("#FFFFFF");
        self.right.set_color("#7048E8").set_corner_radius(10);
        self.right.place().r(24).t(64).size(120, 60);
        self.right.on_tap(move || {
            self.right_taps += 1;
            self.show_status();
        });

        // The blue node takes touches, and it starts to after the buttons
        // did, so the order the views were added in would hand it a tap on
        // the button over it.
        self.scene.laptop.enable_touch();
        self.scene.laptop.touch().up_inside.sub(self, move || {
            self.blue_taps += 1;
            self.show_status();
        });

        self.canvas.on_change.sub(move || self.show_status());
        self.scene.changed.sub(move || self.show_status());
        self.show_status();
    }
}

impl CanvasPinch {
    fn show_status(self: Weak<Self>) {
        self.status.set_text(format!(
            "zoom {:.2}   node {}   blue {}   left {}   right {}",
            self.canvas.zoom(),
            self.scene.taps,
            self.blue_taps,
            self.left_taps,
            self.right_taps,
        ));
    }

    /// The point of the scene drawn at a point of the screen.
    fn under(self: Weak<Self>, screen: Point) -> Point {
        from_main(move || self.canvas.content_point(screen - self.canvas.absolute_frame().origin))
    }

    fn zoom(self: Weak<Self>) -> f32 {
        from_main(move || self.canvas.zoom())
    }
}

/// A small network map: 3 nodes and the links between them.
#[view]
struct PinchScene {
    taps:    usize,
    changed: Event,

    #[init]
    links:  DrawingView,
    router: Container,
    laptop: Container,
    node:   Button,
}

impl Setup for PinchScene {
    fn setup(mut self: Weak<Self>) {
        let links = VectorPath::polyline([(110, 130), (288, 230), (460, 130)]);
        self.links.add_stroke(&links, Color::hex("#5C6B7A"), StrokeStyle::width(4));
        self.links.place().back();

        self.router.set_color("#E8590C").set_corner_radius(24);
        self.router.place().l(86).t(106).size(48, 48);

        self.laptop.set_color("#2F6FED").set_corner_radius(24);
        self.laptop.place().l(436).t(106).size(48, 48);

        self.node.set_text("node").set_text_size(18).set_text_color("#FFFFFF");
        self.node.set_color("#1F9D55").set_corner_radius(12);
        self.node.place().l(228).t(200).size(120, 60);
        self.node.on_tap(move || {
            self.taps += 1;
            self.changed.trigger(());
        });
    }
}

const CHECK_1: &str = r"
     300   16 - #cacbcc
     192   20 - #1b1f24
     256   20 - #afb0b2
     312   20 - #4e5155
     448   20 - #1b1f24
     128   24 - #ffffff
     300   24 - #cacbcc
     380   24 - #fafafa
     436   24 - #a9abad
     460   24 - #35383d
     464   24 - #4e5155
     572   68 - #7048e8
     512   96 - #7048e8
      16  120 - #e8590c
     584  152 - #2f6fed
     464  184 - #5c6b7a
     136  192 - #5c6b7a
     376  236 - #5c6b7a
     212  248 - #1f9d55
      12  268 - #9aa4b2
     308  280 - #1f9d55
     280  284 - #5cb883
     324  284 - #c8e7d5
     280  288 - #5cb883
     292  288 - #1f9d55
     376  324 - #1f9d55
     140  520 - #7048e8
      72  548 - #cabbf6
      80  548 - #7048e8
      88  548 - #c1aff5
     360  592 - #ffffff
     592  592 - #ffffff
";

/// Points of the right button that have the blue node under them. Written
/// by hand, the recorder put no probe on this small area, and it is the
/// area this check is for: with the map drawn over the button these
/// points are blue.
const OVER_BUTTON: &str = r"
     540  116 - #7048e8
     548  112 - #7048e8
     566  118 - #7048e8
";

fn close(a: Point, b: Point) -> bool {
    (a - b).length() < 0.05
}

impl ViewTest for CanvasPinch {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        checkpoint("zoom 1, the map at its own size")?;

        // 2 fingers go apart to 1.5 times their distance. The point
        // between them, the middle of the node, stays where it is.
        let middle = Point::new(300.0, 282.0);
        let before = view.under(middle);
        inject_touches(
            "
            200 282 b 1
            400 282 b 2
            175 282 m 1
            425 282 m 2
            150 282 m 1
            450 282 m 2
            150 282 e 1
            450 282 e 2
        ",
        );
        assert!((view.zoom() - 1.5).abs() < 0.001, "zoom {}", view.zoom());
        assert!(close(before, view.under(middle)));

        // The blue node and its link are under the right button now and
        // the button covers them.
        check_colors(CHECK_1)?;
        check_colors(OVER_BUTTON)?;

        // A tap on the button where the blue node is under it goes to the
        // button.
        inject_touches("549 110 b\n549 110 e");
        assert_eq!(from_main(move || (view.right_taps, view.blue_taps)), (1, 0));
        checkpoint("a tap on right over the blue node, right is 1 and blue 0")?;

        // The part of the blue node below the button still takes its tap.
        inject_touches("558 150 b\n558 150 e");
        assert_eq!(from_main(move || (view.right_taps, view.blue_taps)), (1, 1));
        checkpoint("a tap on the blue node below the button, blue is 1")?;

        // Both fingers move the same way, the map goes with them and the
        // zoom stays.
        let offset = from_main(move || view.canvas.offset());
        inject_touches("150 282 b 1\n450 282 b 2\n190 302 m 1\n490 302 m 2");
        assert!((view.zoom() - 1.5).abs() < 0.001, "zoom {}", view.zoom());
        assert!(close(
            from_main(move || view.canvas.offset()),
            offset + Point::new(40.0, 20.0)
        ));
        inject_touches("190 302 e 1\n490 302 e 2");
        checkpoint("panned by both fingers, right 40 and down 20")?;

        // A pinch that begins with a finger on the node is no tap of it.
        let node = from_main(move || view.scene.node.absolute_frame().center());
        let (x, y) = (node.x.round(), node.y.round());
        inject_touches(format!(
            "{x} {y} b 1\n{} {y} b 2\n{} {y} m 2\n{x} {y} e 1\n{} {y} e 2",
            x + 150.0,
            x + 180.0,
            x + 180.0,
        ));
        assert_eq!(from_main(move || view.scene.taps), 0);
        checkpoint("a pinch began on the node, node stays 0")?;

        // 1 finger on the node is a tap.
        let node = from_main(move || view.scene.node.absolute_frame().center());
        let (x, y) = (node.x.round(), node.y.round());
        inject_touches(format!("{x} {y} b\n{x} {y} e"));
        assert_eq!(from_main(move || view.scene.taps), 1);
        checkpoint("1 finger tapped the node, node is 1")?;

        // A pinch on a trackpad halves the zoom at the cursor.
        let zoom = view.zoom();
        let before = view.under(middle);
        inject_pinch(middle.x, middle.y, 0.5);
        assert!((view.zoom() - zoom / 2.0).abs() < 0.001, "zoom {}", view.zoom());
        assert!(close(before, view.under(middle)));
        checkpoint("a trackpad pinch halved the zoom")?;

        // 2 fingers on the 2 buttons over the canvas are 2 taps and no
        // pinch, each finger is held by a view outside the canvas.
        let zoom = view.zoom();
        inject_touches(
            "
            84  546 b 1
            516 94  b 2
            86  546 m 1
            514 94  m 2
            86  546 e 1
            514 94  e 2
        ",
        );
        assert_eq!(from_main(move || (view.left_taps, view.right_taps)), (1, 2));
        assert!((view.zoom() - zoom).abs() < f32::EPSILON);
        checkpoint("2 fingers on left and right, left is 1 and right 2, zoom the same")?;

        Ok(())
    }
}
