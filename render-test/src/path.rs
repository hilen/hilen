use hilen::{
    refs::main_lock::MainLock,
    render::data::{PathData, RectView, UIRectInstance},
    ui::{BLUE, CLEAR, CornerRadii, FillRule, RED, VectorPath},
    window::{RenderPass, Window},
};

use crate::pipelines::{PATH, UI_RECT};

static PATH_DATA: MainLock<Option<PathData>> = MainLock::new();

pub(crate) fn render_path(pass: &mut RenderPass) {
    let path = PATH_DATA.set({
        let (vertices, indices) = VectorPath::polygon([(0, 0), (80, 100), (20, 200), (200, 20), (20, 50)])
            .fill_mesh(FillRule::NonZero);

        let mut path = PathData::new(BLUE, &vertices, &indices);
        path.prepare((200, 200).into(), Window::render_size(), 1.0, 0.5);

        path.into()
    });

    let path = path.as_ref().unwrap();

    PATH.draw(pass, path);

    UI_RECT.get_mut().add(UIRectInstance::new(
        (450, 200, 200, 200).into(),
        RED,
        CLEAR,
        0.0,
        CornerRadii::default(),
        0.5,
        1.0,
    ));

    UI_RECT.get_mut().draw(
        pass,
        RectView {
            resolution: Window::inner_size(),
            _padding:   0,
        },
    );
}
