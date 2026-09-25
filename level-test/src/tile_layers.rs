use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::Shape,
    level::{
        Banner, LevelCreation, LevelManager, LevelSetup, LevelTest, SpriteTemplates, TileKind, TileMap, level,
    },
    refs::Weak,
    ui::{ImageFilter, Point},
    ui_test::{check_colors, set_record_probe_count},
    window::image::ToImage,
};

const TILE: f32 = 5.0;

/// A tile world wider than the screen: a back wall layer, a ground layer
/// with grass on top and a stone step, and a cutout standing on it in
/// front of both. Then the camera moves by a tile and a half, so cells
/// come in on one edge and leave on the other.
#[level]
#[derive(Default)]
struct TileLayers {}

fn tile(name: &str) -> TileKind {
    let mut image = name.to_image();
    image.set_filter(ImageFilter::Nearest);
    TileKind::solid(image)
}

impl LevelSetup for TileLayers {
    fn setup(&mut self) {
        let origin = Point::new(-30.0, -30.0);

        let mut wall = TileMap::new(24, 12);
        wall.tile_size = TILE;
        wall.origin = origin;
        let back = wall.add_kind(TileKind {
            solid: false,
            ..tile("game/tile_wall.png")
        });
        wall.fill(0, 3, 24, 9, back);
        self.add_tile_map(wall);

        let mut ground = TileMap::new(24, 12);
        ground.tile_size = TILE;
        ground.origin = origin;
        let stone = ground.add_kind(tile("game/tile_stone.png"));
        let grass = ground.add_kind(tile("game/tile_grass.png"));
        ground.fill(0, 0, 24, 2, stone);
        ground.fill(0, 2, 24, 1, grass);
        ground.fill(7, 3, 2, 1, stone);
        ground.fill(8, 4, 1, 1, grass);
        self.add_tile_map(ground);

        self.make_sprite::<Banner>(Shape::Rect((6, 10).into()), (-5, -10))
            .set_image("game/frisk.png");
    }
}

impl LevelTest for TileLayers {
    fn perform_test(_level: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        check_colors(CAMERA_START)?;

        from_main(|| *LevelManager::camera_pos() = Point::new(TILE * 1.5, 0.0));
        wait_for_next_frame();

        check_colors(CAMERA_MOVED)
    }
}

