use anyhow::{Context, Result, anyhow, ensure};
use futures_channel::oneshot;
use log::{debug, warn};
use wgpu::{BufferAsyncError, COPY_BYTES_PER_ROW_ALIGNMENT, MapMode};

use crate::{
    deps::refs::Weak,
    gm::{
        LossyConvert,
        color::{Color, U8Color},
        flat::Size,
    },
    scene::{Camera, Scene, SceneManager},
    scene_drawer::{Fill, SceneDrawer},
    time::Instant,
    window::{
        Window,
        image::{Image, TextureRawData},
    },
};

/// The pixels of a scene photo, see `SceneManager::photo`.
#[derive(Clone, Debug, Default)]
pub struct ScenePhoto {
    pub size: Size<u32>,
    /// 4 bytes a pixel, red, green, blue and alpha, row by row from the
    /// top. The alpha is straight, the color is not multiplied by it.
    pub rgba: Vec<u8>,
}

impl ScenePhoto {
    /// The color of the pixel at `x` and `y`, none outside the photo.
    pub fn pixel(&self, x: u32, y: u32) -> Option<U8Color> {
        if x >= self.size.width || y >= self.size.height {
            return None;
        }
        let at = usize::try_from(y * self.size.width + x).ok()? * 4;
        let [red, green, blue, alpha] = self.rgba.get(at..at + 4)?.try_into().ok()?;
        Some(U8Color::rgba(red, green, blue, alpha))
    }
}

/// The distance of every pixel of a scene photo, see
/// `SceneManager::depth_photo`.
#[derive(Clone, Debug, Default)]
pub struct SceneDepth {
    pub size:   Size<u32>,
    /// The distance in meters from the camera to the surface under each
    /// pixel, along the line of sight, so a plane facing the camera has 1
    /// distance all over. Row by row from the top, infinity where nothing
    /// is drawn.
    pub meters: Vec<f32>,
}

impl SceneDepth {
    /// The distance at `x` and `y`, none outside the photo.
    pub fn at(&self, x: u32, y: u32) -> Option<f32> {
        if x >= self.size.width || y >= self.size.height {
            return None;
        }
        self.meters.get(usize::try_from(y * self.size.width + x).ok()?).copied()
    }

    fn from_photo(photo: ScenePhoto) -> Self {
        let meters = photo
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[red, green, blue, _]| meters(red, green, blue))
            .collect();
        Self {
            size: photo.size,
            meters,
        }
    }
}

/// The log scale the mesh shader writes a distance on, `depth_color` in
/// `mesh.wgsl`: log2 of 1 mm and of 10 km, and the steps between them.
/// The 24 bits all set are left for no surface.
const DEPTH_LOG_NEAR: f32 = -9.965_784;
const DEPTH_LOG_FAR: f32 = 13.287_712;
const DEPTH_STEPS: u32 = 16_777_214;

/// The distance 3 bytes of a depth photo stand for.
fn meters(red: u8, green: u8, blue: u8) -> f32 {
    let steps = u32::from(red) << 16 | u32::from(green) << 8 | u32::from(blue);
    if steps > DEPTH_STEPS {
        return f32::INFINITY;
    }
    let (steps, all): (f32, f32) = (steps.lossy_convert(), DEPTH_STEPS.lossy_convert());
    let part = steps / all;
    (DEPTH_LOG_NEAR + part * (DEPTH_LOG_FAR - DEPTH_LOG_NEAR)).exp2()
}

type Mapped = oneshot::Receiver<Result<(), BufferAsyncError>>;

/// Draws the running scene through `camera` and reads it back, see
/// `SceneManager::photo`. The draw and the copy are sent at once. A native
/// GPU is waited for here, so the future holds only the bytes, a browser
/// is waited for in the future.
pub(crate) fn take(
    camera: Camera,
    size: Size<u32>,
    background: Color,
) -> impl Future<Output = Result<ScenePhoto>> {
    take_filled(camera, size, Fill::Light(background))
}

