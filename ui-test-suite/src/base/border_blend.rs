use std::fmt::Write;

use anyhow::{Result, bail};
use hilen::{
    gm::LossyConvert,
    refs::Weak,
    ui::{
        BlurView, CLEAR, Color, Container, Image, ImageView, Label, Setup, U8Color, View, ViewData,
        ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{capture_screenshot, check_colors, checkpoint, set_record_probe_count},
};

const PROBES: &str = r"
    4    4 - #ededed
   60    4 - #ededed
  336    4 - #ededed
  384    4 - #ededed
  436    4 - #ededed
  488    4 - #ededed
  560    4 - #ededed
  148   16 - #3f3f3f
  192   16 - #6e6e6e
  196   16 - #adadad
  228   16 - #a8a8a8
  244   16 - #4a4a4a
  252   16 - #333333
  144   20 - #ededed
  148   20 - #3f3f3f
  156   20 - #959595
  168   20 - #333333
  172   20 - #333333
  184   20 - #a7a7a7
  188   20 - #333333
  192   20 - #6e6e6e
  196   20 - #adadad
  220   20 - #ededed
  228   20 - #a8a8a8
  240   20 - #ededed
  244   20 - #333333
  252   20 - #333333
  256   20 - #ededed
  264   20 - #ededed
  272   20 - #606060
  280   20 - #ededed
  284   20 - #333333
  292   20 - #959595
  148   24 - #3f3f3f
  168   24 - #333333
  172   24 - #343434
  184   24 - #a7a7a7
  188   24 - #333333
  192   24 - #6e6e6e
  196   24 - #adadad
  228   24 - #a8a8a8
  252   24 - #333333
  272   24 - #606060
  464   44 - #ededed
  104   48 - #333333
  236   48 - #ababab
  512   48 - #3f3f3f
   84   52 - #333333
   92   52 - #ededed
  104   52 - #333333
  236   52 - #ababab
  356   52 - #c3c3c3
  360   52 - #3f3f3f
  364   52 - #a6a6a6
  512   52 - #3f3f3f
  520   52 - #333333
  524   52 - #adadad
  592   56 - #ededed
  156   72 - #ededed
  288   76 - #e5e5e8
  432   80 - #ededed
   16   84 - #ededed
  332   92 - #ededed
  212  100 - #ededed
  488  104 - #ededed
  592  108 - #ededed
  256  112 - #ededed
  164  120 - #ededed
   88  124 - #ededed
  372  124 - #ededed
  300  128 - #ededed
  544  128 - #ededed
   40  132 - #ececed
  228  148 - #ededed
  440  152 - #ededed
    4  164 - #ededed
  148  168 - #ededed
   64  192 - #597c95
  104  192 - #597c95
  164  192 - #597c95
  200  192 - #597c95
  228  192 - #597c95
  256  192 - #597c95
  300  192 - #597c95
  348  192 - #597c95
  388  192 - #597c95
  460  192 - #597c95
  492  192 - #597c95
  592  192 - #597c95
   24  196 - #597c95
  432  196 - #597c95
  540  196 - #597c95
  132  216 - #b3b2b0
  244  216 - #868583
  264  216 - #7c7c7b
  276  216 - #a3a2a0
  132  220 - #b3b2b0
  144  220 - #5f5e5d
  156  220 - #f6f4f1
  160  220 - #f6f4f1
  168  220 - #333333
  172  220 - #4f4e4e
  180  220 - #f6f4f1
  184  220 - #333333
  212  220 - #838281
  216  220 - #b3b2b0
  228  220 - #5a5a59
  240  220 - #f6f4f1
  276  220 - #a3a2a0
  280  220 - #f6f4f1
  316  220 - #9a9998
  132  224 - #b3b2b0
  140  224 - #dbdad7
  172  224 - #4f4e4e
  212  224 - #838281
  216  224 - #b3b2b0
  224  224 - #dcdad8
  244  224 - #868583
  276  224 - #a3a2a0
  324  224 - #cecdca
    4  228 - #f6f4f1
  568  236 - #f6f4f1
  104  248 - #333333
  236  248 - #b1b0ae
  416  248 - #f6f4f1
  512  248 - #404040
   84  252 - #333333
   92  252 - #f6f4f1
  104  252 - #333333
  236  252 - #b1b0ae
  356  252 - #cac9c7
  360  252 - #404040
  364  252 - #acaba9
  512  252 - #404040
  520  252 - #333333
  524  252 - #b3b1af
  592  276 - #f6f4f1
   40  280 - #dbd9d6
  180  280 - #dbd9d6
  320  280 - #dbd9d6
  460  280 - #dbd9d6
   40  284 - #dbd9d6
  180  284 - #dbd9d6
  320  284 - #dbd9d6
  460  284 - #dbd9d6
   40  288 - #dbd9d6
  180  288 - #dbd9d6
  320  288 - #dbd9d6
  460  288 - #dbd9d6
   40  292 - #dbd9d6
  180  292 - #dbd9d6
  268  292 - #ffffff
  320  292 - #dbd9d6
  460  292 - #dbd9d6
   40  296 - #dbd9d6
  128  296 - #ffffff
  180  296 - #dbd9d6
  320  296 - #dbd9d6
  460  296 - #dbd9d6
   40  300 - #dbd9d6
  180  300 - #dbd9d6
  320  300 - #dbd9d6
  460  300 - #dbd9d6
   40  304 - #dbd9d6
  180  304 - #dbd9d6
  320  304 - #dbd9d6
  460  304 - #dbd9d6
  388  320 - #f6f4f1
  528  324 - #f6f4f1
  108  340 - #ffffff
  248  340 - #ffffff
    4  344 - #f6f4f1
  348  344 - #ffffff
   52  348 - #ffffff
  432  348 - #f6f4f1
  484  348 - #ffffff
  592  352 - #f6f4f1
  160  356 - #f6f4f1
  304  360 - #f6f4f1
   64  392 - #597c95
   96  392 - #597c95
  132  392 - #597c95
  168  392 - #597c95
  200  392 - #597c95
  244  392 - #597c95
  300  392 - #597c95
  340  392 - #597c95
  376  392 - #597c95
  408  392 - #597c95
  460  392 - #597c95
  540  392 - #597c95
  580  392 - #597c95
    4  396 - #597c95
   36  396 - #597c95
  488  396 - #597c95
  436  400 - #131211
  120  416 - #d5d5d4
  140  416 - #dddddd
  228  416 - #dddddd
  256  416 - #9e9d9d
  276  416 - #696968
  116  420 - #131211
  120  420 - #d5d5d5
  140  420 - #dddddd
  156  420 - #131211
  160  420 - #131211
  172  420 - #5c5b5a
  228  420 - #dddddd
  248  420 - #131211
  252  420 - #dddddd
  268  420 - #6a6a69
  276  420 - #696968
  280  420 - #131211
  316  420 - #727271
  120  424 - #d5d5d4
  140  424 - #dddddd
  172  424 - #5c5b5a
  224  424 - #939393
  256  424 - #9e9d9d
  276  424 - #696968
  324  424 - #3c3b3b
  388  424 - #131211
  552  428 - #131211
  592  428 - #131211
   36  432 - #131211
  468  436 - #131211
  104  448 - #dddddd
  236  448 - #5a5a59
  416  448 - #131211
  512  448 - #d0cfcf
   84  452 - #dddddd
   92  452 - #131211
  104  452 - #dddddd
  200  452 - #131211
  236  452 - #5a5a59
  300  452 - #131211
  356  452 - #403f3f
  360  452 - #d0cfcf
  364  452 - #605f5f
  512  452 - #d0cfcf
  520  452 - #dddddd
  524  452 - #595857
    4  460 - #131211
  144  464 - #131211
  580  464 - #131211
   40  480 - #282726
  180  480 - #282726
  320  480 - #282726
  460  480 - #282726
   40  484 - #282726
  180  484 - #282726
  272  484 - #201e1c
  320  484 - #282726
  460  484 - #282726
   40  488 - #282726
  180  488 - #282726
  320  488 - #282726
  416  488 - #201e1c
  460  488 - #282726
  548  488 - #201e1c
   40  492 - #282726
  180  492 - #282726
  224  492 - #201e1c
  320  492 - #282726
  360  492 - #201e1c
  460  492 - #282726
  500  492 - #201e1c
    4  496 - #131211
   40  496 - #282726
  180  496 - #282726
  320  496 - #282726
  460  496 - #282726
   40  500 - #282726
  180  500 - #282726
  320  500 - #282726
  460  500 - #282726
   40  504 - #282726
  124  504 - #201e1c
  180  504 - #282726
  320  504 - #282726
  460  504 - #282726
  592  504 - #131211
   84  516 - #131211
  280  516 - #131211
  392  520 - #131211
  524  524 - #131211
  248  528 - #201e1c
  432  528 - #131211
    4  540 - #131211
  132  540 - #201e1c
  480  544 - #201e1c
  580  544 - #131211
   52  548 - #201e1c
  288  548 - #201e1c
  368  548 - #201e1c
  168  552 - #131211
  212  552 - #201e1c
  100  556 - #201e1c
  332  556 - #201e1c
  404  556 - #201e1c
  512  556 - #201e1c
  440  564 - #131211
  544  568 - #201e1c
  376  588 - #131211
  500  588 - #131211
  592  588 - #131211
    4  592 - #597c95
   32  592 - #597c95
   60  592 - #597c95
  104  592 - #597c95
  140  592 - #597c95
  168  592 - #597c95
  196  592 - #597c95
  244  592 - #597c95
  288  592 - #597c95
  328  592 - #597c95
  416  592 - #597c95
  464  592 - #597c95
  528  592 - #597c95
  560  592 - #597c95
";

const TILE_W: f32 = 110.0;
const TILE_H: f32 = 45.0;
const COLUMNS: [&str; 4] = ["rect", "gradient", "image", "blur"];

/// A bordered shape whose fill and border are not both opaque, the look
/// of real app cards and outlined buttons. The pixels on the inner edge
/// of the border are partly border and partly fill. They must come out
/// as a mix of the two over the background, never darker or brighter
/// than every color that could be mixed there.
///
/// Mixing the border and fill colors without weighting them by their
/// alpha broke that. A clear fill is black with zero alpha, so a solid
/// border on a clear button faded toward black on its inner edge. A
/// faint border on an opaque card let the border color win far more than
/// its alpha, so light cards got dark corners and dark cards got bright
/// ones. The blur pipeline dropped the border alpha altogether.
///
/// The rounded corners always have such pixels. A straight edge has them
/// only when the border lands between pixels, so every case is drawn
/// twice, on the pixel grid and half a pixel off it.
#[view]
struct BorderBlend {
    #[init]
    clear_fill: Container,
    light_card: Container,
    dark_card:  Container,
}

struct Case {
    name:   &'static str,
    bg:     Color,
    fill:   Color,
    border: Color,
    text:   Color,
}

/// Outlined toolbar button, a solid border around a clear fill.
const CLEAR_FILL: Case = Case {
    name:   "clear fill, solid border",
    bg:     Color::hex("#ededed"),
    fill:   CLEAR,
    border: Color::hex("#e0e0e6"),
    text:   Color::hex("#333333"),
};

/// Light card, a faint dark border around an opaque white fill.
const LIGHT_CARD: Case = Case {
    name:   "light card, faint dark border",
    bg:     Color::hex("#f6f4f1"),
    fill:   Color::hex("#ffffff"),
    border: Color::hex("#14110d").with_alpha(0.12),
    text:   Color::hex("#333333"),
};

/// Dark card, a faint light border around an opaque dark fill.
const DARK_CARD: Case = Case {
    name:   "dark card, faint light border",
    bg:     Color::hex("#131211"),
    fill:   Color::hex("#201e1c"),
    border: Color::hex("#ffffff").with_alpha(0.09),
    text:   Color::hex("#dddddd"),
};

const CASES: [&Case; 3] = [&CLEAR_FILL, &LIGHT_CARD, &DARK_CARD];

fn column_x(column: usize) -> f32 {
    let column: f32 = column.lossy_convert();
    40.0 + 140.0 * column
}

/// The tile frames of one row, first on the pixel grid, then half a
/// pixel off it.
fn tile_frames(column: usize) -> [(f32, f32, f32, f32); 2] {
    let x = column_x(column);
    [(x, 70.0, TILE_W, TILE_H), (x + 0.5, 125.5, TILE_W, TILE_H)]
}

fn label(host: Weak<Container>, text: &str, color: Color, size: f32) -> Weak<Label> {
    let label = host.add_view::<Label>();
    label.set_text(text).set_text_size(size).set_text_color(color);
    label
}

fn solid_image(color: Color, name: &str) -> Weak<Image> {
    let px = U8Color::from(color);
    let pixels = [[px.r, px.g, px.b, px.a]; 16].concat();
    Image::from_raw_data(pixels, name, (4, 4).into(), 4)
}

fn build_row(panel: Weak<Container>, case: &Case, index: usize) {
    panel.set_color(case.bg);
    label(panel, case.name, case.text, 18.0).set_frame((20, 8, 400, 24));

    for (column, name) in COLUMNS.iter().enumerate() {
        label(panel, name, case.text, 14.0).set_frame((column_x(column), 40, TILE_W, 20));

        for (half, frame) in tile_frames(column).into_iter().enumerate() {
            let tile: Weak<dyn View> = match column {
                0 => {
                    let rect = panel.add_view::<Container>();
                    rect.set_color(case.fill);
                    rect.weak_view()
                }
                1 => {
                    let gradient = panel.add_view::<Container>();
                    gradient.set_gradient(case.fill, case.fill);
                    gradient.weak_view()
                }
                2 => {
                    let image = panel.add_view::<ImageView>();
                    image.set_image(solid_image(case.fill, &format!("border_blend_{index}_{half}")));
                    image.weak_view()
                }
                _ => {
                    let mut blur = panel.add_view::<BlurView>();
                    blur.set_blur_radius(4).set_color(case.fill);
                    blur.weak_view()
                }
            };

            tile.set_border_color(case.border)
                .set_border_width(1)
                .set_corner_radius(10)
                .set_frame(frame);
        }
    }
}

impl Setup for BorderBlend {
    fn setup(self: Weak<Self>) {
        let panels = [self.clear_fill, self.light_card, self.dark_card];

        for (index, (panel, case)) in panels.into_iter().zip(CASES).enumerate() {
            panel.set_frame((0, 200 * index, 600, 190));
            build_row(panel, case, index);
        }
    }
}

/// `top` drawn over `bottom` with straight alpha, each channel 0 to 255.
fn over(top: Color, bottom: Color) -> [f32; 3] {
    let a = top.a;
    [
        (top.r * a + bottom.r * (1.0 - a)) * 255.0,
        (top.g * a + bottom.g * (1.0 - a)) * 255.0,
        (top.b * a + bottom.b * (1.0 - a)) * 255.0,
    ]
}

/// Every pixel of a tile and a 2 pixel ring around it has to lie between
/// the background, the border over the background and the fill over the
/// background, channel by channel. Returns the worst offender.
fn worst_pixel(
    shot: &hilen::window::Screenshot,
    case: &Case,
    frame: (f32, f32, f32, f32),
    panel_y: f32,
) -> Option<(u32, u32, U8Color, f32)> {
    const TOLERANCE: f32 = 3.0;

    let fill_over_bg = if case.fill.a > 0.0 {
        over(case.fill, case.bg)
    } else {
        over(case.bg, case.bg)
    };
    let mixes = [over(case.bg, case.bg), over(case.border, case.bg), fill_over_bg];

    let (x, y, w, h) = frame;
    let y = y + panel_y;
    let mut worst: Option<(u32, u32, U8Color, f32)> = None;

    let left: u32 = (x.floor() - 2.0).lossy_convert();
    let right: u32 = (x + w).ceil().lossy_convert();
    let top: u32 = (y.floor() - 2.0).lossy_convert();
    let bottom: u32 = (y + h).ceil().lossy_convert();

    for py in top..=bottom + 2 {
        for px in left..=right + 2 {
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

impl ViewTest for BorderBlend {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(320);

        checkpoint("each tile edge is a mix of border, fill and background, no dark or bright rim")?;

        let shot = capture_screenshot()?;
        let mut report = String::new();

        for (index, case) in CASES.iter().enumerate() {
            let index: f32 = index.lossy_convert();
            let panel_y = 200.0 * index;

            for (column, name) in COLUMNS.iter().enumerate() {
                for (half, frame) in tile_frames(column).into_iter().enumerate() {
                    let grid = if half == 0 { "on grid" } else { "half pixel" };

                    if let Some((x, y, got, miss)) = worst_pixel(&shot, case, frame, panel_y) {
                        writeln!(
                            report,
                            "{}, {name}, {grid}: pixel {x} {y} is {}, {miss:.0} out of the mixable range",
                            case.name,
                            got.as_hex(),
                        )?;
                    }
                }
            }
        }

        if !report.is_empty() {
            bail!("border pixels outside the colors they can be mixed from:\n{report}");
        }

        check_colors(PROBES)?;

        Ok(())
    }
}
