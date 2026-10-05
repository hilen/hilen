use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak, vents::Event},
    gm::{color::Color, flat::Point},
    ui::{
        Button, CanvasView, Container, DrawingView, Label, Setup, StrokeStyle, VectorPath, ViewData,
        ViewFrame, ViewSubviews, ViewTest, ViewTouch, view,
    },
    ui_test::{check_colors, inject_scroll, inject_touches},
};

/// A canvas draws its subviews bigger: the frame, the corner radius, the
/// border, the text and the path all grow together and stay sharp. A tap
/// lands on the subview drawn under it and arrives in its own points. A
/// drag on the empty area pans and the wheel zooms at the cursor.
#[view]
struct CanvasZoom {
    scene: Weak<ZoomScene>,

    #[init]
    status: Label,
    canvas: CanvasView,
}

impl Setup for CanvasZoom {
    fn setup(mut self: Weak<Self>) {
        self.set_color("#FFFFFF");

        self.status.set_text_size(22).set_text_color("#1B1F24");
        self.status.place().lrt(0).h(44);

        self.canvas
            .set_color("#EEF1F5")
            .set_border_color("#9AA4B2")
            .set_border_width(2)
            .set_corner_radius(8);
        self.canvas.place().t(52).lr(12).b(12);

        self.scene = self.canvas.add_view::<ZoomScene>();
        self.scene.place().back();

        self.canvas.on_change.sub(move || self.show_status());
        self.scene.changed.sub(move || self.show_status());
        self.show_status();
    }
}

impl CanvasZoom {
    fn show_status(self: Weak<Self>) {
        self.status.set_text(format!(
            "zoom {:.2}   taps {}",
            self.canvas.zoom(),
            self.scene.taps
        ));
    }
}

/// What the canvas shows, laid out in a quarter of it, so the whole of it
/// still fits at zoom 2.
#[view]
struct ZoomScene {
    taps:      usize,
    /// Where the last touch landed on the pad, in the points of the pad.
    pad_touch: Point,
    changed:   Event,

    #[init]
    card:       Container,
    card_title: Label,
    button:     Button,
    path:       DrawingView,
    path_title: Label,
    pad:        Container,
    pad_title:  Label,
}

impl Setup for ZoomScene {
    fn setup(mut self: Weak<Self>) {
        self.card
            .set_color("#2F6FED")
            .set_corner_radius(14)
            .set_border_color("#12357A")
            .set_border_width(3);
        self.card.place().tl(16).size(120, 70);
        self.card_title.set_text("card").set_text_size(18).set_text_color("#FFFFFF");
        self.card_title.place().tl(16).size(120, 70);

        self.button.set_text("button").set_text_size(18).set_text_color("#FFFFFF");
        self.button.set_color("#1F9D55").set_corner_radius(10);
        self.button.place().t(16).l(150).size(120, 70);
        self.button.on_tap(move || {
            self.taps += 1;
            self.changed.trigger(());
        });

        let zigzag = VectorPath::polyline([(8, 40), (34, 10), (60, 40), (86, 10), (112, 40)]);
        self.path.add_stroke(&zigzag, Color::hex("#E8590C"), StrokeStyle::width(6));
        self.path.set_color("#FFFFFF").set_corner_radius(6);
        self.path.place().t(100).l(16).size(120, 50);
        self.path_title.set_text("path").set_text_size(14).set_text_color("#1B1F24");
        self.path_title.place().t(150).l(16).size(120, 20);

        self.pad.set_color("#C9D1DB").set_corner_radius(6);
        self.pad.place().t(100).l(150).size(120, 70);
        self.pad.enable_touch();
        self.pad.touch().began.val(move |touch| {
            self.pad_touch = touch.position;
            self.changed.trigger(());
        });
        self.pad_title.set_text("pad").set_text_size(18).set_text_color("#1B1F24");
        self.pad_title.place().t(100).l(150).size(120, 70);
    }
}

const CHECK_1: &str = r"
     276   20 - #3c4044
     380   20 - #1c2025
     224   24 - #ffffff
     256   24 - #cacbcc
     308   24 - #ffffff
     352   24 - #d1d2d3
     576   52 - #9aa4b2
     124   68 - #12357a
     164   76 - #1f9d55
     280   76 - #1f9d55
      32   80 - #2f6fed
     104  100 - #7ba4f4
      76  104 - #2f6fed
      88  104 - #6796f2
     100  104 - #2f6fed
     200  104 - #1f9d55
     220  104 - #1f9d55
     232  108 - #fdfefe
      36  128 - #2f6fed
     136  136 - #12357a
     272  156 - #c9d1db
      68  168 - #e8590c
      44  184 - #e8590c
      96  184 - #e8590c
     136  188 - #e8590c
     228  188 - #1c2025
     208  192 - #43484e
      80  212 - #1d2126
     280  216 - #c9d1db
     592  272 - #ffffff
      12  576 - #9aa4b2
     592  592 - #ffffff
";

