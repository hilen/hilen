use anyhow::Result;

use crate::{
    self as hilen,
    deps::{
        hreads::from_main,
        refs::{Own, Weak},
    },
    gm::{
        LossyConvert,
        color::{BLACK, BLUE, Color, GREEN, PURPLE, RED, TURQUOISE, YELLOW},
        flat::{CornerRadii, Rect},
    },
    render::{
        InstanceBinding, InstanceChunks, LabPipeline,
        data::{RectView, UIRectInstance},
    },
    ui::{Setup, UIManager, ViewCallbacks, ViewTest, view},
    ui_test::{check_colors, set_record_probe_count},
    window::RenderPass,
};

const RECT_SHADER: &str = include_str!("../../../render/pipelines/shaders/ui_rect.wgsl");

/// 25 cells of 24 px fill the 600 px canvas in both directions.
const COLUMNS: usize = 25;
const CELL: f32 = 24.0;

/// More than any device's chunk holds. A chunk takes at most 204 of these
/// 80 byte rects under the 16 KiB cap, so this flush crosses at least two
/// chunk boundaries on every device.
const RECTS: usize = 3 * 204 + 5;

const PALETTE: [Color; 6] = [RED, GREEN, BLUE, YELLOW, PURPLE, TURQUOISE];

/// The uniform instance path draws a flush in chunks of one binding window,
/// see `InstanceChunks`. This flush holds more rects than any chunk, so
/// every chunk boundary falls inside it, and the grid is pinned. A chunk
/// that binds the wrong window or does not restart its instance index at
/// zero paints the wrong colors past the first boundary.
#[view]
struct UniformInstances {
    pipelines: Own<Vec<LabPipeline>>,
}

impl Setup for UniformInstances {
    fn setup(mut self: Weak<Self>) {
        self.pipelines.push(LabPipeline::new(
            "uniform_instances_test",
            RECT_SHADER,
            false,
            false,
            InstanceBinding::Uniform,
        ));
    }
}

impl ViewCallbacks for UniformInstances {
    fn before_render(&self, pass: &mut RenderPass) {
        // The probes index screen pixels and the render area is in points,
        // so the grid, laid out in pixels, shrinks by the display scale
        // of a headed run. Headless runs at 1 and records unchanged.
        let scale = 1.0 / UIManager::display_scale();
        let mut pipelines = self.pipelines.weak();
        let pipeline = &mut pipelines[0];

        for index in 0..RECTS {
            let column: f32 = (index % COLUMNS).lossy_convert();
            let row: f32 = (index / COLUMNS).lossy_convert();

            pipeline.add(UIRectInstance::new(
                Rect::new(column * CELL + 2.0, row * CELL + 2.0, CELL - 4.0, CELL - 4.0),
                PALETTE[index % PALETTE.len()],
                BLACK,
                0.0,
                CornerRadii::default(),
                0.1,
                scale,
            ));
        }

        pipeline.draw(
            pass,
            RectView {
                resolution: UIManager::render_area(),
                _padding:   0,
            },
            None,
        );
    }
}

impl ViewTest for UniformInstances {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(480);

        // A flush inside one chunk would pin nothing about chunking.
        let per_chunk = from_main(|| InstanceChunks::new(size_of::<UIRectInstance>() as u64).per_chunk());
        assert!(
            RECTS > 2 * usize::try_from(per_chunk)?,
            "{RECTS} rects fit in two chunks of {per_chunk}"
        );

        grid_colors()
    }
}

fn grid_colors() -> Result<()> {
    check_colors(GRID_COLORS)
}

