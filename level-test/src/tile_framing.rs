use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::Shape,
    level::{
        Banner, LevelCreation, LevelManager, LevelSetup, LevelTest, SpriteTemplates, TileFrames, TileId,
        TileKind, TileMap, level,
    },
    refs::Weak,
    ui::{ImageFilter, Point},
    ui_test::{check_colors, checkpoint, set_record_probe_count},
    window::image::{Image, ToImage},
};

/// A block world framed like Terraria: a 16 pixel hero 2.6 units tall,
/// 6 pixel blocks at the same pixel size, drawn in pixel art mode. Grass
/// hills over dirt over stone, a lone block, a one block pillar and a
/// floating ledge, each block framed by the neighbors that join it. Then a pit
/// is dug from the top and a tunnel under the ground, and the blocks
/// around both get edges.
#[level]
#[derive(Default)]
struct TileFraming {
    map:  Weak<TileMap>,
    dirt: TileId,
}

/// One pixel of the art in level units.
const PIXEL: f32 = 2.6 / 16.0;
const BLOCK: f32 = 6.0 * PIXEL;
const WIDTH: usize = 40;
const HEIGHT: usize = 24;
/// The column under the hero, the camera looks at it.
const MIDDLE: usize = 20;

fn pixel_art(name: &str) -> Weak<Image> {
    let mut image = name.to_image();
    image.set_filter(ImageFilter::Nearest);
    image
}

fn framed(material: &str) -> TileKind {
    TileKind::solid(pixel_art(&format!("game/blocks/{material}_15_0.png"))).with_frames(TileFrames::new(
        |sides| {
            (0..3)
                .map(|variation| {
                    pixel_art(&format!(
                        "game/blocks/{material}_{}_{variation}.png",
                        sides.bits()
                    ))
                })
                .collect()
        },
    ))
}

/// The top row of ground in each column, hills with a flat run in the
/// middle for the hero.
fn ground_top(x: usize) -> usize {
    [
        12, 12, 13, 13, 13, 14, 14, 13, 12, 12, 11, 11, 11, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 13, 13,
        14, 15, 15, 14, 13, 13, 12, 12, 11, 11, 12, 12, 12, 12, 12,
    ][x]
}

impl LevelSetup for TileFraming {
    fn setup(&mut self) {
        let mut map = TileMap::new(WIDTH, HEIGHT);
        map.tile_size = BLOCK;
        map.origin = Point::new(-BLOCK * 20.0, -BLOCK * 12.0);
        map.outside_solid = false;
        let dirt = map.add_kind(framed("dirt"));
        let grass = map.add_kind(framed("grass"));
        let stone = map.add_kind(framed("stone"));
        map.join(dirt, grass);
        map.join(dirt, stone);
        map.join(grass, stone);

        for x in 0..WIDTH {
            let top = ground_top(x);
            for y in 0..=top {
                let kind = if y == top {
                    grass
                } else if y + 5 < top {
                    stone
                } else {
                    dirt
                };
                map.set(x, y, kind);
            }
        }
        // A lone block in the air, a one block pillar, and a floating ledge.
        map.set(12, 17, dirt);
        map.fill(15, 13, 1, 3, dirt);
        map.set(15, 16, grass);
        map.fill(26, 17, 4, 1, grass);

        self.dirt = dirt;
        self.map = self.add_tile_map(map);

        let hero = Point::new(0.0, BLOCK * 1.0 + 1.3);
        self.make_sprite::<Banner>(Shape::Rect((16.0 * PIXEL, 16.0 * PIXEL).into()), hero)
            .set_image(pixel_art("game/hero.png"));
    }
}

impl LevelTest for TileFraming {
    fn perform_test(level: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        // The runner sets its own zoom after setup, so the test sets it here.
        from_main(|| {
            LevelManager::set_points_per_unit(32.0);
            LevelManager::set_pixel_art(Some(PIXEL));
            *LevelManager::camera_pos() = Point::new(0.0, 0.0);
        });
        wait_for_next_frame();

        checkpoint(
            "grass hills over dirt over stone, dirt and stone meet with no edge, the lone block, the pillar and the floating ledge have edges all round with rounded corners",
        )?;
        check_colors(FRAMED)?;

        from_main(move || {
            let mut map = level.map;
            // A pit 3 blocks wide and 3 deep from the top, right of the hero.
            for x in 23..26 {
                let top = ground_top(x);
                for y in top - 2..=top {
                    map.set(x, y, TileId::EMPTY);
                }
            }
            // A tunnel 5 blocks long and 2 high under the hero, in the stone with dirt
            // above it.
            map.fill(MIDDLE - 3, 5, 5, 2, TileId::EMPTY);
            // The pit floor turns to dirt, grass would grow back later.
            map.set(24, ground_top(24) - 3, level.dirt);
        });
        wait_for_next_frame();

        checkpoint(
            "the pit and the tunnel have edges on every block around them, the tunnel has stone edges below and dirt edges above",
        )?;
        check_colors(DUG)
    }
}

