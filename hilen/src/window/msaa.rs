use std::sync::{
    OnceLock,
    atomic::{AtomicU32, Ordering},
};

use wgpu::Backend;

use crate::{
    pipelines::Pipelines,
    ui::{damage, ui_drawer},
    window::{AppHandler, Font, request_frame},
};

static BACKEND: OnceLock<Backend> = OnceLock::new();

/// The backend of the adapter the window got. Has to be told before the
/// first pipeline asks for the sample count.
pub(crate) fn set_backend(backend: Backend) {
    BACKEND.get_or_init(|| backend);
}

/// 0 until the first read or `set_msaa_sample_count` decides it.
static COUNT: AtomicU32 = AtomicU32::new(0);

/// Samples per pixel of the frame's render pass. 4 anti-aliases the
/// triangulated geometry the SDF pipelines cannot smooth, vector paths,
/// polygons and sprite cutouts. Every pipeline drawing into the pass
/// and the pass attachments must agree on this count.
///
/// An app picks it with `set_msaa_sample_count`. Until it does, the
/// count is the default of the device, or `HILEN_MSAA=1`, the A/B lever
/// for benchmarks. A page has no env vars, there it is `hilen_msaa=1` in
/// the page query.
pub fn msaa_sample_count() -> u32 {
    let count = COUNT.load(Ordering::Relaxed);
    if count != 0 {
        return count;
    }

    #[cfg(target_arch = "wasm32")]
    let value = crate::web::query_param("hilen_msaa");
    #[cfg(not(target_arch = "wasm32"))]
    let value = std::env::var("HILEN_MSAA").ok();

    let default = default_sample_count(
        BACKEND.get().copied(),
        cfg!(target_arch = "wasm32"),
        cfg!(target_os = "ios"),
    );

    let count = value.map_or(default, |value| {
        value.parse().unwrap_or_else(|_| panic!("Invalid HILEN_MSAA value: {value}"))
    });
    assert!(
        matches!(count, 1 | 2 | 4),
        "HILEN_MSAA must be 1, 2 or 4, got {count}"
    );
    COUNT.store(count, Ordering::Relaxed);
    count
}

/// Sets the samples per pixel, 1, 2 or 4, the smoothing of edges that a
/// game calls MSAA. 1 is about half the GPU work of 4 for every frame.
/// Rects, text and images look the same with 1, only vector paths,
/// polygons and sprite cutouts get rougher edges.
///
/// Before the window is up this is the count everything is first built
/// for. In a running app everything built for the old count is dropped
/// and made again for the new one, and the next frame is drawn whole.
/// Call it on the main thread and not from inside a draw, like a
/// `before_render`.
///
/// A pipeline of an app's own, or a `ShaderLab` view, is not made again
/// by this. It has to be built after the call.
pub fn set_msaa_sample_count(count: u32) {
    assert!(
        matches!(count, 1 | 2 | 4),
        "The sample count must be 1, 2 or 4, got {count}"
    );

    let old = COUNT.swap(count, Ordering::Relaxed);
    // Nothing is built before the pipelines are.
    if old == count || !Pipelines::ready() {
        return;
    }

    log::info!("sample count goes from {old} to {count}");

    Pipelines::rebuild();
    ui_drawer::drop_pipelines();
    #[cfg(feature = "level")]
    {
        crate::level_drawer::drop_pipelines();
        crate::game_drawer::drop_pipelines();
    }
    #[cfg(feature = "scene")]
    crate::scene_drawer::drop_pipelines();
    Font::rebuild_brushes();
    AppHandler::window().state.sample_count_changed();

    damage::whole_next("the sample count changed");
    request_frame();
}

/// A browser on WebGL gets no multisampling. That is the path of old and
/// weak devices, the Mali-G51 of an LG C1 TV drew one label at 10 frames a
/// second with 4 samples at 1080p and at 48 with 1.
///
/// An iPhone draws 1 sample. Its screen is 3x, so the rough edges of 1
/// sample are hard to see, and 1 sample is half the GPU work. It is also
/// the only count that keeps a partial frame there, see `docs/damage.md`.
fn default_sample_count(backend: Option<Backend>, browser: bool, iphone: bool) -> u32 {
    if iphone || (browser && backend == Some(Backend::Gl)) {
        1
    } else {
        4
    }
}

#[cfg(test)]
mod test {
    use wgpu::Backend;

    use super::default_sample_count;

    #[test]
    fn a_browser_on_webgl_draws_one_sample() {
        assert_eq!(default_sample_count(Some(Backend::Gl), true, false), 1);
    }

    #[test]
    fn a_browser_on_webgpu_keeps_four_samples() {
        assert_eq!(default_sample_count(Some(Backend::BrowserWebGpu), true, false), 4);
    }

    #[test]
    fn native_keeps_four_samples_on_every_backend() {
        assert_eq!(default_sample_count(Some(Backend::Gl), false, false), 4);
        assert_eq!(default_sample_count(Some(Backend::Metal), false, false), 4);
        assert_eq!(default_sample_count(None, false, false), 4);
    }

    #[test]
    fn an_iphone_draws_one_sample() {
        assert_eq!(default_sample_count(Some(Backend::Metal), false, true), 1);
    }
}
