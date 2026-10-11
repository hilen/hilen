use std::fmt::Write;

use anyhow::{Result, bail};
use hilen::{
    gm::LossyConvert,
    refs::Weak,
    ui::{
        Color, Container, DrawingView, FillRule, Image, ImageView, Label, Setup, TextAlignment, U8Color,
        VectorPath, View, ViewData, ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{capture_screenshot, check_colors, checkpoint, set_record_probe_count},
    window::Screenshot,
};

const PROBES: &str = r"
    4    4 - #f1f4f9
  136    4 - #f1f4f9
  192    4 - #f1f4f9
  244    4 - #f1f4f9
  304    4 - #f1f4f9
  400    4 - #f1f4f9
  452    4 - #f1f4f9
  512    4 - #f1f4f9
  592    4 - #f1f4f9
  352    8 - #f1f4f9
   60   28 - #333333
   68   28 - #f1f4f9
   80   28 - #555556
   52   32 - #333333
   60   32 - #343434
   84   32 - #9d9ea1
   88   32 - #595a5b
  216   32 - #f1f4f9
  488   36 - #f1f4f9
  552   40 - #f1f4f9
   12   48 - #f1f4f9
  392   48 - #f1f4f9
  260   52 - #333333
   76   56 - #f1f4f9
  132   56 - #f1f4f9
  140   56 - #f1f4f9
  244   56 - #797a7c
  248   56 - #414142
  260   56 - #333333
  292   56 - #f1f4f9
  296   56 - #333333
  312   56 - #404040
  332   56 - #404040
  344   56 - #d9dce0
  348   56 - #707173
  456   56 - #f1f4f9
  192   60 - #f1f4f9
  592   64 - #f1f4f9
  508   72 - #101014
  104   76 - #101014
  168   80 - #101014
    4   88 - #f1f4f9
   48   88 - #101014
  220   88 - #101014
  368   88 - #101014
  480   88 - #101014
  136   92 - #101014
  412   92 - #101014
  556   92 - #101014
  260   96 - #101014
  328   96 - #101014
  448   96 - #101014
  524  104 - #101014
  172  112 - #101014
  592  112 - #f1f4f9
  108  116 - #2fa866
  292  116 - #2fa866
   92  120 - #2fa866
  476  120 - #2fa866
  496  120 - #2fa866
   24  124 - #f1f4f9
  232  124 - #101014
  308  124 - #2fa866
   64  128 - #101014
  276  128 - #2fa866
  368  128 - #101014
   80  132 - #2fa866
  128  132 - #2fa866
  460  132 - #2fa866
  112  136 - #2fa866
  320  136 - #2fa866
  416  136 - #101014
  512  136 - #2fa866
  544  136 - #101014
   92  140 - #2fa866
  296  140 - #2fa866
  156  144 - #101014
  268  144 - #2fa866
  284  144 - #2fa866
  484  144 - #2fa866
  500  144 - #2fa866
   76  148 - #2fa866
  308  148 - #2fa866
  108  152 - #2fa866
  460  152 - #2fa866
  588  152 - #f1f4f9
  128  156 - #2fa866
  288  156 - #2fa866
  320  156 - #2fa866
  476  156 - #2fa866
  492  156 - #2fa866
  236  160 - #101014
  508  160 - #2fa866
    4  164 - #f1f4f9
  276  164 - #2fa866
  300  164 - #2fa866
  388  164 - #f1f4f9
   44  168 - #101014
   88  168 - #2fa866
  176  168 - #101014
  468  168 - #2fa866
  112  172 - #2fa866
  288  172 - #2fa866
  348  172 - #101014
  488  172 - #2fa866
  556  176 - #101014
  424  180 - #101014
  148  184 - #101014
  252  188 - #101014
  220  192 - #101014
  504  200 - #101014
   68  204 - #101014
  316  204 - #101014
   32  208 - #101014
  396  208 - #f1f4f9
  112  212 - #101014
  536  212 - #101014
  156  216 - #101014
  276  216 - #101014
  356  216 - #101014
  440  216 - #101014
  476  216 - #101014
  592  220 - #f1f4f9
    4  232 - #f1f4f9
  200  232 - #f1f4f9
  240  236 - #f1f4f9
   84  240 - #f1f4f9
   44  244 - #f1f4f9
  328  244 - #f1f4f9
  416  244 - #f1f4f9
  372  260 - #f1f4f9
  496  260 - #f1f4f9
  552  260 - #f1f4f9
  452  264 - #f1f4f9
    8  276 - #f1f4f9
  120  276 - #a6a8ab
  180  276 - #707173
  236  276 - #333333
  272  276 - #ccced2
   52  280 - #333333
   68  280 - #343434
   72  280 - #f1f4f9
   92  280 - #b2b4b8
  100  280 - #333333
  104  280 - #707173
  120  280 - #a6a8ab
  140  280 - #f1f4f9
  144  280 - #343434
  164  280 - #d5d8dc
  168  280 - #343434
  180  280 - #707173
  184  280 - #ced0d5
  200  280 - #d8dbdf
  208  280 - #f1f4f9
  212  280 - #333333
  216  280 - #4c4c4d
  224  280 - #f1f4f9
  236  280 - #333333
  260  280 - #d3d6da
  268  280 - #494a4a
  272  280 - #ccced2
  284  280 - #707173
  296  280 - #f1f4f9
  300  280 - #333333
  308  280 - #a9abae
  316  280 - #f1f4f9
  400  288 - #f1f4f9
  592  288 - #f1f4f9
  524  300 - #8d8f91
   56  304 - #f1f4f9
   96  304 - #f1f4f9
  124  304 - #8b8c8f
  148  304 - #f1f4f9
  252  304 - #4d4d4e
  300  304 - #848588
  316  304 - #f1f4f9
  336  304 - #f1f4f9
  352  304 - #333333
  360  304 - #363636
  436  304 - #f1f4f9
  476  304 - #f1f4f9
  520  304 - #a9abae
  528  304 - #333333
   80  308 - #f1f4f9
   92  308 - #f1f4f9
  100  308 - #f1f4f9
  128  308 - #f1f4f9
  296  308 - #f1f4f9
  308  308 - #f1f4f9
  324  308 - #f1f4f9
  460  308 - #f1f4f9
  472  308 - #f1f4f9
  480  308 - #f1f4f9
    4  324 - #f1f4f9
  272  332 - #101014
  108  336 - #101014
  140  336 - #101014
  444  336 - #101014
   72  340 - #101014
  412  340 - #101014
  476  340 - #101014
  508  340 - #101014
  172  344 - #101014
  332  344 - #101014
  548  344 - #101014
   40  348 - #101014
  220  348 - #101014
  592  352 - #f1f4f9
  368  356 - #101014
   88  368 - #2fa866
  104  368 - #2fa866
  132  368 - #2fa866
  272  368 - #2fa866
  304  368 - #2fa866
  320  368 - #2fa866
  464  368 - #2fa866
  164  372 - #101014
  288  372 - #2fa866
  488  372 - #2fa866
   76  376 - #2fa866
  240  376 - #101014
  512  376 - #2fa866
  552  376 - #101014
    4  380 - #f1f4f9
  472  380 - #2fa866
   40  384 - #101014
   92  384 - #2fa866
  124  384 - #2fa866
  276  384 - #2fa866
  308  384 - #2fa866
  324  384 - #2fa866
  412  384 - #101014
  496  384 - #2fa866
  360  388 - #101014
  292  392 - #2fa866
  460  392 - #2fa866
  592  392 - #f1f4f9
  108  396 - #2fa866
  280  396 - #2fa866
  484  396 - #2fa866
   84  400 - #2fa866
  132  400 - #2fa866
  176  400 - #101014
  312  400 - #2fa866
  508  400 - #2fa866
  268  404 - #2fa866
  296  404 - #2fa866
  324  404 - #2fa866
  468  404 - #2fa866
   96  408 - #2fa866
  120  408 - #2fa866
  232  408 - #101014
  284  408 - #2fa866
  492  408 - #2fa866
  556  408 - #101014
  300  416 - #2fa866
  456  416 - #2fa866
   32  420 - #101014
  104  420 - #2fa866
  268  420 - #2fa866
  320  420 - #2fa866
  480  420 - #2fa866
   80  424 - #2fa866
  128  424 - #2fa866
  284  424 - #2fa866
  368  424 - #101014
  416  424 - #101014
  504  424 - #2fa866
  204  436 - #f1f4f9
  592  436 - #f1f4f9
  536  440 - #101014
  440  444 - #101014
    4  448 - #f1f4f9
  168  452 - #101014
  132  460 - #101014
   80  464 - #101014
  236  464 - #101014
  324  464 - #101014
  464  464 - #101014
   40  468 - #101014
  280  468 - #101014
  368  468 - #101014
  412  468 - #101014
  508  468 - #101014
  556  468 - #101014
  108  488 - #f1f4f9
  192  496 - #f1f4f9
  528  504 - #f1f4f9
  592  504 - #f1f4f9
    4  508 - #f1f4f9
  264  512 - #f1f4f9
  424  512 - #f1f4f9
  472  512 - #f1f4f9
  148  516 - #f1f4f9
  336  516 - #f1f4f9
   88  524 - #f1f4f9
  300  528 - #f1f4f9
  384  532 - #f1f4f9
   48  536 - #f1f4f9
  552  536 - #f1f4f9
  224  540 - #f1f4f9
  508  540 - #f1f4f9
    8  548 - #f1f4f9
  592  548 - #f1f4f9
  420  552 - #f1f4f9
  116  556 - #f1f4f9
  176  556 - #f1f4f9
  268  564 - #f1f4f9
  320  564 - #f1f4f9
  464  564 - #f1f4f9
  552  580 - #f1f4f9
  368  588 - #f1f4f9
  508  588 - #f1f4f9
    4  592 - #f1f4f9
   68  592 - #f1f4f9
  148  592 - #f1f4f9
  224  592 - #f1f4f9
  296  592 - #f1f4f9
  420  592 - #f1f4f9
  592  592 - #f1f4f9
";

const BACKGROUND: Color = Color::hex("#f1f4f9");
const PICTURE: Color = Color::hex("#101014");
const SHAPE: Color = Color::hex("#2fa866");
const TEXT: Color = Color::hex("#333333");

const TILE: f32 = 150.0;
const SHAPE_SIDE: f32 = 60.0;
const PICTURES: [&str; 3] = ["image", "gradient", "path"];
const SHAPES: [&str; 2] = ["circle", "square"];
const ROWS: [&str; 2] = ["a round shape", "a square shape, half a pixel off the grid"];

/// A plain colored view in front of a picture, like a round badge on a
/// poster. The pixels on its edge are partly the view and partly the
/// picture behind it. They must come out as a mix of those 2, with
/// nothing of the page background in them.
///
/// A flush drew every plain view before every image, gradient and path.
/// The edge of the badge then mixed with the page background, the only
/// thing drawn so far, and its depth hid the picture there. On a dark
/// poster over a light page that left a torn light rim around the badge.
///
/// A round shape always has such pixels. A square has them only when its
/// edge lands between pixels, so the square sits half a pixel off the grid.
#[view]
struct RectOverPicture {}

fn tile_origin(picture: usize, shape: usize) -> (f32, f32) {
    let picture: f32 = picture.lossy_convert();
    let shape: f32 = shape.lossy_convert();
    (30.0 + 190.0 * picture, 70.0 + 250.0 * shape)
}

/// The frame of the shape in front of a tile.
fn shape_frame(picture: usize, shape: usize) -> (f32, f32, f32, f32) {
    let (x, y) = tile_origin(picture, shape);
    let inset = (TILE - SHAPE_SIDE) / 2.0;
    let off = if shape == 0 { 0.0 } else { 0.5 };
    (x + inset + off, y + inset + off, SHAPE_SIDE, SHAPE_SIDE)
}

fn solid_image() -> Weak<Image> {
    let px = U8Color::from(PICTURE);
    let pixels = [[px.r, px.g, px.b, px.a]; 16].concat();
    Image::from_raw_data(pixels, "rect_over_picture", (4, 4).into(), 4)
}

fn add_picture(host: Weak<RectOverPicture>, picture: usize, frame: (f32, f32, f32, f32)) {
    let view: Weak<dyn View> = match picture {
        0 => {
            let image = host.add_view::<ImageView>();
            image.set_image(solid_image());
            image.weak_view()
        }
        1 => {
            let gradient = host.add_view::<Container>();
            gradient.set_gradient(PICTURE, PICTURE);
            gradient.weak_view()
        }
        _ => {
            let mut drawing = host.add_view::<DrawingView>();
            let square = VectorPath::polygon([(0.0, 0.0), (TILE, 0.0), (TILE, TILE), (0.0, TILE)]);
            drawing.add_fill(&square, PICTURE, FillRule::NonZero);
            drawing.weak_view()
        }
    };
    view.set_frame(frame);
}

impl Setup for RectOverPicture {
    fn setup(self: Weak<Self>) {
        self.set_color(BACKGROUND);

        for (shape, shape_name) in SHAPES.iter().enumerate() {
            let (x, y) = tile_origin(0, shape);
            let row = self.add_view::<Label>();
            row.set_text(ROWS[shape])
                .set_text_size(16)
                .set_text_color(TEXT)
                .set_alignment(TextAlignment::Left)
                .set_frame((x, y - 52.0, 540, 22));

            for (picture, picture_name) in PICTURES.iter().enumerate() {
                let (x, y) = tile_origin(picture, shape);

                let title = self.add_view::<Label>();
                title
                    .set_text(format!("{shape_name} over {picture_name}"))
                    .set_text_size(14)
                    .set_text_color(TEXT)
                    .set_alignment(TextAlignment::Left)
                    .set_frame((x, y - 26.0, 180, 20));

                add_picture(self, picture, (x, y, TILE, TILE));

                let front = self.add_view::<Container>();
                front.set_color(SHAPE).set_frame(shape_frame(picture, shape));
                if shape == 0 {
                    front.set_corner_radius(SHAPE_SIDE / 2.0);
                }
            }
        }
    }
}

fn channels(color: Color) -> [f32; 3] {
    [color.r * 255.0, color.g * 255.0, color.b * 255.0]
}

/// Every pixel of a shape and a 2 pixel ring around it has to lie between
/// the picture and the shape, channel by channel. Returns the worst
/// offender.
fn worst_pixel(shot: &Screenshot, frame: (f32, f32, f32, f32)) -> Option<(u32, u32, U8Color, f32)> {
    const TOLERANCE: f32 = 3.0;

    let mixes = [channels(PICTURE), channels(SHAPE)];
    let (x, y, w, h) = frame;
    let mut worst: Option<(u32, u32, U8Color, f32)> = None;

    let left: u32 = (x.floor() - 2.0).lossy_convert();
    let right: u32 = ((x + w).ceil() + 2.0).lossy_convert();
    let top: u32 = (y.floor() - 2.0).lossy_convert();
    let bottom: u32 = ((y + h).ceil() + 2.0).lossy_convert();

    for py in top..=bottom {
        for px in left..=right {
            let pixel = shot.get_pixel((px, py));
            let got = [f32::from(pixel.r), f32::from(pixel.g), f32::from(pixel.b)];
            let mut miss = 0.0f32;

            for channel in 0..3 {
                let low = mixes.iter().map(|m| m[channel]).fold(f32::MAX, f32::min);
                let high = mixes.iter().map(|m| m[channel]).fold(f32::MIN, f32::max);
                miss = miss.max(low - got[channel]).max(got[channel] - high);
            }

            if miss > TOLERANCE && worst.is_none_or(|(_, _, _, m)| miss > m) {
                worst = Some((px, py, pixel, miss));
            }
        }
    }

    worst
}

impl ViewTest for RectOverPicture {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(320);

        checkpoint("each green shape has a clean edge on its dark picture, no light rim")?;

        let shot = capture_screenshot()?;
        let mut report = String::new();

        for (shape, shape_name) in SHAPES.iter().enumerate() {
            for (picture, picture_name) in PICTURES.iter().enumerate() {
                if let Some((x, y, got, miss)) = worst_pixel(&shot, shape_frame(picture, shape)) {
                    writeln!(
                        report,
                        "{shape_name} over {picture_name}: pixel {x} {y} is {}, {miss:.0} out of the mixable range",
                        got.as_hex(),
                    )?;
                }
            }
        }

        if !report.is_empty() {
            bail!("edge pixels that hold the page background:\n{report}");
        }

        check_colors(PROBES)?;

        Ok(())
    }
}
