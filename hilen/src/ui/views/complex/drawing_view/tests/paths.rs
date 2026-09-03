use anyhow::Result;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{
        color::{BLACK, BLUE, BROWN, GREEN, LIGHT_BLUE, ORANGE, PURPLE, RED, TURQUOISE, WHITE, YELLOW},
        flat::{FillRule, LineCap, LineJoin, Point, StrokeStyle, VectorPath},
    },
    ui::{CircleView, DrawingView, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, set_record_probe_count},
};

/// Pins every path primitive: strokes with miter, round and bevel joins
/// and butt, round and square caps, a cubic bezier stroke, a spiral
/// polyline, the same self intersecting star filled with both fill
/// rules, a donut whose hole comes from a second sub path, a closed
/// cubic blob drawn as fill plus stroked outline, and the graph case,
/// a jagged polyline over a translucent area fill with a circle view
/// dot on the last value.
#[view]
struct DrawingPaths {
    #[init]
    drawing: DrawingView,
    dot:     CircleView,
}

/// A pentagram: five points connected every second one, so the outline
/// self intersects and the two fill rules disagree about the middle.
fn star(center: Point, radius: f32) -> Vec<Point> {
    (0u8..5)
        .map(|i| {
            let angle = f32::from(i).mul_add(144.0, -90.0).to_radians();
            Point {
                x: radius.mul_add(angle.cos(), center.x),
                y: radius.mul_add(angle.sin(), center.y),
            }
        })
        .collect()
}

/// Three turns around the center with a steadily growing radius. Every
/// segment meets the next at a slightly different angle, which stresses
/// round joins far more than a zigzag.
fn spiral(center: Point, turns: f32, steps: u8) -> Vec<Point> {
    (0..=steps)
        .map(|i| {
            let progress = f32::from(i) / f32::from(steps);
            let angle = progress * turns * std::f32::consts::TAU;
            let radius = progress.mul_add(42.0, 5.0);
            Point {
                x: radius.mul_add(angle.cos(), center.x),
                y: radius.mul_add(angle.sin(), center.y),
            }
        })
        .collect()
}

const GRAPH_VALUES: [f32; 29] = [
    30.0, 150.0, 55.0, 170.0, 20.0, 120.0, 80.0, 185.0, 40.0, 95.0, 25.0, 160.0, 70.0, 180.0, 35.0, 140.0,
    90.0, 175.0, 15.0, 110.0, 60.0, 165.0, 45.0, 130.0, 85.0, 178.0, 50.0, 155.0, 118.0,
];
const GRAPH_BASE: f32 = 575.0;

fn graph_points() -> Vec<Point> {
    GRAPH_VALUES
        .iter()
        .zip(0u8..)
        .map(|(value, i)| Point {
            x: f32::from(i).mul_add(20.0, 20.0),
            y: GRAPH_BASE - value,
        })
        .collect()
}

impl Setup for DrawingPaths {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.drawing.place().back();

        let zigzag = |offset: f32| {
            VectorPath::polyline([
                (20.0 + offset, 80.0),
                (70.0 + offset, 25.0),
                (120.0 + offset, 80.0),
                (170.0 + offset, 25.0),
            ])
        };

        self.drawing.add_stroke(&zigzag(0.0), BLUE, StrokeStyle::width(10));

        self.drawing.add_stroke(
            &zigzag(200.0),
            RED,
            StrokeStyle::width(10).cap(LineCap::Round).join(LineJoin::Round),
        );

        self.drawing.add_stroke(
            &zigzag(400.0),
            BLACK,
            StrokeStyle::width(10).cap(LineCap::Square).join(LineJoin::Bevel),
        );

        let curve = VectorPath::builder()
            .move_to((20, 140))
            .cubic_to((110, 80), (200, 200), (290, 140))
            .build();
        self.drawing
            .add_stroke(&curve, PURPLE, StrokeStyle::width(6).cap(LineCap::Round));

