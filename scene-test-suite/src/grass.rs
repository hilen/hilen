use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, Grass, GrassSpot, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sky, Sun, Wind,
        scene, step_scene,
    },
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const EAST: f32 = 300.0;
/// Blade, ground and tint colors far apart, so a probe on a blade, on the
/// bare ground and on a tinted blade cannot be mixed up.
const GROUND: &str = "#6b4a2b";
const TINT: &str = "#ffc020";
/// Blades per patch of 8 by 8 m.
const PER_PATCH: u32 = 14 * 64;

/// Where grass grows: everywhere at height 0, but in a bare square ahead
/// of the camera at the start, and tinted yellow east of x = 0 there.
fn ground(x: f32, z: f32) -> Option<GrassSpot> {
    if (-2.0..2.0).contains(&x) && (-9.0..-5.0).contains(&z) {
        return None;
    }
    let tint = if x > 0.0 && x < EAST / 2.0 {
        Color::hex(TINT)
    } else {
        Color::hex("#ffffff")
    };
    Some(GrassSpot {
        height: 0.0,
        normal: Vec3::Y,
        tint,
    })
}

/// A field of wide blades over brown ground, out to 30 m and thinning
/// from 12. Every patch grows in 1 step, the scene only moves when the
/// test steps it, so the blades and the sway land the same way each run.
#[scene]
#[derive(Default)]
struct GrassField {}

impl SceneSetup for GrassField {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 1.6, 0.0),
            target: Vec3::new(0.0, 0.4, -10.0),
            ..Camera::default()
        };
        self.sun = Sun {
            direction: Vec3::new(-0.4, -1.0, -0.3),
            ..Sun::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#5d8fd6"),
            Color::hex("#b9d3f0"),
            Color::hex("#6b5a48"),
        ));
        self.make_node::<Prop>(Shape3::Plane(1000.0), Vec3::ZERO)
            .set_color(Color::hex(GROUND))
            .set_roughness(1.0);

        let mut grass = Grass::new(Grass::blade("grass_test_blade", 0.6, 0.16), ground);
        grass.density = 14.0;
        grass.patch_size = 8.0;
        grass.distance = 30.0;
        grass.thin_from = 12.0;
        grass.spots_per_frame = u32::MAX;
        // A sway that fits a blade of 0.6 m, the default is for a taller
        // meadow and lays it flat.
        grass.sway_distance = 0.6;
        self.grass.push(grass);
        self.wind = Wind {
            direction: Vec3::X,
            strength:  0.0,
        };
    }
}

impl SceneTest for GrassField {
    fn stepped() -> bool {
        true
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let counts = move || from_main(move || (scene.grass[0].patch_count(), scene.grass[0].blade_count()));

        step_scene(1);
        let (patches, blades) = counts();
        // 30 m in every direction from the camera, 8 m patches.
        ensure!((40..=64).contains(&patches), "{patches} patches around the start");
        // Full patches, less the bare square.
        let full = u32::try_from(patches)? * PER_PATCH;
        ensure!(
            (full - PER_PATCH..full).contains(&blades),
            "{blades} blades in {patches} patches"
        );
        wait_for_next_frame();
        check_colors(STILL)?;

        from_main(move || {
            let mut scene = scene;
            scene.wind.strength = 0.5;
        });
        step_scene(20);
        wait_for_next_frame();
        check_colors(WINDY)?;

        from_main(move || {
            let mut scene = scene;
            scene.camera.position = Vec3::new(EAST, 1.6, 0.0);
            scene.camera.target = Vec3::new(EAST, 0.4, -10.0);
            scene.wind.strength = 0.0;
        });
        step_scene(1);
        let (moved, _) = counts();
        ensure!(
            moved == patches,
            "{moved} patches 300 m east, {patches} at the start, the old ones go and new ones grow"
        );
        wait_for_next_frame();
        check_colors(MOVED)
    }
}

