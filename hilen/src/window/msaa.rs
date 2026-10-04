use std::sync::OnceLock;

use wgpu::Backend;

static BACKEND: OnceLock<Backend> = OnceLock::new();

/// The backend of the adapter the window got. Has to be told before the
/// first pipeline asks for the sample count.
pub(crate) fn set_backend(backend: Backend) {
    BACKEND.get_or_init(|| backend);
}

/// Samples per pixel of the frame's render pass. 4 anti-aliases the
/// triangulated geometry the SDF pipelines cannot smooth, vector paths,
/// polygons and sprite cutouts. Every pipeline drawing into the pass
/// and the pass attachments must agree on this count. `HILEN_MSAA=1`
/// switches multisampling off, the A/B lever for benchmarks. A page has
/// no env vars, there it is `hilen_msaa=1` in the page query.
pub fn msaa_sample_count() -> u32 {
    static COUNT: OnceLock<u32> = OnceLock::new();
    *COUNT.get_or_init(|| {
        #[cfg(target_arch = "wasm32")]
        let value = crate::web::query_param("hilen_msaa");
        #[cfg(not(target_arch = "wasm32"))]
        let value = std::env::var("HILEN_MSAA").ok();

        let default = default_sample_count(BACKEND.get().copied(), cfg!(target_arch = "wasm32"));

        let count = value.map_or(default, |value| {
            value.parse().unwrap_or_else(|_| panic!("Invalid HILEN_MSAA value: {value}"))
        });
        assert!(
            matches!(count, 1 | 2 | 4),
            "HILEN_MSAA must be 1, 2 or 4, got {count}"
        );
        count
    })
}

/// A browser on WebGL gets no multisampling. That is the path of old and
/// weak devices, the Mali-G51 of an LG C1 TV drew one label at 10 frames a
/// second with 4 samples at 1080p and at 48 with 1.
fn default_sample_count(backend: Option<Backend>, browser: bool) -> u32 {
    if browser && backend == Some(Backend::Gl) {
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
        assert_eq!(default_sample_count(Some(Backend::Gl), true), 1);
    }

    #[test]
    fn a_browser_on_webgpu_keeps_four_samples() {
        assert_eq!(default_sample_count(Some(Backend::BrowserWebGpu), true), 4);
    }

    #[test]
    fn native_keeps_four_samples_on_every_backend() {
        assert_eq!(default_sample_count(Some(Backend::Gl), false), 4);
        assert_eq!(default_sample_count(Some(Backend::Metal), false), 4);
        assert_eq!(default_sample_count(None, false), 4);
    }
}
