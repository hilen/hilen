use anyhow::Result;
use hilen::{
    gm::Shape,
    level::{Banner, LevelCreation, LevelSetup, LevelTest, SpriteTemplates, level},
    refs::Weak,
    ui::ImageFilter,
    ui_test::{check_colors, set_record_probe_count},
    window::image::ToImage,
};

/// Two 8 pixel tiles drawn 200 pixels wide with the nearest filter, every
/// texel a hard 25 pixel square. The linear default would blend each
/// texel into its neighbors.
#[level]
#[derive(Default)]
struct PixelArt {}

impl LevelSetup for PixelArt {
    fn setup(&mut self) {
        for (x, name) in [(-11, "game/tile_stone.png"), (11, "game/tile_grass.png")] {
            let mut image = name.to_image();
            image.set_filter(ImageFilter::Nearest);
            self.make_sprite::<Banner>(Shape::Rect((20, 20).into()), (x, 0))
                .set_image(image);
        }
    }
}

impl LevelTest for PixelArt {
    fn perform_test(_level: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        check_colors(NEAREST)
    }
}

const NEAREST: &str = r"
   4    4 - #597c95
 300    4 - #597c95
 592    4 - #597c95
 152   48 - #597c95
 444   52 - #597c95
 248   96 - #597c95
 352  100 - #597c95
   4  124 - #597c95
 592  124 - #597c95
 128  200 - #a3a3b0
 156  200 - #a3a3b0
 180  200 - #a3a3b0
 204  200 - #a3a3b0
 288  200 - #4b4b55
 332  200 - #4caf50
 372  200 - #4caf50
 420  200 - #4caf50
 452  200 - #4caf50
 472  200 - #4caf50
 504  200 - #4caf50
  92  204 - #a3a3b0
 244  204 - #a3a3b0
 352  204 - #4caf50
 400  204 - #4caf50
 264  212 - #a3a3b0
 436  212 - #4caf50
 312  216 - #4caf50
 380  216 - #4caf50
 112  224 - #a3a3b0
 140  224 - #a3a3b0
 168  224 - #a3a3b0
 196  224 - #a3a3b0
 220  224 - #a3a3b0
 288  224 - #4b4b55
 364  224 - #4caf50
 484  224 - #4caf50
 336  228 - #4caf50
 396  228 - #4caf50
 420  228 - #4caf50
 460  228 - #4caf50
 440  232 - #4caf50
 508  232 - #4caf50
 268  236 - #4b4b55
 352  236 - #4caf50
  92  240 - #a3a3b0
 312  240 - #4caf50
 452  244 - #4caf50
 380  248 - #4caf50
 212  252 - #5d5d69
 336  252 - #2e7d32
 344  252 - #2e7d32
 356  252 - #2e7d32
 412  252 - #2e7d32
 432  252 - #2e7d32
 484  252 - #2e7d32
 288  256 - #4b4b55
 424  256 - #2e7d32
 468  256 - #2e7d32
 348  260 - #2e7d32
 460  260 - #2e7d32
 476  260 - #2e7d32
 340  264 - #2e7d32
 356  264 - #2e7d32
 420  264 - #2e7d32
 508  264 - #8b5a2b
   4  268 - #597c95
 112  268 - #a3a3b0
 192  268 - #5d5d69
 472  268 - #2e7d32
 336  272 - #2e7d32
 344  272 - #2e7d32
 356  272 - #2e7d32
 412  272 - #2e7d32
 432  272 - #2e7d32
 460  272 - #2e7d32
 484  272 - #2e7d32
 168  276 - #a3a3b0
 268  276 - #4b4b55
 312  276 - #8b5a2b
 384  276 - #8b5a2b
 112  292 - #a3a3b0
 188  292 - #a3a3b0
  92  296 - #a3a3b0
 328  296 - #8b5a2b
 424  296 - #8b5a2b
 508  296 - #8b5a2b
 216  300 - #a3a3b0
 300  300 - #597c95
 364  300 - #6d4320
 372  304 - #6d4320
 384  304 - #6d4320
 448  304 - #8b5a2b
 360  308 - #6d4320
 268  312 - #4b4b55
 368  312 - #6d4320
 376  312 - #6d4320
 480  312 - #8b5a2b
 356  316 - #8b5a2b
 384  316 - #6d4320
 112  320 - #a3a3b0
 324  320 - #8b5a2b
 416  320 - #8b5a2b
  92  324 - #a3a3b0
 236  324 - #a3a3b0
 288  324 - #4b4b55
 364  324 - #6d4320
 376  324 - #6d4320
 508  324 - #8b5a2b
 164  328 - #5d5d69
  96  344 - #a3a3b0
 312  344 - #8b5a2b
 488  344 - #8b5a2b
 144  348 - #5d5d69
 280  348 - #4b4b55
 336  352 - #6d4320
 344  352 - #6d4320
 356  352 - #6d4320
 436  352 - #6d4320
 456  352 - #6d4320
 336  360 - #6d4320
 344  360 - #6d4320
 392  360 - #8b5a2b
 444  360 - #6d4320
 592  360 - #597c95
   4  364 - #597c95
 352  364 - #6d4320
 448  368 - #6d4320
 284  372 - #4b4b55
 336  372 - #6d4320
 356  372 - #6d4320
 436  372 - #6d4320
 456  372 - #6d4320
 492  372 - #8b5a2b
 108  376 - #4b4b55
 156  376 - #4b4b55
 196  376 - #4b4b55
 252  376 - #4b4b55
 312  376 - #8b5a2b
 132  384 - #4b4b55
  92  396 - #4b4b55
 168  396 - #4b4b55
 220  396 - #4b4b55
 264  396 - #4b4b55
 288  396 - #4b4b55
 328  396 - #8b5a2b
 372  396 - #8b5a2b
 404  396 - #8b5a2b
 444  396 - #8b5a2b
 476  396 - #8b5a2b
 508  396 - #8b5a2b
   4  472 - #597c95
 592  476 - #597c95
  92  488 - #597c95
 252  500 - #597c95
 356  500 - #597c95
 452  540 - #597c95
 156  552 - #597c95
   4  592 - #597c95
 312  592 - #597c95
 592  592 - #597c95
";