/// Still grass: the bare square ahead, yellow blades right of the middle,
/// the field thinning into bare ground far away.
const STILL: &str = r"
     144    4 - #a2c0e7
     264    4 - #a1bfe7
     472    4 - #a3c0e7
     592    4 - #a5c2e8
     368   60 - #a9c5e9
       4  104 - #b0caea
     236  120 - #b0caea
     460  144 - #b2cceb
     584  160 - #b4cdeb
     152  240 - #6d5745
     364  240 - #6d5745
     540  240 - #6d5745
     200  264 - #559549
      56  268 - #508d45
     268  268 - #549348
     460  272 - #394b2c
      36  276 - #385b36
      48  288 - #549248
     152  296 - #365735
     400  300 - #3f532d
     284  308 - #569649
     112  312 - #6c5644
     508  316 - #3f532f
     576  316 - #6c5644
       4  336 - #3c6339
      64  360 - #518f46
     372  360 - #6c5543
     460  360 - #33432b
     284  368 - #549247
     568  368 - #63573e
     220  372 - #569649
     172  376 - #569849
     588  376 - #3b4e2b
     104  392 - #355735
     544  396 - #32402d
     272  400 - #335234
     468  400 - #37492a
     340  416 - #6b5543
     444  428 - #6b5543
      96  432 - #6b5543
       4  436 - #508d45
     212  440 - #4c8342
     536  440 - #6b5543
     388  448 - #3d502e
     132  452 - #6b5543
     468  472 - #6b5542
     556  476 - #3d522c
     340  480 - #455d2d
      84  488 - #6b5542
     196  504 - #315032
     272  516 - #365b35
       4  520 - #569749
     136  520 - #497e40
     404  524 - #6b5442
     464  536 - #6b5442
     512  540 - #41572b
     592  548 - #4a652a
     140  584 - #549447
     380  584 - #6a5442
     240  588 - #3b6238
     448  588 - #6a5442
      44  592 - #6b5442
     316  592 - #37482c
     548  592 - #6b5442
";

/// The wind leans every blade to the right.
const WINDY: &str = r"
       4    4 - #a5c2e8
     364    4 - #a1bfe7
     592   32 - #a9c4e8
     184   92 - #adc8ea
     468  100 - #aec9ea
     308  116 - #afcaea
      52  140 - #b2cceb
      72  240 - #6d5745
     248  240 - #6d5745
     320  240 - #6d5745
     476  240 - #6d5745
     388  264 - #4c652f
     584  264 - #516b2f
       4  272 - #46773f
     164  280 - #569649
     140  288 - #569649
     188  296 - #569649
     108  304 - #569749
     292  308 - #569649
     148  320 - #559448
     556  320 - #49602f
     408  324 - #303e2c
     492  328 - #6c5644
      32  348 - #477b3f
     460  360 - #33432b
     124  368 - #559448
     348  368 - #506c2c
     248  372 - #6b5543
     588  376 - #303e2b
     208  388 - #569849
     292  392 - #4d8643
      28  404 - #314e32
     140  408 - #314d32
     240  408 - #579849
     164  420 - #325033
     528  420 - #3b4e2d
     368  424 - #6b5543
      36  428 - #539347
     452  428 - #6b5543
     268  440 - #4f8a45
      60  460 - #569749
     332  464 - #394b2d
     592  468 - #6b5543
     400  472 - #4d672d
     520  472 - #6b5542
     212  492 - #375e36
     468  492 - #6b5442
      40  508 - #569749
     200  516 - #315032
      96  520 - #365836
     144  520 - #395e37
     288  520 - #6b5442
      44  528 - #569749
     524  528 - #3b4f2c
     588  528 - #3b4e2d
     364  532 - #3f542d
      24  536 - #569749
     436  552 - #6b5442
     240  580 - #6a5442
      44  592 - #6b5442
     116  592 - #335234
     316  592 - #41723c
     496  592 - #33442b
     588  592 - #303f2b
";

/// The field grown again 300 m east, no bare square, no tint.
const MOVED: &str = r"
       4    4 - #a5c2e8
     132    4 - #a3c0e7
     508    4 - #a3c1e7
     256   32 - #a5c2e8
     392  112 - #afc9ea
     116  128 - #b0cbeb
     592  128 - #b1cceb
       4  156 - #b3ceeb
     100  240 - #6d5745
     276  240 - #6d5745
     496  240 - #6d5745
     424  244 - #6d5745
      28  268 - #6c5644
     192  268 - #3a6038
     356  268 - #508b45
     500  280 - #549148
     448  284 - #569449
     160  288 - #549348
     576  300 - #549148
     228  308 - #355635
     124  312 - #559448
     308  312 - #538f46
      80  316 - #559548
     432  316 - #549148
     480  320 - #335234
       4  332 - #47793f
     188  336 - #314d32
     344  336 - #539147
     280  348 - #6c5543
     588  352 - #549148
     524  364 - #57984a
     216  368 - #569749
     116  380 - #6b5543
      44  384 - #6b5543
     268  396 - #549348
     480  400 - #6b5543
     320  408 - #396137
     172  412 - #6b5543
     396  412 - #579849
     592  420 - #6b5543
       4  424 - #43733d
     260  432 - #2f4c32
     448  440 - #2f4b31
      96  444 - #3d683a
     228  444 - #2f4c32
     528  444 - #6b5543
     196  456 - #304c31
      32  464 - #325033
     148  484 - #498140
     376  488 - #6b5442
      52  492 - #6b5542
     288  508 - #395e37
     336  508 - #559648
     576  512 - #345835
     216  516 - #345734
      12  532 - #6b5442
     236  548 - #304e31
     468  548 - #6b5442
     584  552 - #2e4b31
     244  584 - #304e31
       8  592 - #355535
     120  592 - #43723d
     364  592 - #2e4a31
     568  592 - #45783e
";
