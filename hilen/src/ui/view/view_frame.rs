use crate::{
    gm::{
        ToF32,
        flat::{Point, Rect, Size},
    },
    ui::{UIManager, View, ViewSubviews},
};

pub trait ViewFrame {
    fn z_position(&self) -> f32;
    fn set_z_position(&mut self, z: f32) -> &mut Self;
    fn bump_z_position(&self, z: f32) -> &Self;
    fn frame(&self) -> &Rect;
    fn absolute_frame(&self) -> &Rect;
    fn x(&self) -> f32;
    fn y(&self) -> f32;
    fn max_x(&self) -> f32;
    fn max_y(&self) -> f32;
    fn size(&self) -> Size;
    fn width(&self) -> f32;
    fn height(&self) -> f32;
    fn set_x(&mut self, x: impl ToF32) -> &mut Self;
    fn set_y(&mut self, y: impl ToF32) -> &mut Self;
    fn set_height(&mut self, height: impl ToF32) -> &mut Self;
    fn set_position(&self, origin: impl Into<Point>) -> &Self;
    fn set_center(&mut self, center: impl Into<Point>) -> &mut Self;
    fn set_frame(&self, rect: impl Into<Rect>) -> &Self;
    fn set_size(&self, width: impl ToF32, height: impl ToF32) -> &Self;
    fn edit_frame(&mut self, edit: impl FnOnce(&mut Rect)) -> &mut Self;
    fn trigger_events(&mut self);
}

impl<T: ?Sized + View> ViewFrame for T {
    fn z_position(&self) -> f32 {
        self.__base_view().z_position
    }

    fn set_z_position(&mut self, z: f32) -> &mut Self {
        assert!(
            self.superview().is_null(),
            "Z position must be set before adding to superview"
        );
        self.__base_view().z_position = z;
        self.__base_view().z_position_custom = true;
        self
    }

    fn bump_z_position(&self, z: f32) -> &Self {
        self.__base_view().z_position -= z;
        for sub in self.subviews() {
            sub.bump_z_position(z + UIManager::subview_z_offset());
        }
        self
    }

    fn frame(&self) -> &Rect {
        &self.__base_view().frame
    }

    fn absolute_frame(&self) -> &Rect {
        &self.__base_view().__absolute_frame
    }

    fn x(&self) -> f32 {
        self.frame().origin.x
    }

    fn y(&self) -> f32 {
        self.frame().origin.y
    }

    fn max_x(&self) -> f32 {
        self.frame().max_x()
    }

    fn max_y(&self) -> f32 {
        self.frame().max_y()
    }

    fn size(&self) -> Size {
        self.frame().size
    }

    fn width(&self) -> f32 {
        self.frame().size.width
    }

    fn height(&self) -> f32 {
        self.frame().size.height
    }

    fn set_x(&mut self, x: impl ToF32) -> &mut Self {
        let x = x.to_f32();
        let frame = &mut self.__base_view().frame;
        // Bit compare: any value change counts, epsilon would miss tiny moves.
        let pos_changed = frame.origin.x.to_bits() != x.to_bits();
        frame.origin.x = x;
        self.__base_view().trigger_pos_changed |= pos_changed;

        self
    }

    fn set_y(&mut self, y: impl ToF32) -> &mut Self {
        let y = y.to_f32();
        let frame = &mut self.__base_view().frame;
        let pos_changed = frame.origin.y.to_bits() != y.to_bits();
        frame.origin.y = y;
        self.__base_view().trigger_pos_changed |= pos_changed;

        self
    }

    fn set_height(&mut self, height: impl ToF32) -> &mut Self {
        let height = height.to_f32();
        let frame = &mut self.__base_view().frame;
        let size_changed = frame.size.height.to_bits() != height.to_bits();
        frame.size.height = height;
        self.__base_view().trigger_size_changed |= size_changed;

        self
    }

    fn set_position(&self, origin: impl Into<Point>) -> &Self {
        let origin = origin.into();
        let frame = &mut self.__base_view().frame;
        let pos_changed = frame.origin != origin;
        frame.origin = origin;
        self.__base_view().trigger_pos_changed |= pos_changed;

        self
    }

    fn set_center(&mut self, center: impl Into<Point>) -> &mut Self {
        let center = center.into();
        let frame = &mut self.__base_view().frame;
        let pos_changed = frame.center() != center;
        frame.set_center(center);
        self.__base_view().trigger_pos_changed |= pos_changed;

        self
    }

    fn set_frame(&self, rect: impl Into<Rect>) -> &Self {
        let rect = rect.into();
        let frame = &mut self.__base_view().frame;

        let pos_changed = rect.origin != frame.origin;
        let size_changed = rect.size != frame.size;

        *frame = rect;

        self.__base_view().trigger_pos_changed |= pos_changed;
        self.__base_view().trigger_size_changed |= size_changed;

        self
    }

    fn set_size(&self, width: impl ToF32, height: impl ToF32) -> &Self {
        let size = (width, height).into();
        let frame = &mut self.__base_view().frame;

        let changed = size != frame.size;

        frame.size = size;

        self.__base_view().trigger_size_changed |= changed;

        self
    }

    fn edit_frame(&mut self, edit: impl FnOnce(&mut Rect)) -> &mut Self {
        let frame = &mut self.__base_view().frame;
        let prev_frame = *frame;
        edit(frame);

        let pos_changed = prev_frame.origin != frame.origin;
        let size_changed = prev_frame.size != frame.size;

        self.__base_view().trigger_pos_changed |= pos_changed;
        self.__base_view().trigger_size_changed |= size_changed;

        self
    }

    fn trigger_events(&mut self) {
        let view = self.__base_view();

        if view.trigger_size_changed {
            view.size_changed.trigger(());
        }
        if view.trigger_pos_changed {
            view.position_changed.trigger(());
        }

        view.trigger_size_changed = false;
        view.trigger_pos_changed = false;
    }
}
