use anyhow::{Context, Result, anyhow, ensure};
use futures_channel::oneshot;
use log::{debug, warn};
use wgpu::{BufferAsyncError, COPY_BYTES_PER_ROW_ALIGNMENT, MapMode};

use crate::{
    gm::{
        color::{Color, U8Color},
        flat::Size,
    },
    scene::{Camera, SceneManager},
    scene_drawer::SceneDrawer,
    time::Instant,
    window::Window,
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
    let started = Instant::now();
    #[cfg(not_wasm)]
    let photo = send(camera, size, background).and_then(|(buffer, mut mapped)| {
        Window::device().poll(wgpu::PollType::wait_indefinitely())?;
        mapped
            .try_recv()
            .context("the photo was dropped before the GPU gave it back")?
            .context("the GPU did not give the photo back after the wait")?
            .map_err(|err| anyhow!("the photo could not be read from the GPU: {err}"))?;
        read(&buffer, size)
    });
    #[cfg(wasm)]
    let sent = send(camera, size, background);
    async move {
        #[cfg(wasm)]
        let photo = async {
            let (buffer, mapped) = sent?;
            mapped
                .await
                .context("the photo was dropped before the GPU gave it back")?
                .map_err(|err| anyhow!("the photo could not be read from the GPU: {err}"))?;
            read(&buffer, size)
        }
        .await;
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

/// Draws the photo, copies it into a buffer and asks for the buffer.
fn send(camera: Camera, size: Size<u32>, background: Color) -> Result<(wgpu::Buffer, Mapped)> {
    ensure!(!SceneManager::no_scene(), "no scene runs to take a photo of");
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

    let buffer = SceneDrawer::photo(SceneManager::scene(), &camera, size, background);

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
