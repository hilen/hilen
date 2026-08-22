use std::{cell::RefCell, ops::Deref};

use bytemuck::Pod;
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device};

use crate::{
    render::{buffer_helper::BufferHelper, device_helper::DeviceHelper},
    window::{BufferUsages, Window},
};

#[derive(Debug)]
pub(crate) struct UniformBind<T> {
    data:   RefCell<T>,
    buffer: Buffer,
    bind:   BindGroup,
}

impl<T> UniformBind<T> {
    pub(crate) fn bind(&self) -> &BindGroup {
        &self.bind
    }
}

impl<T: Default + Pod> UniformBind<T> {
    fn new(device: &Device, layout: &BindGroupLayout) -> Self {
        let data = T::default();
        let buffer = device.buffer(&data, BufferUsages::UNIFORM | BufferUsages::COPY_DST);
        let bind = device.bind(&buffer, layout);
        Self {
            data: data.into(),
            buffer,
            bind,
        }
    }
}

impl<T: Pod + PartialEq> UniformBind<T> {
    pub(crate) fn update(&self, data: T) {
        if self.data.borrow().deref() == &data {
            return;
        }
        self.buffer.update(data);
        self.data.replace(data);
    }
}

impl<T: Default + Pod> From<BindGroupLayout> for UniformBind<T> {
    fn from(layout: BindGroupLayout) -> Self {
        Self::new(Window::device(), &layout)
    }
}