const CAMERA_START: &str = r"
   4    4 - #3b2f2a
 160    4 - #3b2f2a
 436    4 - #3b2f2a
 592    4 - #3b2f2a
 300   76 - #3b2f2a
  84  108 - #3b2f2a
 428  184 - #3b2f2a
 204  208 - #3b2f2a
 592  216 - #3b2f2a
  40  228 - #2c231f
 320  256 - #2c231f
 400  352 - #4caf50
 440  364 - #2e7d32
 220  372 - #300e0b
 260  372 - #ffc90e
 268  372 - #ffc90e
 220  376 - #300e0b
 256  376 - #ffc90e
 264  376 - #ffc90e
 416  376 - #6d4320
 220  380 - #300e0b
 240  380 - #54270e
 244  380 - #ffc90e
 264  380 - #ffc90e
 272  380 - #ce9b0e
 220  384 - #300e0b
 244  384 - #ffc90e
 248  384 - #ffc90e
 256  384 - #ffc90e
 264  384 - #3d120e
 260  388 - #ffc90e
 268  388 - #ffc90e
 276  388 - #9b6b0e
 256  392 - #ffc90e
 264  392 - #ffc90e
 272  392 - #b28d0a
 252  396 - #efba0e
 400  396 - #8b5a2b
 260  400 - #673a0e
 264  400 - #673a0e
 268  400 - #673a0e
 352  400 - #a3a3b0
 448  400 - #4b4b55
 248  404 - #bb890e
 252  416 - #67a4e0
 256  416 - #67a4e0
 260  416 - #67a4e0
 252  420 - #401e1f
 260  420 - #e607f8
 372  420 - #a3a3b0
 420  420 - #a3a3b0
 252  428 - #efba0e
 256  428 - #ffc90e
 260  428 - #3d120e
 244  432 - #67a4e0
 264  432 - #67a4e0
 248  436 - #67a4e0
 256  436 - #659bd4
 260  436 - #67a4e0
 396  436 - #4b4b55
 444  436 - #4b4b55
 272  444 - #2e0f0c
 368  448 - #4b4b55
   4  452 - #4caf50
  72  452 - #4caf50
 152  452 - #4caf50
 208  452 - #4caf50
 328  452 - #4caf50
 512  452 - #4caf50
 564  452 - #4caf50
 592  452 - #4caf50
 252  460 - #4caf50
  40  464 - #2e7d32
  92  464 - #2e7d32
 188  464 - #2e7d32
 292  464 - #2e7d32
 312  464 - #2e7d32
 408  464 - #2e7d32
 476  464 - #2e7d32
 540  464 - #2e7d32
 116  468 - #8b5a2b
 228  468 - #2e7d32
 360  468 - #2e7d32
 336  476 - #8b5a2b
 568  476 - #6d4320
 444  480 - #8b5a2b
  12  484 - #8b5a2b
 388  484 - #8b5a2b
  84  488 - #6d4320
 132  488 - #6d4320
 160  488 - #6d4320
 268  488 - #8b5a2b
 312  488 - #6d4320
 592  488 - #8b5a2b
 108  492 - #6d4320
 236  492 - #6d4320
 360  492 - #6d4320
 512  492 - #6d4320
 544  492 - #8b5a2b
  56  496 - #8b5a2b
 204  496 - #8b5a2b
 412  500 - #a3a3b0
 480  500 - #a3a3b0
 292  512 - #7a7a86
 376  512 - #5d5d69
 444  512 - #4b4b55
   4  516 - #a3a3b0
  32  516 - #7a7a86
 132  516 - #7a7a86
 232  516 - #7a7a86
 332  516 - #7a7a86
 532  516 - #7a7a86
 580  516 - #5d5d69
 420  520 - #a3a3b0
  84  528 - #a3a3b0
 184  528 - #a3a3b0
 384  528 - #a3a3b0
 268  532 - #5d5d69
 364  532 - #5d5d69
 468  532 - #5d5d69
 516  532 - #5d5d69
  40  536 - #7a7a86
 116  536 - #5d5d69
 316  536 - #5d5d69
 404  536 - #a3a3b0
 152  540 - #a3a3b0
 496  540 - #4b4b55
  12  544 - #4b4b55
 552  544 - #4b4b55
 224  548 - #4b4b55
 440  548 - #4b4b55
 252  556 - #a3a3b0
  48  564 - #4b4b55
 104  564 - #a3a3b0
 132  564 - #7a7a86
 176  564 - #5d5d69
 284  564 - #7a7a86
 332  564 - #7a7a86
 384  564 - #7a7a86
 432  564 - #7a7a86
 484  564 - #7a7a86
 580  564 - #5d5d69
  76  568 - #5d5d69
  20  572 - #a3a3b0
 420  572 - #a3a3b0
 232  576 - #a3a3b0
 452  576 - #a3a3b0
 532  576 - #a3a3b0
 484  580 - #a3a3b0
 256  588 - #7a7a86
 352  588 - #a3a3b0
  52  592 - #a3a3b0
 108  592 - #7a7a86
 156  592 - #7a7a86
 192  592 - #7a7a86
 312  592 - #7a7a86
 392  592 - #7a7a86
 424  592 - #7a7a86
 560  592 - #7a7a86
 592  592 - #7a7a86
";

