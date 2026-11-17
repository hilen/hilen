use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Camera, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

/// The fades of the 3 cubes, each with its place along x.
const FADES: [(f32, f32); 3] = [(1.0, -2.4), (0.5, 0.0), (0.0, 2.4)];

/// 3 cubes floating over a light floor under a sun that casts, whole, at a
/// half and at nothing. The half cube casts a dithered shadow as thin as
/// itself, the one at nothing casts none.
#[scene]
#[derive(Default)]
struct FadeShadows {}

impl SceneSetup for FadeShadows {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 6.0, 6.0),
            target: Vec3::new(0.0, 0.6, -0.8),
            ..Camera::default()
        };
        self.sun.direction = Vec3::new(-0.3, -1.0, -0.6);
        self.sun.shadows = true;
        // The default is 1024 on a phone and in the browser.
        self.sun.shadow_map_size = 2048;

        self.make_node::<Prop>(Shape3::Plane(40.0), Vec3::ZERO)
            .set_color(Color::hex("#c8ccd0"))
            .set_roughness(0.9);
        for (fade, x) in FADES {
            self.make_node::<Prop>(Shape3::cube(1.2), Vec3::new(x, 1.6, 0.0))
                .set_color(Color::hex("#3a7fd0"))
                .set_roughness(0.7)
                .set_fade(fade);
        }
    }
}

impl SceneTest for FadeShadows {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);
        wait_for_next_frame();
        check_colors(SHADOWS)
    }
}

/// A whole shadow, a dithered one and none.
const SHADOWS: &str = r"
       4    4 - #597c95
      56    4 - #597c95
     108    4 - #597c95
     184    4 - #597c95
     236    4 - #597c95
     292    4 - #597c95
     344    4 - #597c95
     416    4 - #597c95
     472    4 - #597c95
     524    4 - #597c95
     580    4 - #597c95
     148    8 - #597c95
     380   12 - #597c95
     444   20 - #597c95
     156   44 - #597c95
     592   56 - #c0c3c7
     288  172 - #bfc3c7
     540  184 - #bfc3c7
      96  224 - #417cc6
     156  224 - #417cc6
     400  224 - #bfc3c7
     136  228 - #417cc6
     176  232 - #325c93
     116  240 - #417cc6
     172  240 - #325c93
      96  248 - #417cc6
     168  248 - #325c93
     176  252 - #325c93
      76  256 - #417cc6
     144  256 - #417cc6
     180  260 - #325c93
     116  264 - #417cc6
     168  264 - #325c93
     160  276 - #325c93
     176  276 - #325c93
     168  280 - #325c93
     100  288 - #396bab
     136  288 - #396bab
     172  288 - #325c93
     184  288 - #325c93
      72  292 - #386bab
     164  296 - #325c93
     180  300 - #325c93
     120  308 - #386bab
     168  308 - #325c93
     144  312 - #386bab
     180  316 - #6e7072
     104  320 - #386bab
     172  320 - #325c93
      84  324 - #386bab
     544  324 - #bfc3c7
     176  328 - #6e7072
     228  328 - #9da0a3
     176  332 - #6e7072
     232  332 - #9c9fa3
     100  340 - #6e7072
     120  340 - #6e7072
     140  340 - #6e7072
     156  340 - #6e7072
     148  344 - #6e7072
     172  344 - #6e7072
     308  344 - #9fa2a5
     108  348 - #6e7072
     136  348 - #6e7072
     164  348 - #6e7072
     272  348 - #a1a4a7
     128  352 - #6e7072
     152  352 - #6e7072
      96  356 - #6e7072
     116  356 - #6e7072
     140  356 - #6e7072
     168  356 - #6e7072
     408  356 - #bfc3c7
     592  448 - #bfc3c7
       4  456 - #bfc3c7
     296  472 - #bfc3c7
     464  476 - #bfc3c7
     172  532 - #bfc3c7
       4  592 - #bfc3c7
     336  592 - #bfc3c7
     456  592 - #bfc3c7
     592  592 - #bfc3c7
";
