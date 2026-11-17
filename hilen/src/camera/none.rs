//! A system with no camera support in the engine, tvOS.

use crate::camera::{
    CameraDevice, CameraError,
    backend::{Backend, Done, Session},
    frame::FrameSlot,
};

pub(super) struct System;

impl Backend for System {
    fn request_access(&self, done: Done<()>) {
        done(Err(CameraError::NoCamera.into()));
    }

    fn devices(&self, done: Done<Vec<CameraDevice>>) {
        done(Err(CameraError::NoCamera.into()));
    }

    fn open(&self, device: &CameraDevice, frames: FrameSlot, done: Done<Box<dyn Session>>) {
        drop(frames);
        done(Err(CameraError::NotFound(device.name.clone()).into()));
    }
}