/// Draws the distance of every pixel through `camera` and reads it back,
/// see `SceneManager::depth_photo`.
pub(crate) fn take_depth(camera: Camera, size: Size<u32>) -> impl Future<Output = Result<SceneDepth>> {
    let photo = take_filled(camera, size, Fill::Depth);
    async move { photo.await.map(SceneDepth::from_photo) }
}

fn take_filled(camera: Camera, size: Size<u32>, fill: Fill) -> impl Future<Output = Result<ScenePhoto>> {
    let started = Instant::now();
    let sent = send(camera, size, fill);
    #[cfg(not_wasm)]
    let photo = sent.and_then(|(buffer, mapped)| wait(&buffer, mapped, size));
    async move {
        #[cfg(wasm)]
        let photo = wait(sent, size).await;
        match &photo {
            Ok(_) => debug!(
                "scene photo of {}x{} read back in {} ms",
                size.width,
                size.height,
                started.elapsed().as_millis()
            ),
            Err(err) => warn!("scene photo of {}x{} failed: {err}", size.width, size.height),
        }
        photo
    }
}

/// Draws `scene` once into an image of `size` pixels with every mip level,
/// see `SceneManager::mipped_picture`. A native GPU is waited for here and
/// the image made at once, on the thread that asked, a browser gives the
/// pixels back after a frame of the loop.
pub(crate) fn take_mipped(
    scene: &dyn Scene,
    name: String,
    size: Size<u32>,
    background: Color,
    cutout: Option<f32>,
) -> impl Future<Output = Result<Weak<Image>>> + use<> {
    let started = Instant::now();
    let sent = send_scene(scene, &scene.camera, size, Fill::Light(background));
    #[cfg(not_wasm)]
    let image = sent
        .and_then(|(buffer, mapped)| wait(&buffer, mapped, size))
        .map(|photo| mipped_image(photo, &name, cutout));
    async move {
        #[cfg(wasm)]
        let image = wait(sent, size).await.map(|photo| mipped_image(photo, &name, cutout));
        match &image {
            Ok(_) => debug!(
                "mipped picture {name} of {}x{} made in {} ms",
                size.width,
                size.height,
                started.elapsed().as_millis()
            ),
            Err(err) => warn!(
                "mipped picture {name} of {}x{} failed: {err}",
                size.width, size.height
            ),
        }
        image
    }
}

/// The image of a picture read back, with its levels built on the CPU.
fn mipped_image(photo: ScenePhoto, name: &str, cutout: Option<f32>) -> Weak<Image> {
    let raw = TextureRawData {
        data:     photo.rgba,
        size:     photo.size,
        channels: 4,
    };
    let levels = match cutout {
        Some(cutout) => raw.into_cutout_levels((cutout.clamp(0.0, 1.0) * 255.0).round().lossy_convert()),
        None => raw.into_levels(),
    };
    Image::from_levels(levels, name)
}

#[cfg(not_wasm)]
fn wait(buffer: &wgpu::Buffer, mut mapped: Mapped, size: Size<u32>) -> Result<ScenePhoto> {
    Window::device().poll(wgpu::PollType::wait_indefinitely())?;
    mapped
        .try_recv()
        .context("the photo was dropped before the GPU gave it back")?
        .context("the GPU did not give the photo back after the wait")?
        .map_err(|err| anyhow!("the photo could not be read from the GPU: {err}"))?;
    read(buffer, size)
}

#[cfg(wasm)]
async fn wait(sent: Result<(wgpu::Buffer, Mapped)>, size: Size<u32>) -> Result<ScenePhoto> {
    let (buffer, mapped) = sent?;
    mapped
        .await
        .context("the photo was dropped before the GPU gave it back")?
        .map_err(|err| anyhow!("the photo could not be read from the GPU: {err}"))?;
    read(&buffer, size)
}

/// Draws the running scene, copies it into a buffer and asks for the
/// buffer.
fn send(camera: Camera, size: Size<u32>, fill: Fill) -> Result<(wgpu::Buffer, Mapped)> {
    ensure!(!SceneManager::no_scene(), "no scene runs to take a photo of");
    send_scene(SceneManager::scene(), &camera, size, fill)
}