const FRAMED: &str = r"
   4    4 - #597c95
 216    4 - #597c95
 388    4 - #597c95
 592    4 - #597c95
 488   24 - #597c95
 112   28 - #597c95
 296   88 - #597c95
 576  120 - #36e377
 492  128 - #2eb082
 516  128 - #36e377
 540  128 - #2eb082
  88  132 - #434a5f
  60  140 - #434a5f
 588  140 - #f4ac66
 156  152 - #36e377
 172  168 - #2eb082
 508  188 - #36e377
 520  188 - #36e377
 532  192 - #2eb082
   4  196 - #597c95
 284  196 - #3f2631
 328  200 - #3f2631
 488  200 - #2eb082
 176  204 - #434a5f
 308  208 - #c0cbdc
 272  212 - #eaa56c
 292  212 - #c0cbdc
 312  216 - #c0cbdc
 468  216 - #36e377
 544  216 - #36e377
 260  220 - #3f2631
 288  220 - #c0cbdc
 336  220 - #3f2631
 304  224 - #c0cbdc
 316  224 - #c0cbdc
 292  228 - #262b44
 312  228 - #262b44
 564  228 - #2eb082
 152  232 - #434a5f
 280  232 - #c0cbdc
 508  236 - #f4ac66
 316  240 - #c0cbdc
 276  248 - #c0cbdc
 288  248 - #c0cbdc
 304  248 - #3f2631
 316  248 - #c0cbdc
 324  248 - #c0cbdc
 400  248 - #36e377
 412  248 - #36e377
 436  248 - #36e377
 260  252 - #3f2631
 296  252 - #c0cbdc
 308  252 - #c0cbdc
 572  252 - #2eb082
 176  256 - #434a5f
 304  256 - #c0cbdc
 288  260 - #52607c
 292  260 - #52607c
 300  260 - #3f2631
 308  260 - #52607c
 288  264 - #52607c
 292  264 - #52607c
 300  264 - #3f2631
 308  264 - #52607c
 312  264 - #52607c
 336  264 - #3f2631
 268  268 - #3f2631
 296  268 - #3f2631
 300  268 - #3f2631
 304  268 - #3f2631
 240  272 - #434a5f
 464  272 - #9f5a52
 100  276 - #36e377
 112  276 - #36e377
 200  276 - #36e377
 368  276 - #36e377
 504  280 - #9f5a52
 148  284 - #2eb082
 336  284 - #2eb082
  96  288 - #2eb082
 180  288 - #2eb082
 388  288 - #2eb082
 552  296 - #cb815e
   4  300 - #434a5f
  28  308 - #2eb082
  68  308 - #36e377
 292  308 - #9f5a52
 592  312 - #9f5a52
  52  316 - #2eb082
  84  316 - #2eb082
 452  316 - #f4ac66
 380  320 - #cb815e
 340  324 - #9f5a52
 132  328 - #9f5a52
 416  328 - #f4ac66
 496  328 - #9f5a52
 244  336 - #f4ac66
 540  336 - #cb815e
 312  352 - #9f5a52
 444  352 - #cb815e
  64  356 - #cb815e
 192  356 - #f4ac66
 368  364 - #cb815e
 112  368 - #9f5a52
  16  372 - #cb815e
 412  372 - #cb815e
 472  372 - #9f5a52
 592  376 - #cb815e
 160  384 - #9f5a52
 544  384 - #cb815e
 280  388 - #f4ac66
 320  396 - #9f5a52
 220  404 - #cb815e
  60  408 - #9f5a52
 360  408 - #9f5a52
 120  416 - #cb815e
   4  420 - #cb815e
 392  428 - #88848f
 432  428 - #88848f
 588  428 - #88848f
 168  432 - #cb815e
 496  432 - #4f4b59
 300  448 - #9f5a52
  48  456 - #f4ac66
  92  456 - #6c6874
 240  456 - #88848f
 348  456 - #88848f
 396  464 - #6c6874
 464  464 - #4f4b59
 552  464 - #a5a1ab
 196  468 - #a5a1ab
   4  472 - #cb815e
 520  472 - #88848f
 144  476 - #88848f
 284  492 - #a5a1ab
 588  492 - #4f4b59
 328  500 - #6c6874
 432  500 - #88848f
 492  500 - #88848f
 132  520 - #a5a1ab
  72  524 - #a5a1ab
 192  524 - #4f4b59
 552  524 - #4f4b59
   8  528 - #88848f
 260  528 - #6c6874
 388  528 - #6c6874
 432  548 - #4f4b59
 104  556 - #88848f
 160  556 - #a5a1ab
 344  556 - #a5a1ab
  12  576 - #4f4b59
 492  576 - #88848f
 584  576 - #4f4b59
 216  584 - #6c6874
  72  588 - #88848f
 288  588 - #a5a1ab
 396  588 - #88848f
 456  588 - #88848f
 136  592 - #6c6874
 536  592 - #6c6874