        self.drawing.add_stroke(
            &VectorPath::polyline(spiral((450, 145).into(), 3.0, 96)),
            LIGHT_BLUE,
            StrokeStyle::width(3).join(LineJoin::Round),
        );

        self.drawing.add_fill(
            &VectorPath::polygon(star((90, 290).into(), 70.0)),
            ORANGE,
            FillRule::NonZero,
        );
        self.drawing.add_fill(
            &VectorPath::polygon(star((240, 290).into(), 70.0)),
            TURQUOISE,
            FillRule::EvenOdd,
        );

        let donut = VectorPath::builder().circle((390, 290), 60).circle((390, 290), 28).build();
        self.drawing.add_fill(&donut, BROWN, FillRule::EvenOdd);

        let blob = VectorPath::builder()
            .move_to((530, 225))
            .cubic_to((585, 235), (592, 300), (555, 340))
            .cubic_to((525, 370), (472, 348), (474, 300))
            .cubic_to((476, 258), (490, 218), (530, 225))
            .close()
            .build();
        self.drawing.add_fill(&blob, YELLOW, FillRule::NonZero);
        self.drawing
            .add_stroke(&blob, BLACK, StrokeStyle::width(3).join(LineJoin::Round));

        let graph = graph_points();

        let mut area = graph.clone();
        area.push((580, GRAPH_BASE).into());
        area.push((20, GRAPH_BASE).into());
        self.drawing.add_fill(
            &VectorPath::polygon(area),
            GREEN.with_alpha(0.15),
            FillRule::NonZero,
        );

        self.drawing.add_stroke(
            &VectorPath::polyline(graph),
            GREEN,
            StrokeStyle::width(2).join(LineJoin::Round),
        );

        self.dot.set_radius(5);
        self.dot.set_circle_color(GREEN);
        self.dot.place().t(452).l(575).size(10, 10);
    }
}

impl ViewTest for DrawingPaths {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(320);

        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || {
            // Three zigzags, the curve, the spiral, two stars, the
            // donut, blob fill and outline, graph area and line.
            assert_eq!(view.drawing.paths().len(), 12);
        });

        check_colors(COLORS)?;

        Ok(())
    }
}