const CAMERA_MOVED: &str = r"
   4    4 - #3b2f2a
 160    4 - #3b2f2a
 592    4 - #3b2f2a
 300   76 - #3b2f2a
 456   76 - #3b2f2a
 176  196 - #2c231f
 364  208 - #3b2f2a
   4  212 - #3b2f2a
 512  220 - #2c231f
 592  328 - #3b2f2a
 328  352 - #4caf50
 148  364 - #251714
 204  364 - #300e0b
 368  364 - #2e7d32
 148  368 - #39110d
 184  372 - #ffc90e
 192  372 - #ffc90e
 180  376 - #b5830e
 184  376 - #ffc90e
 188  376 - #ffc90e
 192  376 - #ffc90e
 340  376 - #6d4320
 172  380 - #efba0e
 180  380 - #b5830e
 188  380 - #ffc90e
 192  380 - #ffc90e
 196  380 - #ffc90e
 168  384 - #ffc90e
 176  384 - #bb890e
 180  384 - #b5830e
 192  384 - #3d120e
 196  384 - #ffc90e
 184  388 - #ffc90e
 192  388 - #ffc90e
 196  388 - #ffc90e
 200  388 - #ffc90e
 188  392 - #ffc90e
 196  392 - #b28d0a
 148  396 - #251714
 180  396 - #ffc90e
 188  396 - #ffc90e
 192  396 - #ffc90e
 200  396 - #210a08
 184  400 - #673a0e
 284  400 - #a3a3b0
 316  400 - #a3a3b0
 176  404 - #ffc90e
 180  404 - #87580e
 372  412 - #4b4b55
 176  416 - #67a4e0
 180  416 - #67a4e0
 184  416 - #67a4e0
 184  420 - #e607f8
 348  420 - #a3a3b0
 296  424 - #a3a3b0
 184  428 - #54270e
 324  428 - #4b4b55
 168  432 - #67a4e0
 188  432 - #67a4e0
 172  436 - #67a4e0
 180  436 - #659bd4
 172  444 - #38110d
 180  444 - #27100d
 176  448 - #29120f
 184  448 - #36100c
 276  448 - #4b4b55
   4  452 - #4caf50
  76  452 - #4caf50
 136  452 - #4caf50
 392  452 - #4caf50
 432  452 - #4caf50
 460  452 - #4caf50
 500  452 - #4caf50
 540  452 - #4caf50
 364  460 - #4caf50
  48  464 - #8b5a2b
 168  464 - #2e7d32
 204  464 - #2e7d32
 300  464 - #2e7d32
 564  464 - #2e7d32
 588  464 - #8b5a2b
 104  468 - #2e7d32
 232  468 - #2e7d32
 264  468 - #2e7d32
 332  468 - #2e7d32
 412  468 - #8b5a2b
 484  468 - #2e7d32
 440  476 - #6d4320
 136  488 - #6d4320
 384  488 - #6d4320
 472  488 - #8b5a2b
 508  488 - #6d4320
 556  488 - #6d4320
 588  488 - #8b5a2b
   8  492 - #6d4320
 164  492 - #8b5a2b
 208  492 - #6d4320
 236  492 - #6d4320
 408  492 - #6d4320
 588  492 - #8b5a2b
 592  492 - #8b5a2b
  44  496 - #8b5a2b
  80  496 - #8b5a2b
 108  496 - #8b5a2b
 268  496 - #8b5a2b
 312  496 - #8b5a2b
 352  496 - #8b5a2b
 440  508 - #7a7a86
   8  516 - #7a7a86
 180  516 - #a3a3b0
 204  516 - #5d5d69
 404  516 - #5d5d69
 324  520 - #4b4b55
 232  524 - #7a7a86
 296  524 - #a3a3b0
 372  524 - #4b4b55
 484  524 - #7a7a86
 548  524 - #a3a3b0
  60  528 - #a3a3b0
 128  528 - #a3a3b0
 156  528 - #a3a3b0
 456  528 - #a3a3b0
   8  532 - #7a7a86
  92  532 - #5d5d69
 516  532 - #7a7a86
 392  536 - #5d5d69
   8  540 - #7a7a86
 428  540 - #a3a3b0
 592  540 - #7a7a86
  36  544 - #4b4b55
 340  544 - #4b4b55
 268  552 - #a3a3b0
  72  556 - #4b4b55
 164  556 - #7a7a86
 480  556 - #a3a3b0
 540  556 - #7a7a86
 372  560 - #4b4b55
 108  564 - #7a7a86
 208  564 - #7a7a86
 308  564 - #7a7a86
 408  564 - #7a7a86
 452  564 - #5d5d69
 508  564 - #7a7a86
   4  568 - #5d5d69
  48  572 - #a3a3b0
 408  580 - #a3a3b0
 560  580 - #a3a3b0
   8  584 - #7a7a86
  92  584 - #5d5d69
 140  584 - #5d5d69
 240  584 - #5d5d69
 592  584 - #5d5d69
   8  592 - #7a7a86
 180  592 - #a3a3b0
 280  592 - #a3a3b0
 324  592 - #4b4b55
 356  592 - #7a7a86
 436  592 - #7a7a86
 484  592 - #7a7a86
 528  592 - #a3a3b0
";
