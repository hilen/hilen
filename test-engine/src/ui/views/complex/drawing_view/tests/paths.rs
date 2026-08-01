use anyhow::Result;
use hreads::{from_main, wait_for_next_frame};
use refs::Weak;

use crate::{
    self as test_engine,
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
      68   24 - #0000e7
     272   24 - #ff0000
     372   24 - #ff0000
     472   24 - #000000
     572   24 - #000000
     168   28 - #0000e7
      72   32 - #0000e7
     460   32 - #000000
     476   32 - #000000
      80   36 - #0000e7
     164   36 - #0000e7
     356   36 - #ff0000
      60   40 - #0000e7
     260   40 - #ff0000
     280   40 - #ff0000
     480   40 - #000000
     560   40 - #000000
      52   44 - #0000e7
      92   44 - #0000e7
     156   44 - #0000e7
     288   44 - #ff0000
     352   44 - #ff0000
     456   44 - #000000
      44   48 - #0000e7
     252   48 - #ff0000
     488   48 - #000000
     544   48 - #000000
     148   52 - #0000e7
     244   52 - #ff0000
     292   52 - #ff0000
     348   52 - #ff0000
     448   52 - #000000
     496   52 - #000000
      40   56 - #0000e7
      96   56 - #0000e7
     136   56 - #0000e7
     340   56 - #ff0000
     436   56 - #000000
     540   56 - #000000
     240   60 - #ff0000
     300   60 - #ff0000
     332   60 - #ff0000
     500   60 - #000000
      36   64 - #0000e7
     108   64 - #0000e7
     132   64 - #0000e7
     432   64 - #000000
     536   64 - #000000
     232   68 - #ff0000
     304   68 - #ff0000
     504   68 - #000000
     528   68 - #000000
      28   72 - #0000e7
     128   72 - #0000e7
     312   72 - #ff0000
     328   72 - #ff0000
     428   72 - #000000
     112   76 - #0000e7
     228   76 - #ff0000
     320   76 - #ff0000
     512   76 - #000000
     524   76 - #000000
      24   80 - #0000e7
     216   80 - #ff0000
     120   84 - #0000e7
     420   84 - #000000
     592   92 - #ffffff
     456  100 - #40e3ff
     436  104 - #00daff
     452  104 - #ffffff
     468  104 - #00daff
     480  112 - #00daff
     452  116 - #00daff
      76  120 - #ff00ff
      84  120 - #ff00ff
     356  120 - #ffffff
     416  120 - #00daff
     436  120 - #00daff
      52  124 - #ff00ff
      60  124 - #ff00ff
      68  124 - #ff00ff
      80  124 - #ff00ff
      92  124 - #ff00ff
     100  124 - #ff00ff
      40  128 - #ff00ff
      44  128 - #ff00ff
     108  128 - #ff00ff
     116  128 - #ff00ff
     412  128 - #00daff
     428  128 - #00daff
     436  128 - #ffffff
     452  128 - #00daff
     468  128 - #ffffff
     476  128 - #00daff
     492  128 - #00daff
      28  132 - #ff00ff
      32  132 - #ff00ff
      36  132 - #ff00ff
     124  132 - #ff00ff
     132  132 - #ff00ff
     460  132 - #00daff
      24  136 - #ff00ff
      28  136 - #ff00ff
     140  136 - #ff00ff
     144  136 - #ff00ff
     148  136 - #ff00ff
     480  136 - #00daff
      20  140 - #ff00ff
     156  140 - #ff00ff
     288  140 - #ff00ff
     408  140 - #bff6ff
     436  140 - #80edff
     496  140 - #00daff
     164  144 - #ff00ff
     168  144 - #ff00ff
     172  144 - #ff00ff
     280  144 - #ff00ff
     284  144 - #ff00ff
     408  144 - #80edff
     424  144 - #00daff
     180  148 - #ff00ff
     188  148 - #ff00ff
     272  148 - #ff00ff
     276  148 - #ff00ff
     468  148 - #00daff
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
     592  156 - #ffffff
     228  160 - #ffbfff
     232  160 - #ffbfff
     236  160 - #ffbfff
     480  160 - #00daff
     436  164 - #00daff
     448  168 - #00daff
     424  172 - #00daff
     540  172 - #ffffff
     348  176 - #ffffff
     436  180 - #00daff
     456  180 - #00daff
      56  184 - #ffffff
     148  204 - #ffffff
     296  204 - #ffffff
       4  208 - #ffffff
     240  224 - #00ffff
     512  224 - #000000
     520  224 - #bfbf00
      88  228 - #ffcb00
     376  232 - #daaa7c
     548  232 - #ffff00
     492  240 - #ffff00
     244  244 - #00ffff
     356  244 - #daaa7c
     400  244 - #daaa7c
     424  244 - #daaa7c
     232  252 - #00ffff
     568  252 - #ffff00
     248  256 - #00ffff
     376  256 - #daaa7c
     340  260 - #daaa7c
     508  260 - #ffff00
     436  264 - #daaa7c
     540  264 - #ffff00
      28  268 - #ffd840
      36  268 - #ffd840
      44  268 - #ffd840
      48  268 - #ffd840
      52  268 - #ffd840
      56  268 - #ffd840
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
     180  268 - #40ffff
     188  268 - #40ffff
     192  268 - #40ffff
     196  268 - #40ffff
     200  268 - #40ffff
     204  268 - #40ffff
     208  268 - #40ffff
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
     412  268 - #daaa7c
     476  272 - #000000
     364  276 - #daaa7c
     136  280 - #ffcb00
     216  280 - #00ffff
     332  280 - #daaa7c
     564  280 - #ffff00
     200  284 - #00ffff
     264  284 - #00ffff
     428  284 - #daaa7c
     500  284 - #ffff00
      88  288 - #ffcb00
     276  288 - #00ffff
     448  288 - #daaa7c
     212  292 - #00ffff
     524  292 - #ffff00
     472  296 - #bfbfbf
      68  300 - #ffcb00
     356  300 - #daaa7c
     420  300 - #daaa7c
     472  300 - #808080
     548  300 - #ffff00
     264  304 - #00ffff
     336  304 - #daaa7c
     472  304 - #808080
     100  308 - #ffcb00
     224  308 - #00ffff
     500  308 - #ffff00
     572  308 - #ffff00
     248  312 - #00ffff
     444  312 - #daaa7c
     212  316 - #00ffff
     376  316 - #daaa7c
     232  320 - #00ffff
     404  320 - #daaa7c
     256  324 - #00ffff
     272  324 - #00ffff
     348  324 - #daaa7c
     540  324 - #ffff00
     220  328 - #00ffff
     516  328 - #ffff00
      64  332 - #ffcb00
     424  336 - #daaa7c
     488  336 - #ffff00
     124  340 - #ffcb00
     204  340 - #00ffff
     276  340 - #00ffff
     368  340 - #daaa7c
     392  344 - #daaa7c
     520  352 - #ffff00
       4  360 - #ffffff
     592  380 - #ffffff
      88  384 - #ffffff
     160  400 - #d9ffd9
     300  400 - #ffffff
     452  400 - #ffffff
      24  420 - #ffffff
     240  420 - #d9ffd9
     360  424 - #d9ffd9
     516  424 - #d9ffd9
      84  440 - #a3ffa3
     152  452 - #d9ffd9
     416  452 - #ffffff
     576  456 - #00ff00
     580  456 - #00ff00
     580  460 - #00ff00
      48  468 - #d9ffd9
     344  468 - #d9ffd9
     528  472 - #d9ffd9
     292  476 - #bfffbf
       4  480 - #ffffff
     472  480 - #d9ffd9
     228  492 - #6dff6d
      92  500 - #a3ffa3
     176  500 - #bfffbf
     368  504 - #d9ffd9
     424  516 - #d9ffd9
     500  516 - #d9ffd9
     320  520 - #d9ffd9
     544  520 - #d9ffd9
      24  532 - #d9ffd9
     136  536 - #d9ffd9
     272  536 - #d9ffd9
     592  552 - #ffffff
     484  556 - #d9ffd9
      68  564 - #d9ffd9
     172  564 - #d9ffd9
     352  564 - #d9ffd9
     216  572 - #d9ffd9
     412  572 - #d9ffd9
       4  592 - #ffffff
     136  592 - #ffffff
     296  592 - #ffffff
     548  592 - #ffffff
";