/// Draws `scene` through `camera`, copies it into a buffer and asks for
/// the buffer.
fn send_scene(
    scene: &dyn Scene,
    camera: &Camera,
    size: Size<u32>,
    fill: Fill,
) -> Result<(wgpu::Buffer, Mapped)> {
    ensure!(
        size.width > 0 && size.height > 0,
        "a scene photo needs a size, got {}x{}",
        size.width,
        size.height
    );
    let limits = Window::device().limits();
    let max_side = limits.max_texture_dimension_2d;
    ensure!(
        size.width <= max_side && size.height <= max_side,
        "a scene photo of {}x{} is over the {max_side} pixels a side this GPU draws",
        size.width,
        size.height
    );
    let bytes = row_bytes(size.width) * u64::from(size.height);
    ensure!(
        bytes <= limits.max_buffer_size,
        "a scene photo of {}x{} needs {bytes} bytes, over the {} of a buffer on this GPU",
        size.width,
        size.height,
        limits.max_buffer_size
    );

    let buffer = SceneDrawer::photo(scene, camera, size, fill);

    let (sender, mapped) = oneshot::channel();
    buffer.slice(..).map_async(MapMode::Read, move |result| {
        if sender.send(result).is_err() {
            debug!("a scene photo was mapped after its future was dropped");
        }
    });

    // A browser gives the buffer back by itself, after a frame of the loop.
    #[cfg(wasm)]
    crate::window::request_frame();

    Ok((buffer, mapped))
}

/// The bytes of a row in the buffer, padded the way a copy needs.
fn row_bytes(width: u32) -> u64 {
    (u64::from(width) * 4).next_multiple_of(u64::from(COPY_BYTES_PER_ROW_ALIGNMENT))
}

/// The pixels out of the mapped `buffer`, rows cut to their width.
fn read(buffer: &wgpu::Buffer, size: Size<u32>) -> Result<ScenePhoto> {
    let tight = usize::try_from(size.width)? * 4;
    let row = usize::try_from(row_bytes(size.width))?;
    let mut rgba = Vec::with_capacity(tight * usize::try_from(size.height)?);
    {
        let mapped = buffer.slice(..).get_mapped_range()?;
        for line in mapped.chunks_exact(row) {
            rgba.extend_from_slice(&line[..tight]);
        }
    }
    buffer.unmap();
    Ok(ScenePhoto { size, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `depth_color` in `mesh.wgsl` writes for `meters`.
    fn bytes_of(meters: f32) -> (u8, u8, u8) {
        let part = ((meters.log2() - DEPTH_LOG_NEAR) / (DEPTH_LOG_FAR - DEPTH_LOG_NEAR)).clamp(0.0, 1.0);
        let all: f32 = DEPTH_STEPS.lossy_convert();
        let steps: u32 = (part * all).round().lossy_convert();
        let [_, red, green, blue] = steps.to_be_bytes();
        (red, green, blue)
    }

    #[test]
    fn the_scale_runs_from_a_millimeter_to_ten_kilometers() {
        assert!((DEPTH_LOG_NEAR - 0.001_f32.log2()).abs() < 1e-5);
        assert!((DEPTH_LOG_FAR - 10_000_f32.log2()).abs() < 1e-5);
        assert!((meters(0, 0, 0) - 0.001).abs() < 1e-9);
    }

    #[test]
    fn a_distance_comes_back_to_a_millionth_of_itself() {
        for want in [0.01, 0.14, 0.27, 1.0, 5.72, 300.0] {
            let (red, green, blue) = bytes_of(want);
            let back = meters(red, green, blue);
            assert!((back - want).abs() / want < 2e-6, "{want} came back as {back}");
        }
    }

    #[test]
    fn white_is_no_surface() {
        assert_eq!(meters(255, 255, 255), f32::INFINITY);
        assert!(meters(255, 255, 254).is_finite());
    }
}
