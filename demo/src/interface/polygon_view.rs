use hilen::{
    RenderPass, VertexBuffer,
    level::LevelManager,
    refs::Weak,
    ui::{
        Anchor::{Height, Top, Width, X},
        Button, Container, Point, PositionView, Setup, ViewCallbacks, ViewData, ViewFrame, ViewSubviews,
        WHITE, view,
    },
};

use crate::interface::HAS_BACK_BUTTON;

#[view]
pub struct PolygonView {
    points: VertexBuffer,
    views:  Vec<Weak<PositionView>>,
    #[init]
    add:    Button,
    print:  Button,
    center: Container,
}

impl Setup for PolygonView {
    fn setup(self: Weak<Self>) {
        LevelManager::stop_level();

        self.apply_style(HAS_BACK_BUTTON);

        self.add.set_text("Add").place().t(200).r(10).size(100, 50);
        self.add.on_tap(move || {
            self.add_point((0, 0).into());
        });

        self.print.set_text("Print");
        self.print.place().anchor(Top, self.add, 10).same([Width, Height, X], self.add);
        self.print.on_tap(move || {
            dbg!(&self.points);
        });

        self.center.set_color(WHITE).place().size(5, 5).center();

        // after(0.1, move || self.add_first_points());
    }
}

impl PolygonView {
    pub fn display_points(mut self: Weak<Self>, points: Vec<Point>) {
        let points = points.into_iter().map(|p| p * 50.0);

        for mut view in self.views.drain(..) {
            view.remove_from_superview();
        }

        self.points.clear();

        for point in points {
            self.add_point(point);
        }

        self.views.iter_mut().for_each(|v| v.update_label());
    }

    fn add_point(mut self: Weak<Self>, pos: Point) {
        let mut view = self.add_view::<PositionView>();
        view.set_position(pos);
        view.set_tag(self.points.vertices.len());
        view.additional_label = format!("{}:", self.points.vertices.len()).into();
        self.views.push(view);
        let pos = LevelManager::convert_touch(pos + self.frame().origin);
        self.points.vertices.push(pos);

        view.moved.val(self, move |new_pos| {
            self.points.vertices[view.tag()] = LevelManager::convert_touch(new_pos + self.frame().origin);
        });
    }

    fn _add_first_points(self: Weak<Self>) {
        self.add_point((200, 200).into());
        self.add_point((200, 500).into());
        self.add_point((500, 200).into());
    }
}

impl ViewCallbacks for PolygonView {
    fn before_render(&self, _pass: &mut RenderPass) {
        // let drawer = Window::drawer();
        //
        // drawer.polygon_test.clear();
        //
        // if self.points.is_empty() {
        //     return;
        // }
        //
        // drawer.polygon_test.add(&self.points, (0, 0).into(), GREEN,
        // 0.0);
        //
        // drawer.polygon_test.draw(
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
