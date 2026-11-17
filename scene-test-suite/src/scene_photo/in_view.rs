use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    scene::{SceneSetup, SceneTest, SceneTestView, SceneView, scene},
    ui::{Color, Label, ViewFrame, ViewSubviews},
};

use super::{Blocks, build_rig, caption, shoot_and_show};

/// The same rig and photos as `PhotoOfScene`, with the scene in a
/// `SceneView` on the left of the canvas. A photo is far larger than the
/// view and has its own camera, the view shows on as before.
#[scene]
#[derive(Default)]
struct PhotoOfSceneInView {
    root:   Weak<SceneTestView>,
    status: Weak<Label>,
}

impl SceneSetup for PhotoOfSceneInView {
    fn setup(&mut self) {
        build_rig(self);
    }
}

impl SceneTest for PhotoOfSceneInView {
    fn scene_view(view: Weak<SceneTestView>) -> Option<Weak<SceneView>> {
        let mut scene_view = view.add_view::<SceneView>();
        scene_view.set_frame((10.0, 60.0, 300.0, 400.0));
        scene_view.set_background(Color::hex("#3d4b57"));
        Some(scene_view)
    }

    fn overlay(mut scene: Weak<Self>, view: Weak<SceneTestView>) {
        scene.root = view;
        scene.status = caption(view, "", (10.0, 10.0, 580.0, 30.0));
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        let (root, status) = from_main(move || (scene.root, scene.status));
        shoot_and_show(root, status, (330.0, 160.0, 256.0, 192.0), &BLOCKS, move || {
            from_main(move || scene.camera)
        })
    }
}

const BLOCKS: Blocks = Blocks {
    rig:   RIG,
    shown: SHOWN,
};

const RIG: &str = r"
         588   16 - #1c2530
         132   24 - #767b82
         376   24 - #595f67
         144   28 - #737980
         152   28 - #b8bbbe
         200   28 - #1c2530
         248   28 - #414952
         284   28 - #272f3a
         288   28 - #f0f0f1
         332   28 - #fefefe
         440   28 - #8a8f95
         452   28 - #898d93
         176  232 - #b63f34
          76  248 - #6c9cca
          80  248 - #5d94c4
          76  252 - #6093c1
          96  260 - #2d6c9a
          52  264 - #255377
         228  284 - #846b1b
         264  288 - #c5a022
         592  304 - #597c95
         308  328 - #bec2c5
          12  456 - #3d4b57
         188  456 - #3d4b57
         592  592 - #597c95
";

const SHOWN: &str = r"
          12   16 - #1c2530
         144   20 - #1c2530
         244   24 - #dcdedf
         300   24 - #ffffff
         412   24 - #868b91
         152   28 - #585e66
         300   28 - #e5e6e7
         316   28 - #1c2530
         324   28 - #3a424c
         432   28 - #b6b9bc
         332  160 - #ffffff
         168  232 - #b63f34
         452  232 - #49343a
         400  240 - #327aad
         508  244 - #d9bb50
          76  248 - #6c9cca
          76  252 - #6093c1
         256  252 - #e3b924
         228  256 - #846b1b
         440  260 - #b6a1a2
         500  260 - #846b1b
         404  268 - #325773
          92  272 - #26567a
         264  288 - #c5a022
         572  336 - #b1b6b9
          12  456 - #3d4b57
         592  592 - #597c95
";
