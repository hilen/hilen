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
        // The grid is laid out in screen pixels, the same unit the test canvas
        // is fixed in, so it lands on the same 600 by 600 pixels on every
        // platform. The rect shader maps a coordinate to a pixel with the
        // window resolution and the per instance scale, so the resolution is
        // the real framebuffer and the scale is one, no display scale in it.
        // Reading the canvas render area and the display scale instead grows
        // the grid on a headed retina run and distorts it on a non square
        // framebuffer, and the probes miss it.
        let scale = 1.0;
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
                resolution: UIManager::window_resolution(),
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
              12    4 - #ff0000
              72    4 - #597c95
             136    4 - #00ffff
             168    4 - #597c95
             208    4 - #0000e7
             244    4 - #ff00ff
             280    4 - #00ffff
             340    4 - #0000e7
             380    4 - #ffff00
             424    4 - #00ffff
             472    4 - #00ff00
             572    4 - #00ffff
              36   12 - #00ff00
             108   12 - #ff00ff
             448   12 - #ff0000
             492   12 - #0000e7
             516   12 - #ffff00
             188   16 - #00ff00
             544   16 - #ff00ff
             152   20 - #ff0000
             224   20 - #ffff00
             312   24 - #597c95
               4   28 - #00ff00
             288   28 - #597c95
             468   28 - #0000e7
             568   28 - #ff0000
             592   28 - #00ff00
             376   32 - #ff00ff
             132   36 - #ff0000
             172   36 - #0000e7
             204   36 - #ffff00
             348   36 - #ffff00
             404   36 - #00ffff
             444   36 - #00ff00
             492   36 - #ffff00
             516   36 - #ff00ff
              72   40 - #597c95
             236   40 - #ff00ff
              40   44 - #0000e7
             104   44 - #00ffff
             268   44 - #ff0000
             548   44 - #00ffff
             296   52 - #0000e7
             468   52 - #ffff00
             424   56 - #00ff00
              12   60 - #0000e7
             156   60 - #0000e7
             180   60 - #ffff00
             252   60 - #ff0000
             324   60 - #ffff00
             348   60 - #ff00ff
             372   60 - #00ffff
             396   60 - #ff0000
             444   60 - #0000e7
             588   60 - #0000e7
             524   64 - #00ffff
              76   68 - #00ffff
             128   68 - #00ff00
             208   68 - #ff00ff
             560   68 - #00ff00
             496   72 - #597c95
             100   76 - #00ff00
             236   76 - #ff0000
             460   80 - #ff00ff
              52   84 - #00ffff
             156   84 - #ffff00
             180   84 - #ff00ff
             276   84 - #0000e7
             324   84 - #ff00ff
             348   84 - #00ffff
             420   84 - #0000e7
              16   88 - #ffff00
             300   88 - #ffff00
             388   88 - #00ff00
             544   88 - #00ff00
             584   92 - #ffff00
             116  100 - #0000e7
             144  100 - #597c95
             516  100 - #00ff00
              36  108 - #00ffff
              60  108 - #ff0000
              84  108 - #00ff00
             204  108 - #ff0000
             228  108 - #00ff00
             252  108 - #0000e7
             364  108 - #00ff00
             444  108 - #ff00ff
             484  108 - #ff0000
             540  108 - #0000e7
             564  108 - #ffff00
             176  112 - #00ffff
               4  116 - #ff00ff
             420  116 - #ffff00
             304  120 - #597c95
             388  120 - #597c95
             272  124 - #ff00ff
             336  124 - #597c95
             580  124 - #00ffff
             112  128 - #ffff00
             140  128 - #ff00ff
             548  128 - #ffff00
              60  132 - #00ff00
             204  132 - #00ff00
             252  132 - #ffff00
             372  132 - #0000e7
             444  132 - #00ffff
             468  132 - #ff0000
             492  132 - #00ff00
             516  132 - #0000e7
              32  136 - #ff0000
             164  136 - #00ffff
             232  140 - #0000e7
             404  140 - #ffff00
              88  148 - #ffff00
             348  148 - #0000e7
             200  152 - #0000e7
             372  152 - #ffff00
              12  156 - #ff0000
             108  156 - #ff00ff
             132  156 - #00ffff
             252  156 - #ff00ff
             276  156 - #00ffff
             300  156 - #ff0000
             324  156 - #00ff00
             540  156 - #ff00ff
             564  156 - #00ffff
              52  160 - #0000e7
             164  160 - #ff0000
             476  160 - #00ff00
             512  160 - #ffff00
             592  160 - #ff0000
             392  164 - #ff00ff
             416  164 - #00ffff
             444  168 - #597c95
              28  172 - #0000e7
             224  172 - #ff00ff
             340  172 - #ffff00
             112  176 - #00ffff
              84  180 - #ff00ff
             148  180 - #00ff00
             180  180 - #0000e7
             204  180 - #ffff00
             300  180 - #00ff00
             464  180 - #0000e7
             540  180 - #00ffff
             244  184 - #00ffff
              44  188 - #0000e7
             268  188 - #ff0000
             380  188 - #ff00ff
             508  188 - #ff00ff
             584  188 - #00ff00
             356  192 - #597c95
               4  196 - #0000e7
             448  196 - #0000e7
             480  196 - #597c95
             560  196 - #00ff00
             332  200 - #ffff00
              28  204 - #ffff00
              68  204 - #ff00ff
             108  204 - #ff0000
             180  204 - #ffff00
             204  204 - #ff00ff
             228  204 - #00ffff
             276  204 - #00ff00
             300  204 - #0000e7
             396  204 - #ff0000
             420  204 - #00ff00
             520  208 - #00ffff
             144  212 - #597c95
             256  212 - #ff0000
             464  212 - #ffff00
             544  216 - #597c95
             364  220 - #ff0000
             500  220 - #00ffff
             580  220 - #ffff00
               8  224 - #ffff00
              36  228 - #ff00ff
              60  228 - #00ffff
              84  228 - #ff0000
             120  228 - #597c95
             300  228 - #ffff00
             324  228 - #ff00ff
             396  228 - #00ff00
             420  228 - #0000e7
             444  228 - #ffff00
             188  232 - #ff00ff
             272  232 - #0000e7
             232  236 - #ff0000
             472  236 - #ff00ff
             532  236 - #00ff00
             560  236 - #0000e7
             160  244 - #ff00ff
             208  244 - #ff0000
              68  252 - #ff0000
             108  252 - #0000e7
             132  252 - #ffff00
             180  252 - #00ffff
             252  252 - #0000e7
             276  252 - #ffff00
             300  252 - #ff00ff
             324  252 - #00ffff
             348  252 - #ff0000
             372  252 - #00ff00
             396  252 - #0000e7
             420  252 - #ffff00
             444  252 - #ff00ff
             492  252 - #ff0000
             516  252 - #00ff00
             588  252 - #ff00ff
               4  256 - #ff00ff
             468  260 - #00ffff
             552  264 - #597c95
              44  268 - #ff0000
             204  268 - #00ff00
             236  268 - #0000e7
             108  276 - #ffff00
             132  276 - #ff00ff
             172  276 - #ff0000
             276  276 - #ff00ff
             300  276 - #00ffff
             324  276 - #ff0000
             348  276 - #00ff00
             372  276 - #0000e7
             396  276 - #ffff00
             420  276 - #ff00ff
             492  276 - #00ff00
             516  276 - #0000e7
              76  284 - #0000e7
             460  284 - #ff0000
             584  284 - #00ffff
              20  292 - #ff0000
              44  292 - #00ff00
             108  300 - #ff00ff
             132  300 - #00ffff
             156  300 - #ff0000
             204  300 - #0000e7
             228  300 - #ffff00
             252  300 - #ff00ff
             276  300 - #00ffff
             300  300 - #ff0000
             324  300 - #00ff00
             372  300 - #ffff00
             396  300 - #ff00ff
             420  300 - #00ffff
             444  300 - #ff0000
             468  300 - #00ff00
             516  300 - #ffff00
             540  300 - #ff00ff
             564  300 - #00ffff
             172  308 - #00ff00
             496  308 - #0000e7
              56  316 - #ffff00
             348  316 - #ffff00
             580  316 - #00ff00
              12  324 - #00ff00
              36  324 - #0000e7
              84  324 - #ff00ff
             108  324 - #00ffff
             192  324 - #597c95
             252  324 - #00ffff
             276  324 - #ff0000
             324  324 - #0000e7
             372  324 - #ff00ff
             396  324 - #00ffff
             420  324 - #ff0000
             444  324 - #00ff00
             468  324 - #0000e7
             516  324 - #ff00ff
             148  332 - #00ff00
             292  332 - #00ff00
             540  332 - #00ffff
             224  336 - #597c95
              60  344 - #ff00ff
             124  344 - #00ff00
             592  344 - #0000e7
              28  348 - #ffff00
              88  348 - #00ffff
             180  348 - #ffff00
             204  348 - #ff00ff
             252  348 - #ff0000
             300  348 - #0000e7
             348  348 - #ff00ff
             372  348 - #00ffff
             396  348 - #ff0000
             420  348 - #00ff00
             444  348 - #0000e7
             484  348 - #ff00ff
             516  348 - #00ffff
             564  348 - #00ff00
             268  356 - #00ff00
             316  356 - #ffff00
             540  356 - #ff0000
               4  360 - #597c95
              44  364 - #ff00ff
             236  364 - #ff0000
             292  364 - #ffff00
              84  372 - #ff0000
             124  372 - #0000e7
             156  372 - #ffff00
             180  372 - #ff00ff
             204  372 - #00ffff
             348  372 - #00ffff
             372  372 - #ff0000
             396  372 - #00ff00
             420  372 - #0000e7
             444  372 - #ffff00
             468  372 - #ff00ff
             492  372 - #00ffff
             588  372 - #ffff00
             256  380 - #00ff00
             516  380 - #ff0000
             548  380 - #00ff00
             300  388 - #ff00ff
              52  392 - #ff0000
              12  396 - #ff00ff
              84  396 - #00ff00
             108  396 - #0000e7
             132  396 - #ffff00
             180  396 - #00ffff
             204  396 - #ff0000
             228  396 - #00ff00
             324  396 - #00ffff
             348  396 - #ff0000
             372  396 - #00ff00
             420  396 - #ffff00
             444  396 - #ff00ff
             468  396 - #00ffff
             492  396 - #ff0000
             156  404 - #ff00ff
             268  404 - #ffff00
             520  404 - #00ff00
             572  404 - #ffff00
              64  412 - #00ff00
             392  412 - #ffff00
             440  416 - #00ffff
             180  420 - #ff0000
             240  420 - #597c95
             300  420 - #00ffff
             324  420 - #ff0000
             348  420 - #00ff00
             372  420 - #0000e7
             416  420 - #ff00ff
             468  420 - #ff0000
             540  420 - #ffff00
             592  420 - #00ffff
              44  424 - #ff0000
              88  424 - #0000e7
             144  424 - #597c95
             508  424 - #0000e7
               4  428 - #00ffff
             116  428 - #ffff00
             212  428 - #00ff00
             564  428 - #ff00ff
             396  436 - #ff00ff
             484  436 - #0000e7
              36  444 - #00ff00
              60  444 - #0000e7
             132  444 - #00ffff
             156  444 - #ff0000
             180  444 - #00ff00
             252  444 - #ff00ff
             276  444 - #00ffff
             300  444 - #ff0000
             324  444 - #00ff00
             348  444 - #0000e7
             420  444 - #00ffff
             588  444 - #ff0000
             528  448 - #597c95
              80  452 - #ffff00
             380  452 - #ffff00
             460  452 - #00ff00
             500  452 - #0000e7
             556  452 - #00ffff
              20  460 - #00ff00
             112  460 - #00ffff
             220  460 - #ff00ff
             436  464 - #00ff00
              60  468 - #ffff00
             180  468 - #0000e7
             276  468 - #ff0000
             332  468 - #0000e7
             480  468 - #597c95
             540  468 - #00ffff
             360  472 - #597c95
             572  472 - #ff0000
             140  476 - #ff0000
             248  476 - #00ffff
             520  476 - #ff00ff
             304  480 - #597c95
             408  480 - #597c95
               4  488 - #0000e7
              80  488 - #00ffff
             164  488 - #0000e7
             548  488 - #ff0000
              44  492 - #ffff00
             108  492 - #ff0000
             196  492 - #ff00ff
             332  492 - #ffff00
             384  492 - #597c95
             444  492 - #0000e7
             468  492 - #ffff00
             492  492 - #ff00ff
             592  492 - #0000e7
             224  496 - #00ffff
             284  500 - #00ff00
             356  500 - #ff00ff
             148  508 - #ffff00
             520  508 - #ff0000
             124  512 - #0000e7
             316  512 - #ff00ff
             572  512 - #0000e7
              12  516 - #ffff00
              76  516 - #ff0000
             180  516 - #ff00ff
             204  516 - #00ffff
             252  516 - #00ff00
             396  516 - #00ff00
             420  516 - #0000e7
              40  520 - #ff00ff
             496  520 - #00ffff
             224  524 - #ff0000
             444  524 - #ffff00
             548  524 - #00ff00
             104  528 - #597c95
             160  532 - #ff00ff
             372  532 - #00ff00
             472  532 - #00ffff
             304  536 - #ff00ff
             344  536 - #ff0000
              68  540 - #ff0000
             132  540 - #ffff00
             204  540 - #ff0000
             252  540 - #0000e7
             276  540 - #ffff00
             324  540 - #00ffff
             396  540 - #0000e7
             420  540 - #ffff00
             492  540 - #ff0000
             516  540 - #00ff00
             588  540 - #ff00ff
              16  544 - #ff00ff
              44  548 - #00ffff
             172  556 - #ff0000
             232  556 - #0000e7
             556  556 - #ff00ff
              60  564 - #00ff00
              84  564 - #0000e7
             108  564 - #ffff00
             144  564 - #597c95
             260  564 - #ffff00
             292  564 - #00ffff
             320  564 - #ff0000
             348  564 - #00ff00
             372  564 - #0000e7
             396  564 - #ffff00
             420  564 - #ff00ff
             444  564 - #00ffff
             468  564 - #ff0000
             492  564 - #00ff00
             580  564 - #00ffff
               4  572 - #00ffff
             524  572 - #0000e7
             212  576 - #597c95
             592  584 - #597c95
              36  588 - #00ff00
              84  588 - #ffff00
             152  588 - #ff0000
             276  588 - #00ffff
             432  588 - #597c95
              60  592 - #0000e7
             116  592 - #ff00ff
             188  592 - #00ff00
             236  592 - #ffff00
             308  592 - #ff0000
             344  592 - #0000e7
             376  592 - #ffff00
             408  592 - #597c95
             460  592 - #597c95
             496  592 - #597c95
             552  592 - #597c95
";
