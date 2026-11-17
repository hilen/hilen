use anyhow::Result;
use hilen::{
    gm::Shape,
    level::{Banner, Flip, LevelCreation, LevelSetup, LevelTest, SpriteTemplates, level},
    refs::Weak,
    ui_test::{check_colors, set_record_probe_count},
};

/// The same cutout four times, left to right: as it is, mirrored left
/// to right, mirrored top to bottom, and both.
#[level]
#[derive(Default)]
struct SpriteFlip {}

impl LevelSetup for SpriteFlip {
    fn setup(&mut self) {
        for (x, flip_x, flip_y) in [
            (-21, false, false),
            (-7, true, false),
            (7, false, true),
            (21, true, true),
        ] {
            let mut sprite = self.make_sprite::<Banner>(Shape::Rect((6, 10).into()), (x, 0));
            sprite.set_image("game/frisk.png");
            sprite.flip = Flip { x: flip_x, y: flip_y };
        }
    }
}

impl LevelTest for SpriteFlip {
    fn perform_test(_level: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        check_colors(FLIPPED)
    }
}

const FLIPPED: &str = r"
   4    4 - #597c95
 284    4 - #597c95
 592    4 - #597c95
 440   88 - #597c95
 144   96 - #597c95
 296  136 - #597c95
   4  140 - #597c95
 592  140 - #597c95
  76  252 - #3d120e
 100  252 - #3d120e
 232  252 - #3d120e
 368  252 - #3d120e
 380  252 - #3d120e
 516  252 - #3d120e
 216  256 - #3d120e
 356  256 - #3d120e
 504  256 - #352c32
 368  260 - #4d495e
 380  260 - #4d495e
 392  260 - #342b31
 488  260 - #342b31
 500  260 - #4d495e
 512  260 - #4d495e
 256  264 - #3d4653
 376  264 - #4d495e
 380  264 - #67a4e0
 504  264 - #4d495e
 364  268 - #67a4e0
 384  268 - #67a4e0
 496  268 - #67a4e0
 516  268 - #67a4e0
  60  272 - #300e0b
 108  272 - #ffc90e
 212  272 - #ffc90e
 372  272 - #efba0e
 388  272 - #3d120e
 508  272 - #81520e
  96  276 - #ffc90e
 116  276 - #3d4653
 224  276 - #b5830e
 496  276 - #67a4e0
  84  280 - #ffc90e
 108  280 - #ffc90e
 216  280 - #ffc90e
 224  280 - #b5830e
 236  280 - #ffc90e
 368  280 - #587196
 380  280 - #e607f8
 500  280 - #e607f8
 100  284 - #ffc90e
 208  284 - #ffc90e
 224  284 - #b5830e
 228  284 - #bb890e
 368  284 - #587196
 372  284 - #67a4e0
 376  284 - #67a4e0
 508  284 - #67a4e0
 116  288 - #9b6b0e
 216  288 - #ffc90e
 364  288 - #3b110e
 516  288 - #39110d
  96  292 - #ffc90e
 104  292 - #ffc90e
 112  292 - #b28d0a
 208  292 - #b28d0a
 364  292 - #465a6b
 372  292 - #77490e
 508  292 - #77490e
  64  296 - #3d120e
 228  296 - #81520e
 256  296 - #3d4653
 368  296 - #bb890e
 384  296 - #383942
 388  296 - #383942
 492  296 - #383942
 496  296 - #383942
 504  296 - #87580e
 100  300 - #673a0e
 104  300 - #673a0e
 108  300 - #673a0e
 212  300 - #673a0e
 216  300 - #673a0e
 220  300 - #673a0e
 380  300 - #d5a10e
 388  300 - #d5a10e
 492  300 - #d5a10e
 500  300 - #d5a10e
  88  304 - #bb890e
 224  304 - #87580e
 392  304 - #000000
 488  304 - #000000
 508  304 - #81520e
 536  304 - #3d4653
  84  308 - #465a6b
 376  308 - #ffc90e
 236  312 - #3d120e
 340  312 - #3d120e
 368  312 - #ffc90e
 388  312 - #ffc90e
 480  312 - #3d120e
 492  312 - #ffc90e
 504  312 - #b5830e
 508  312 - #bb890e
  88  316 - #587196
  96  316 - #67a4e0
 224  316 - #67a4e0
 376  316 - #ffc90e
 396  316 - #3d4653
 504  316 - #b5830e
  88  320 - #587196
 100  320 - #e607f8
 220  320 - #e607f8
 228  320 - #587196
 364  320 - #ffc90e
 384  320 - #ffc90e
 392  320 - #ce9b0e
 488  320 - #ffc90e
 496  320 - #ffc90e
 504  320 - #b5830e
 516  320 - #ffc90e
  96  324 - #673a0e
 212  324 - #3d120e
 224  324 - #673a0e
 376  324 - #ffc90e
 396  324 - #3d4653
 504  324 - #b5830e
  96  328 - #ffc90e
 228  328 - #81520e
 388  328 - #ffc90e
 492  328 - #ffc90e
  84  332 - #67a4e0
 104  332 - #67a4e0
 216  332 - #67a4e0
 224  332 - #77490e
 236  332 - #67a4e0
 344  332 - #3d120e
  88  336 - #67a4e0
 100  336 - #67a4e0
 232  336 - #67a4e0
 536  336 - #3d4653
 108  340 - #597c95
 372  340 - #3d120e
 512  340 - #3d120e
  76  344 - #3d120e
 224  344 - #352c32
 236  344 - #3d120e
 392  344 - #301413
 528  344 - #3d120e
  88  348 - #3d120e
 100  348 - #3d120e
 212  348 - #3d120e
 356  348 - #3d120e
 496  348 - #3d120e
   4  456 - #597c95
 296  468 - #597c95
 464  492 - #597c95
 180  532 - #597c95
 592  572 - #597c95
   4  592 - #597c95
 356  592 - #597c95
";
