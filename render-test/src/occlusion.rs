use hilen::{
    RenderPass,
    render::{
        UIRectPipeline,
        data::{RectView, UIImageInstance, UIRectInstance},
    },
    ui::{BLUE, CLEAR, CornerRadii, GREEN, RED, UIImages},
    window::Window,
};

use crate::pipelines::{IMAGE_DRAWER, UI_RECT};

pub(crate) fn render_occlusion(pass: &mut RenderPass) {
    let rect = UI_RECT.get_mut();
    let image = IMAGE_DRAWER.get_mut();

    add_equal_z_stacks(rect);
    add_layered_z_stack(rect);

    rect.add(UIRectInstance::new(
        (200, 50, 100, 100).into(),
        GREEN,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.5,
        1.0,
    ));

    rect.draw(
        pass,
        RectView {
            resolution: Window::inner_size(),
            _padding:   0,
        },
    );

    image.add_with_image(
        UIImageInstance {
            position:     (225, 75).into(),
            size:         (50, 50).into(),
            border_color: CLEAR,
            border_width: 0.0,
            corner_radii: CornerRadii::default(),
            z_position:   0.4,
            flags:        0,
            scale:        1.0,
            uv_position:  (0, 0).into(),
            uv_size:      (1, 1).into(),
        },
        UIImages::rb(),
    );

    image.draw(
        pass,
        RectView {
            resolution: Window::inner_size(),
            _padding:   0,
        },
    );
}

fn add_equal_z_stacks(rect: &mut UIRectPipeline) {
    rect.add(UIRectInstance::new(
        (50, 50, 50, 50).into(),
        RED,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.5,
        1.0,
    ));

    rect.add(UIRectInstance::new(
        (75, 75, 50, 50).into(),
        GREEN,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.5,
        1.0,
    ));
    rect.add(UIRectInstance::new(
        (100, 100, 50, 50).into(),
        BLUE,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.5,
        1.0,
    ));

    rect.add(UIRectInstance::new(
        (100, 250, 50, 50).into(),
        BLUE,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.5,
        1.0,
    ));
    rect.add(UIRectInstance::new(
        (75, 225, 50, 50).into(),
        GREEN,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.5,
        1.0,
    ));
    rect.add(UIRectInstance::new(
        (50, 200, 50, 50).into(),
        RED,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.5,
        1.0,
    ));
}

fn add_layered_z_stack(rect: &mut UIRectPipeline) {
    rect.add(UIRectInstance::new(
        (50, 350, 50, 50).into(),
        RED,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.3,
        1.0,
    ));
    rect.add(UIRectInstance::new(
        (75, 375, 50, 50).into(),
        GREEN,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.2,
        1.0,
    ));
    rect.add(UIRectInstance::new(
        (100, 400, 50, 50).into(),
        BLUE,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.1,
        1.0,
    ));
}