const CHECK_2: &str = r"
     296   20 - #ffffff
     380   20 - #1c2025
     256   24 - #cacbcc
     212   28 - #1b1f24
     244   84 - #12357a
      64   92 - #2f6fed
     316  132 - #1f9d55
     188  152 - #2f6fed
     388  152 - #1f9d55
     164  156 - #8baff5
     172  156 - #dbe6fc
     136  160 - #2f6fed
     164  160 - #8baff5
     456  160 - #1f9d55
     172  164 - #dbe6fc
      52  212 - #12357a
     272  212 - #12357a
     312  264 - #c9d1db
     212  268 - #e8590c
     456  312 - #383d43
     436  320 - #1b1f24
     452  320 - #c9d1db
     456  324 - #373c41
     404  336 - #6c7179
     180  364 - #4e5257
     172  372 - #797c81
     160  376 - #eef1f5
     172  376 - #797c81
     180  376 - #4e5257
     140  384 - #64676c
      12  576 - #9aa4b2
     592  592 - #ffffff
";

const CHECK_3: &str = r"
     384   16 - #1b1f24
     308   20 - #ffffff
     256   24 - #cacbcc
     576   52 - #9aa4b2
     220  120 - #12357a
      16  124 - #2f6fed
     264  128 - #1f9d55
     352  180 - #49af75
     364  180 - #8bcca7
     140  184 - #2f6fed
     380  184 - #1f9d55
     404  184 - #1f9d55
     108  188 - #2f6fed
     352  188 - #49af75
     416  188 - #93d0ad
     428  188 - #d9eee2
     332  192 - #6bbe8e
     364  192 - #ffffff
     428  192 - #d9eee2
     224  244 - #12357a
      16  252 - #12357a
     212  348 - #e8590c
     368  352 - #1c2025
     404  356 - #c9d1db
     128  392 - #4c5054
      88  400 - #34383d
     128  400 - #4c5054
      88  404 - #1b1f24
      88  412 - #34383d
     396  420 - #c9d1db
     248  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_4: &str = r"
     384   16 - #1b1f24
     292   20 - #ffffff
     256   24 - #cacbcc
     216   28 - #1b1f24
      16   92 - #12357a
     516   96 - #1f9d55
     212  100 - #12357a
     336  156 - #51b37b
      96  164 - #83a9f4
     388  164 - #88cba4
      80  168 - #2f6fed
     120  168 - #2f6fed
     348  168 - #1f9d55
     400  168 - #5bb782
     420  168 - #1f9d55
     376  172 - #79c499
      68  176 - #bbd0f9
     100  176 - #6594f2
     400  176 - #5bb782
     448  176 - #42ac70
     340  180 - #ffffff
     376  180 - #79c499
     216  224 - #2f6fed
     204  240 - #12357a
      28  300 - #e8590c
     400  360 - #72787f
     412  364 - #c9d1db
     116  416 - #9fa3a7
      68  424 - #1b1f24
     268  436 - #c9d1db
      12  576 - #9aa4b2
     592  592 - #ffffff
";

fn close(a: Point, b: Point) -> bool {
    (a - b).length() < 0.01
}

impl ViewTest for CanvasZoom {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Zoom 1, the scene sits in the top left quarter at its own size.
        check_colors(CHECK_1)?;

        // Zoom 2 around the top left corner: the same scene fills the
        // canvas, every part twice as big.
        from_main(move || {
            let mut canvas = view.canvas;
            canvas.zoom_at((0, 0), 2.0);
        });
        check_colors(CHECK_2)?;

        // A tap where the button is drawn now. At zoom 1 this point is
        // empty canvas.
        inject_touches("432 154 b\n432 154 e");
        assert_eq!(from_main(move || view.scene.taps), 1);

        // A tap where the button was at zoom 1 is on the card now.
        inject_touches("222 103 b\n222 103 e");
        assert_eq!(from_main(move || view.scene.taps), 1);

        // The pad gets its touch in its own points: 20 screen points into
        // it are 10 of its own at zoom 2.
        inject_touches("332 272 b\n332 272 e");
        assert_eq!(from_main(move || view.scene.pad_touch), Point::new(10.0, 10.0));

        // A drag on the empty area pans by the way of the finger.
        inject_touches("300 450 b\n250 480 m\n250 480 e");
        assert!(close(
            from_main(move || view.canvas.offset()),
            Point::new(-50.0, 30.0)
        ));
        check_colors(CHECK_3)?;

        // The wheel zooms and keeps what is under the cursor in place.
        inject_touches("300 300 m");
        let under_cursor = move || {
            from_main(move || {
                let cursor = Point::new(300.0, 300.0) - view.canvas.absolute_frame().origin;
                view.canvas.content_point(cursor)
            })
        };
        let before = under_cursor();
        inject_scroll(60);
        let zoom = from_main(move || view.canvas.zoom());
        assert!((zoom - 2.0 * 0.12_f32.exp()).abs() < 0.001, "zoom {zoom}");
        assert!(close(before, under_cursor()));
        check_colors(CHECK_4)?;

        Ok(())
    }
}
