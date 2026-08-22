use hilen::{
    RenderPass,
    refs::Weak,
    ui::{NumberView, Setup, ViewCallbacks, ViewData, view},
};

use crate::interface::HAS_BACK_BUTTON;

#[view]
pub struct RenderView {
    #[init]
    val: NumberView,
}

impl Setup for RenderView {
    fn setup(self: Weak<Self>) {
        self.val.set_step(0.1).place().size(50, 100).bl(0);

        self.apply_style(HAS_BACK_BUTTON);
    }
}

impl ViewCallbacks for RenderView {
    fn before_render(&self, _pass: &mut RenderPass) {
        // let drawer = Window::drawer();

        // drawer.sprite_box.add((2, 2).into(), (0, 0).into(), 0.0, RED,
        // 0.5); drawer.sprite_box.add((2, 2).into(), (40, 0).into(),
        // 0.0, GREEN, 0.5); drawer.sprite_box.add((2, 2).into(),
        // (40, 40).into(), 0.0, BLUE, 0.5); drawer.sprite_box.
        // add((2, 2).into(), (0, 40).into(), 0.0, TURQUOISE,
        // 0.5); drawer.sprite_box.draw(pass, 1.0, 0.0, (0, 0).into(),
        // UIManager::resolution());

        // drawer.polygon.clear();
        //
        // drawer.polygon.draw(
        //     pass,
        //     SpriteView {
        //         camera_pos:      Point::default(),
        //         resolution:      UIManager::resolution(),
        //         camera_rotation: 0.0,
        //         scale:           1.0,
        //     },
        // );
    }
}
