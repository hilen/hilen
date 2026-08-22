use crate::game::Size;

pub enum Shape {
    Ball(f32),
    Box(f32),
    Rect(Size),
}