const COLORS: &str = r"
       4    4 - #ffffff
     220    4 - #ffffff
     272   24 - #ff0000
     372   24 - #ff0000
     472   24 - #000000
     572   24 - #000000
     168   28 - #0000e7
      72   32 - #0000e7
     460   32 - #000000
      56   36 - #0000e7
     164   36 - #0000e7
     356   36 - #ff0000
     560   36 - #000000
     260   40 - #ff0000
     280   40 - #ff0000
     480   40 - #000000
      88   44 - #0000e7
     156   44 - #0000e7
     456   44 - #000000
     548   48 - #000000
      48   52 - #0000e7
      96   52 - #0000e7
     148   52 - #0000e7
     244   52 - #ff0000
     292   52 - #ff0000
     348   52 - #ff0000
     448   52 - #000000
     496   52 - #000000
     140   56 - #0000e7
     340   56 - #ff0000
     436   56 - #000000
     100   60 - #0000e7
     240   60 - #ff0000
     300   60 - #ff0000
     332   60 - #ff0000
     504   60 - #000000
     536   60 - #000000
      36   64 - #0000e7
     136   64 - #0000e7
     432   64 - #000000
     104   68 - #0000e7
     128   68 - #0000e7
     232   68 - #ff0000
     304   68 - #ff0000
      24   72 - #0000e7
     112   72 - #0000e7
     312   72 - #ff0000
     328   72 - #ff0000
     428   72 - #000000
     508   72 - #000000
     528   72 - #000000
     124   76 - #0000e7
     228   76 - #ff0000
     320   76 - #ff0000
     216   80 - #ff0000
     516   80 - #000000
     120   84 - #0000e7
     420   84 - #000000
     592   88 - #ffffff
     456  100 - #40e3ff
     436  104 - #00daff
     452  104 - #ffffff
     468  104 - #00daff
     480  112 - #00daff
     360  116 - #ffffff
     452  116 - #00daff
      76  120 - #ff00ff
     416  120 - #00daff
      52  124 - #ff00ff
      60  124 - #ff00ff
      68  124 - #ff00ff
      84  124 - #ff00ff
      92  124 - #ff00ff
     100  124 - #ff00ff
      40  128 - #ff00ff
      44  128 - #ff00ff
     108  128 - #ff00ff
     116  128 - #ff00ff
     124  128 - #ff00ff
     412  128 - #00daff
     436  128 - #ffffff
     452  128 - #00daff
     468  128 - #ffffff
     492  128 - #00daff
      28  132 - #ff00ff
      36  132 - #ff00ff
     132  132 - #ff00ff
     460  132 - #00daff
      24  136 - #ff00ff
     140  136 - #ff00ff
     144  136 - #ff00ff
     480  136 - #00daff
     496  136 - #00daff
      20  140 - #ff00ff
     152  140 - #ff00ff
     156  140 - #ff00ff
     160  140 - #ff00ff
     288  140 - #ff00ff
     408  140 - #bff6ff
     436  140 - #80edff
     168  144 - #ff00ff
     280  144 - #ff00ff
     284  144 - #ff00ff
     408  144 - #80edff
     424  144 - #00daff
     180  148 - #ff00ff
     272  148 - #ff00ff
     276  148 - #ff00ff
     468  148 - #00daff
     592  148 - #ffffff
     192  152 - #ff00ff
     200  152 - #ff00ff
     208  152 - #ff00ff
     260  152 - #ff00ff
     264  152 - #ff00ff
     444  152 - #00daff
     452  152 - #00daff
     204  156 - #ff00ff
     216  156 - #ff00ff
     224  156 - #ff00ff
     244  156 - #ff00ff
     252  156 - #ff00ff
     412  156 - #00daff
     428  156 - #00daff
     228  160 - #ffbfff
     232  160 - #ffbfff
     236  160 - #ffbfff
     480  160 - #00daff
     436  164 - #00daff
     448  168 - #00daff
     348  172 - #ffffff
     424  172 - #00daff
     540  172 - #ffffff
     436  180 - #00daff
     456  180 - #00daff
      56  184 - #ffffff
       4  204 - #ffffff
     148  204 - #ffffff
     300  208 - #ffffff
     240  224 - #00ffff
     520  224 - #000000
      88  228 - #ffcb00
     504  228 - #000000
     540  228 - #000000
     396  232 - #daaa7c
     368  236 - #daaa7c
     560  240 - #000000
     244  244 - #00ffff
     348  248 - #daaa7c
     412  248 - #daaa7c
      92  252 - #ffcb00
     232  252 - #00ffff
     388  252 - #daaa7c
     500  252 - #ffff00
     528  256 - #ffff00
     572  256 - #000000
     480  260 - #000000
      28  268 - #ffd840
      40  268 - #ffd840
      52  268 - #ffd840
      60  268 - #ffd840
      64  268 - #ffd840
      68  268 - #ffd840
      72  268 - #ffd840
     108  268 - #ffd840
     112  268 - #ffd840
     116  268 - #ffd840
     120  268 - #ffd840
     124  268 - #ffd840
     128  268 - #ffd840
     132  268 - #ffd840
     136  268 - #ffd840
     140  268 - #ffd840
     144  268 - #ffd840
     148  268 - #ffd840
     152  268 - #ffd840
     176  268 - #40ffff
     184  268 - #40ffff
     192  268 - #40ffff
     204  268 - #40ffff
     212  268 - #40ffff
     216  268 - #40ffff
     220  268 - #40ffff
     228  268 - #bfffff
     232  268 - #bfffff
     236  268 - #bfffff
     240  268 - #bfffff
     244  268 - #bfffff
     248  268 - #bfffff
     252  268 - #bfffff
     260  268 - #40ffff
     264  268 - #40ffff
     268  268 - #40ffff
     272  268 - #40ffff
     276  268 - #40ffff
     280  268 - #40ffff
     284  268 - #40ffff
     288  268 - #40ffff
     292  268 - #40ffff
     296  268 - #40ffff
     300  268 - #40ffff
     304  268 - #40ffff
     336  268 - #daaa7c
     368  268 - #daaa7c
     576  268 - #000000
     416  272 - #daaa7c
     504  272 - #ffff00
     440  276 - #daaa7c
     548  276 - #ffff00
     116  280 - #ffcb00
     476  280 - #000000
     576  284 - #808000
      92  288 - #ffcb00
     208  288 - #00ffff
     276  288 - #00ffff
     264  292 - #00ffff
     352  292 - #daaa7c
     524  292 - #ffff00
     576  292 - #000000
      64  296 - #ffcb00
     472  296 - #bfbfbf
     500  296 - #ffff00
     544  296 - #ffff00
     332  300 - #daaa7c
     472  300 - #808080
     576  300 - #000000
     220  304 - #00ffff
     260  304 - #00ffff
     416  304 - #daaa7c
     472  304 - #808080
     116  312 - #ffcb00
     248  312 - #00ffff
     444  312 - #daaa7c
     572  312 - #000000
       4  316 - #ffffff
      88  316 - #ffcb00
     236  316 - #00ffff
     376  316 - #daaa7c
     476  316 - #000000
     540  316 - #ffff00
      64  320 - #ffcb00
     208  320 - #00ffff
     268  320 - #00ffff
     340  320 - #daaa7c
     476  320 - #000000
     512  320 - #ffff00
     568  320 - #000000
     224  324 - #00ffff
     252  324 - #00ffff
     400  324 - #daaa7c
     480  328 - #000000
     564  328 - #000000
     528  332 - #ffff00
     560  332 - #000000
     272  336 - #00ffff
      52  340 - #ffcb00
     124  340 - #ffcb00
     204  340 - #00ffff
     360  340 - #daaa7c
     420  340 - #daaa7c
     388  348 - #daaa7c
     500  348 - #000000
     544  348 - #000000
     516  352 - #000000
     532  352 - #000000
      88  384 - #ffffff
     160  392 - #00ff00
     456  392 - #ffffff
     280  396 - #00ff00
       4  404 - #ffffff
     592  412 - #ffffff
     356  416 - #00ff00
     240  420 - #00ff00
     152  428 - #00ff00
     512  432 - #00ff00
     284  436 - #a3ffa3
     320  436 - #00ff00
      84  444 - #36ff36
      36  448 - #00ff00
     432  448 - #00ff00
     580  456 - #00ff00
     168  460 - #a3ffa3
     344  464 - #00ff00
     476  468 - #d9ffd9
     124  472 - #d9ffd9
     232  472 - #6dff6d
     292  476 - #bfffbf
     532  476 - #00ff00
       4  484 - #ffffff
      48  492 - #d9ffd9
     424  492 - #00ff00
     372  496 - #00ff00
     176  500 - #bfffbf
     260  500 - #00ff00
     576  500 - #d9ffd9
      92  504 - #36ff36
     328  504 - #d9ffd9
     492  516 - #d9ffd9
     224  520 - #00ff00
     536  528 - #d9ffd9
      60  532 - #d9ffd9
     428  532 - #d9ffd9
     136  536 - #d9ffd9
     300  536 - #00ff00
     384  536 - #00ff00
      20  540 - #00ff00
     576  540 - #d9ffd9
     100  548 - #00ff00
     464  548 - #d9ffd9
     192  556 - #d9ffd9
     500  564 - #d9ffd9
      72  572 - #d9ffd9
     248  572 - #d9ffd9
     288  572 - #d9ffd9
     348  572 - #d9ffd9
     400  572 - #d9ffd9
     548  572 - #d9ffd9
       4  592 - #ffffff
     140  592 - #ffffff
     456  592 - #ffffff
";