";

const DUG: &str = r"
   4    4 - #597c95
 240    4 - #597c95
 408    4 - #597c95
 592    4 - #597c95
 124   32 - #597c95
 500   32 - #597c95
 324   84 - #597c95
 576  120 - #36e377
 512  128 - #2eb082
 536  128 - #36e377
 556  128 - #36e377
  64  136 - #434a5f
 488  136 - #2eb082
 588  140 - #f4ac66
 168  156 - #36e377
 508  188 - #36e377
 528  188 - #36e377
 284  196 - #3f2631
 328  200 - #3f2631
 488  200 - #2eb082
 308  208 - #c0cbdc
   4  212 - #597c95
 272  212 - #eaa56c
 292  212 - #c0cbdc
 156  216 - #cb815e
 312  216 - #c0cbdc
 540  216 - #36e377
 556  216 - #36e377
 260  220 - #3f2631
 288  220 - #c0cbdc
 336  220 - #3f2631
 304  224 - #c0cbdc
 316  224 - #c0cbdc
 292  228 - #262b44
 312  228 - #262b44
 280  232 - #c0cbdc
 316  240 - #c0cbdc
 480  240 - #434a5f
 260  248 - #3f2631
 276  248 - #c0cbdc
 288  248 - #c0cbdc
 304  248 - #3f2631
 324  248 - #c0cbdc
 572  248 - #2eb082
 296  252 - #c0cbdc
 308  252 - #c0cbdc
 592  252 - #2eb082
 304  256 - #c0cbdc
 524  256 - #cb815e
 292  260 - #52607c
 300  260 - #3f2631
 308  260 - #52607c
 264  264 - #3f2631
 288  264 - #52607c
 292  264 - #52607c
 300  264 - #3f2631
 308  264 - #52607c
 312  264 - #52607c
 296  268 - #3f2631
 300  268 - #3f2631
 304  268 - #3f2631
 312  268 - #c0cbdc
 332  268 - #3f2631
 112  276 - #36e377
 144  276 - #2eb082
 196  276 - #36e377
 208  276 - #36e377
 372  276 - #36e377
 560  276 - #9f5a52
 220  280 - #2eb082
 244  284 - #2eb082
 272  284 - #2eb082
 356  284 - #2eb082
 484  284 - #434a5f
  96  288 - #2eb082
 308  288 - #2eb082
 384  292 - #2eb082
 588  304 - #9f5a52
   8  308 - #36e377
  48  308 - #36e377
  68  308 - #36e377
 292  308 - #9f5a52
 532  312 - #9f5a52
  28  316 - #2eb082
  84  316 - #2eb082
 160  316 - #cb815e
 452  316 - #434a5f
 340  324 - #9f5a52
 204  328 - #9f5a52
 408  332 - #434a5f
 580  332 - #9f5a52
  60  336 - #f4ac66
 120  336 - #f4ac66
 492  340 - #f4ac66
 268  348 - #cb815e
 164  360 - #cb815e
 592  360 - #f4ac66
 316  364 - #9f5a52
 432  364 - #9f5a52
 536  364 - #6c6874
 228  372 - #cb815e
 376  372 - #cb815e
  48  376 - #9f5a52
 112  376 - #cb815e
   8  384 - #9f5a52
 488  388 - #88848f
  84  396 - #9f5a52
 196  404 - #cb815e
 392  408 - #f4ac66
 524  408 - #a5a1ab
 132  412 - #9f5a52
 240  412 - #cb815e
 280  412 - #9f5a52
 432  412 - #9f5a52
 572  412 - #cb815e
 336  416 - #cb815e
  40  428 - #cb815e
 552  440 - #88848f
 180  448 - #cb815e
 236  448 - #434a5f
 304  448 - #434a5f
 384  448 - #f4ac66
 520  448 - #88848f
  92  456 - #6c6874
 136  456 - #88848f
 464  456 - #6c6874
 588  468 - #a5a1ab
   4  472 - #cb815e
  56  476 - #f4ac66
 524  488 - #4f4b59
 200  496 - #88848f
 432  500 - #88848f
 484  508 - #88848f
 248  512 - #434a5f
  60  516 - #6c6874
 132  516 - #4f4b59
 364  516 - #6c6874
  12  520 - #4f4b59
 552  520 - #4f4b59
 308  524 - #88848f
 588  528 - #a5a1ab
  96  544 - #6c6874
 156  548 - #4f4b59
 196  548 - #88848f
 272  548 - #88848f
 400  552 - #4f4b59
 452  552 - #88848f
 520  552 - #a5a1ab
   8  568 - #88848f
 584  576 - #4f4b59
 160  588 - #a5a1ab
 248  588 - #88848f
 288  588 - #a5a1ab
 348  588 - #a5a1ab
 396  588 - #88848f
 492  588 - #a5a1ab
  40  592 - #88848f
 108  592 - #6c6874
 428  592 - #6c6874
 536  592 - #6c6874
";