const GRID_COLORS: &str = r"
              24    4 - #ff0000
              60    4 - #00ff00
             108    4 - #0000e7
             132    4 - #0000e7
             176    4 - #ffff00
             232    4 - #ff00ff
             336    4 - #597c95
             472    4 - #ffff00
             516    4 - #ff00ff
             556    4 - #00ffff
             592    4 - #ff0000
             260    8 - #00ffff
             308    8 - #ff0000
             404    8 - #0000e7
              84   12 - #00ff00
             368   12 - #00ff00
             440   12 - #ffff00
             152   20 - #ffff00
             196   20 - #ff00ff
             284   24 - #597c95
               8   28 - #ff0000
              36   36 - #ac3e4a
              72   36 - #2dbe4a
             120   36 - #2d3ebe
             172   36 - #acbe4a
             208   36 - #ac3eca
             256   36 - #2dbeca
             300   36 - #ac3e4a
             320   36 - #ac3e4a
             344   36 - #2dbe4a
             368   36 - #2dbe4a
             392   36 - #2d3ebe
             420   36 - #2d3ebe
             472   36 - #acbe4a
             496   36 - #ac3eca
             516   36 - #ac3eca
             536   36 - #2dbeca
             556   36 - #2dbeca
             576   40 - #597c95
             228   44 - #00ffff
             448   44 - #ff00ff
             148   48 - #ff00ff
             292   52 - #00ff00
               4   56 - #00ff00
             360   56 - #0000e7
              96   60 - #597c95
             184   60 - #ff00ff
             332   60 - #597c95
             516   60 - #00ffff
             540   60 - #ff0000
             264   64 - #ff0000
             388   64 - #ffff00
              64   68 - #0000e7
             132   68 - #ffff00
              24   76 - #2dbe4a
              84   76 - #2d3ebe
             112   76 - #acbe4a
             160   76 - #ac3eca
             204   76 - #2dbeca
             248   76 - #ac3e4a
             276   76 - #ac3e4a
             308   76 - #2dbe4a
             328   76 - #2dbe4a
             364   76 - #2d3ebe
             412   76 - #acbe4a
             452   76 - #ac3eca
             488   76 - #2dbeca
             560   76 - #ac3e4a
             588   76 - #2dbe4a
             344   84 - #ffff00
             536   84 - #00ff00
             180   88 - #00ffff
             228   88 - #ff0000
               4   92 - #0000e7
              40   92 - #0000e7
              64   92 - #ffff00
             280   92 - #00ff00
             376   92 - #ffff00
             472   92 - #00ffff
             508   92 - #ff0000
              88   96 - #ffff00
             324  100 - #0000e7
             428  100 - #597c95
             120  104 - #ff00ff
             400  104 - #ff00ff
             192  108 - #597c95
             456  108 - #00ffff
             544  108 - #00ff00
             568  108 - #00ff00
              28  116 - #2d3ebe
              64  116 - #acbe4a
             104  116 - #ac3eca
             152  116 - #2dbeca
             172  116 - #2dbeca
             212  116 - #ac3e4a
             264  116 - #2dbe4a
             304  116 - #2d3ebe
             344  116 - #acbe4a
             364  116 - #acbe4a
             496  116 - #ac3e4a
             520  116 - #ac3e4a
             592  116 - #2d3ebe
             228  124 - #00ff00
               4  128 - #ffff00
             380  128 - #597c95
             416  128 - #00ffff
             440  128 - #ff0000
              88  132 - #ff00ff
             136  132 - #00ffff
             472  132 - #ff0000
             540  132 - #0000e7
             188  136 - #597c95
              40  140 - #ffff00
             116  140 - #00ffff
             592  140 - #ffff00
             324  144 - #ffff00
             564  144 - #0000e7
             292  152 - #ffff00
             432  152 - #597c95
               8  156 - #acbe4a
              28  156 - #acbe4a
              76  156 - #ac3eca
             136  156 - #2dbeca
             168  156 - #ac3e4a
             212  156 - #2dbe4a
             232  156 - #2dbe4a
             252  156 - #2d3ebe
             272  156 - #2d3ebe
             312  156 - #acbe4a
             348  156 - #ac3eca
             400  156 - #2dbeca
             460  156 - #ac3e4a
             504  156 - #2dbe4a
             536  156 - #2d3ebe
             372  164 - #00ffff
              48  168 - #597c95
             112  168 - #ff0000
             584  168 - #ff00ff
              16  172 - #ff00ff
             304  172 - #ff00ff
             556  172 - #ffff00
             232  176 - #0000e7
             416  176 - #ff0000
             328  180 - #ff00ff
             136  184 - #ff0000
             200  184 - #0000e7
             484  184 - #0000e7
              96  192 - #597c95
               8  196 - #ac3eca
              28  196 - #ac3eca
              56  196 - #2dbeca
              76  196 - #2dbeca
             124  196 - #ac3e4a
             156  196 - #2dbe4a
             176  196 - #2dbe4a
             248  196 - #acbe4a
             268  196 - #acbe4a
             300  196 - #ac3eca
             348  196 - #2dbeca
             396  196 - #ac3e4a
             420  196 - #ac3e4a
             444  196 - #2dbe4a
             464  196 - #2dbe4a
             496  196 - #2d3ebe
             516  196 - #2d3ebe
             536  196 - #acbe4a
             568  196 - #acbe4a
             592  196 - #ac3eca
             220  204 - #ffff00
             324  204 - #00ffff
             372  208 - #ff0000
             556  208 - #ff00ff
             120  212 - #00ff00
             196  212 - #ffff00
             492  212 - #ffff00
               4  216 - #00ffff
             248  216 - #ff00ff
             160  220 - #0000e7
             392  220 - #00ff00
             468  220 - #0000e7
             284  224 - #597c95
             416  224 - #00ff00
             332  228 - #597c95
              88  232 - #ff0000
              16  236 - #2dbeca
              40  236 - #2dbeca
              64  236 - #ac3e4a
             112  236 - #2dbe4a
             136  236 - #2dbe4a
             164  236 - #2d3ebe
             184  236 - #2d3ebe
             220  236 - #acbe4a
             268  236 - #ac3eca
             308  236 - #2dbeca
             360  236 - #ac3e4a
             444  236 - #2d3ebe
             500  236 - #acbe4a
             536  236 - #ac3eca
             560  236 - #ac3eca
             584  236 - #2dbeca
             252  244 - #00ffff
             392  244 - #0000e7
             472  248 - #ffff00
             340  252 - #00ff00
               8  256 - #ff0000
             160  256 - #ffff00
             200  256 - #ff00ff
             292  256 - #ff0000
              80  260 - #00ff00
             428  268 - #597c95
             108  272 - #0000e7
             376  272 - #00ff00
             560  272 - #00ffff
              28  276 - #ac3e4a
              56  276 - #2dbe4a
             136  276 - #2d3ebe
             184  276 - #acbe4a
             212  276 - #ac3eca
             232  276 - #ac3eca
             264  276 - #2dbeca
             304  276 - #ac3e4a
             328  276 - #ac3e4a
             352  276 - #2dbe4a
             404  276 - #2d3ebe
             452  276 - #acbe4a
             472  276 - #acbe4a
             492  276 - #ac3eca
             516  276 - #ac3eca
             540  276 - #2dbeca
             588  276 - #ac3e4a
              84  288 - #0000e7
             164  288 - #ff00ff
               4  292 - #00ff00
             440  292 - #ff00ff
             552  296 - #ff0000
             192  300 - #597c95
             288  300 - #597c95
             372  300 - #0000e7
             472  300 - #ff00ff
             512  300 - #00ffff
             140  304 - #597c95
             592  304 - #00ff00
             244  312 - #ff0000
              12  316 - #2dbe4a
              36  316 - #2dbe4a
              72  316 - #2d3ebe
             116  316 - #acbe4a
             184  316 - #ac3eca
             220  316 - #2dbeca
             268  316 - #ac3e4a
             296  316 - #2dbe4a
             316  316 - #2dbe4a
             352  316 - #2d3ebe
             368  316 - #2d3ebe
             392  316 - #acbe4a
             424  316 - #acbe4a
             456  316 - #ac3eca
             492  316 - #2dbeca
             536  316 - #ac3e4a
             564  316 - #ac3e4a
             164  324 - #00ffff
             200  328 - #ff0000
             588  328 - #0000e7
             136  332 - #ff00ff
             372  332 - #ffff00
             440  332 - #00ffff
             472  332 - #00ffff
               4  336 - #0000e7
             104  336 - #ff00ff
             408  336 - #ff00ff
             276  340 - #00ff00
             532  340 - #00ff00
             500  344 - #ff0000
             232  348 - #ff0000
             588  352 - #0000e7
              20  356 - #2d3ebe
              40  356 - #2d3ebe
              64  356 - #acbe4a
              88  356 - #acbe4a
             120  356 - #ac3eca
             152  356 - #2dbeca
             184  356 - #2dbeca
             208  356 - #ac3e4a
             256  356 - #2dbe4a
             296  356 - #2d3ebe
             316  356 - #2d3ebe
             344  356 - #acbe4a
             376  356 - #acbe4a
             396  356 - #ac3eca
             420  356 - #ac3eca
             444  356 - #2dbeca
             472  356 - #2dbeca
             552  356 - #2dbe4a
               8  368 - #ffff00
             520  368 - #00ff00
             168  372 - #ff0000
             236  372 - #597c95
             496  372 - #00ff00
             572  372 - #597c95
             100  376 - #00ffff
             136  376 - #00ffff
             212  376 - #00ff00
             328  376 - #ffff00
             264  380 - #0000e7
             296  380 - #ffff00
             392  380 - #00ffff
             448  380 - #ff0000
             472  380 - #ff0000
              64  384 - #ff00ff
             184  388 - #ff0000
             364  388 - #ff00ff
              24  396 - #acbe4a
              88  396 - #ac3eca
             112  396 - #2dbeca
             136  396 - #2dbeca
             156  396 - #ac3e4a
             204  396 - #2dbe4a
             232  396 - #2dbe4a
             280  396 - #2d3ebe
             312  396 - #acbe4a
             348  396 - #ac3eca
             416  396 - #2dbeca
             456  396 - #ac3e4a
             500  396 - #2dbe4a
             536  396 - #2d3ebe
             560  396 - #2d3ebe
             592  396 - #acbe4a
             332  400 - #597c95
             472  404 - #00ff00
               4  408 - #ff00ff
             260  408 - #ffff00
             396  408 - #ff0000
             444  408 - #00ff00
             292  412 - #ff00ff
             352  412 - #00ffff
              60  420 - #00ffff
             188  420 - #597c95
             316  420 - #ff00ff
             376  420 - #00ffff
             516  420 - #0000e7
             572  420 - #597c95
             160  428 - #00ff00
             240  428 - #597c95
               4  432 - #ff00ff
              96  432 - #597c95
              36  436 - #ac3eca
             132  436 - #ac3e4a
             180  436 - #2dbe4a
             212  436 - #2d3ebe
             264  436 - #acbe4a
             300  436 - #ac3eca
             348  436 - #2dbeca
             368  436 - #2dbeca
             404  436 - #ac3e4a
             440  436 - #2dbe4a
             464  436 - #2dbe4a
             488  436 - #2d3ebe
             508  436 - #2d3ebe
             552  436 - #acbe4a
             592  436 - #ac3eca
             280  444 - #ff00ff
             316  444 - #00ffff
              68  448 - #ff0000
             156  452 - #0000e7
             252  452 - #ff00ff
             572  452 - #597c95
             184  456 - #0000e7
             372  456 - #ff0000
               4  460 - #00ffff
             228  464 - #ffff00
             520  464 - #ffff00
             112  468 - #00ff00
             400  468 - #00ff00
             480  468 - #597c95
              28  472 - #00ffff
             436  472 - #0000e7
             592  472 - #00ffff
              76  476 - #ac3e4a
             152  476 - #2d3ebe
             200  476 - #acbe4a
             264  476 - #ac3eca
             300  476 - #2dbeca
             324  476 - #2dbeca
             348  476 - #ac3e4a
             460  476 - #2d3ebe
             500  476 - #acbe4a
             560  476 - #ac3eca
             376  484 - #00ff00
               8  488 - #ff0000
              52  488 - #00ff00
             180  488 - #ffff00
             240  488 - #597c95
             416  488 - #0000e7
             536  488 - #00ffff
             100  492 - #0000e7
             128  492 - #0000e7
             208  496 - #ff00ff
             488  496 - #ff00ff
              76  500 - #00ff00
             280  500 - #00ffff
             572  500 - #597c95
             516  504 - #ff00ff
             340  508 - #00ff00
             444  508 - #ffff00
               4  512 - #ff0000
             372  512 - #00ff00
              28  516 - #ac3e4a
              56  516 - #2dbe4a
             116  516 - #2d3ebe
             168  516 - #acbe4a
             200  516 - #ac3eca
             224  516 - #ac3eca
             248  516 - #2dbeca
             268  516 - #2dbeca
             308  516 - #ac3e4a
             356  516 - #2dbe4a
             408  516 - #2d3ebe
             464  516 - #acbe4a
             496  516 - #ac3eca
             552  516 - #2dbeca
             592  516 - #ac3e4a
              88  528 - #0000e7
             144  528 - #597c95
             480  528 - #597c95
             280  532 - #ff0000
             388  532 - #ffff00
             452  532 - #ff00ff
             572  532 - #597c95
             424  536 - #ffff00
             512  536 - #00ffff
               4  540 - #00ff00
              52  540 - #0000e7
             176  540 - #ff00ff
             232  540 - #00ffff
             340  540 - #0000e7
             200  544 - #00ffff
              28  556 - #2dbe4a
              72  556 - #2d3ebe
             116  556 - #acbe4a
             152  556 - #ac3eca
             212  556 - #2dbeca
             260  556 - #ac3e4a
             280  556 - #ac3e4a
             304  556 - #2dbe4a
             324  556 - #2dbe4a
             356  556 - #2d3ebe
             404  556 - #acbe4a
             440  556 - #ac3eca
             464  556 - #ac3eca
             488  556 - #2dbeca
             536  556 - #ac3e4a
             560  556 - #ac3e4a
             588  556 - #2dbe4a
             228  564 - #ff0000
             372  564 - #ffff00
             508  564 - #ff0000
               4  568 - #0000e7
             184  568 - #00ffff
             344  568 - #ffff00
             480  572 - #597c95
             544  580 - #00ff00
              28  584 - #0000e7
              84  584 - #ffff00
             140  584 - #597c95
             592  584 - #0000e7
               4  592 - #0000e7
              52  592 - #ffff00
             112  592 - #ff00ff
             168  592 - #00ffff
             208  592 - #ff0000
             248  592 - #00ff00
             288  592 - #597c95
             332  592 - #597c95
             356  592 - #ffff00
             380  592 - #597c95
             412  592 - #ff00ff
             448  592 - #00ffff
             492  592 - #ff0000
             516  592 - #ff0000
             568  592 - #00ff00
";
