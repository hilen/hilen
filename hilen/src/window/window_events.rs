use std::path::PathBuf;

use winit::{
    event::{ElementState, KeyEvent, MouseButton, Touch},
    window::Theme,
};

use crate::{
    gm::flat::{Point, Size},
    window::RenderFrame,
};

pub trait WindowEvents {
    fn window_ready(&mut self) {}
    fn update(&mut self) {}
    fn render(&mut self, _frame: &mut RenderFrame) {}
    /// Whether this frame must render into a texture that can be
    /// sampled mid frame, for example to blur the drawn scene.
    fn needs_sampleable_frame(&self) -> bool {
        false
    }
    fn resize(&mut self, _inner_pos: Point, _outer_pos: Point, _inner_size: Size, _outer_size: Size) {}
    fn mouse_moved(&mut self, _position: Point) -> bool {
        false
    }
    fn mouse_event(&mut self, _state: ElementState, _button: MouseButton) -> bool {
        false
    }
    fn mouse_scroll(&mut self, _delta: Point) {}
    fn cursor_left(&mut self) {}
    fn touch_event(&mut self, _touch: Touch) -> bool {
        false
    }
    fn key_event(&mut self, _event: KeyEvent) {}
    fn dropped_file(&mut self, _path: PathBuf) {}
    fn theme_changed(&mut self, _theme: Theme) {}
}
