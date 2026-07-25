use std::{path::PathBuf, sync::OnceLock};

use anyhow::Result;
use refs::main_lock::MainLock;
use vents::Event;

static ROOT_PATH: OnceLock<PathBuf> = OnceLock::new();

static PROGRESS: MainLock<Event<f32>> = MainLock::new();

pub struct Assets;

impl Assets {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn init(root_path: impl Into<PathBuf>) {
        use refs::manage::DataManager;

        hreads::assert_main_thread();

        let root_path = root_path.into();

        ROOT_PATH.set(root_path.clone()).expect("Double setting of root path");

        let paths = crate::assets_paths::AssetsPaths::new(root_path);

        crate::window::image::Image::set_root_path(&paths.images);
        crate::audio::Sound::set_root_path(&paths.sounds);
        crate::window::Font::set_root_path(&paths.fonts);
    }

    pub fn path() -> PathBuf {
        ROOT_PATH
            .get()
            .expect("Assets root path is not set yet")
            .as_path()
            .join("assets")
    }

    /// Fired on the main thread with the loaded fraction while a group
    /// downloads. Native loads from disk on demand, so it only ever
    /// fires the final 1.0.
    pub fn load_progress() -> &'static Event<f32> {
        &PROGRESS
    }

    /// Resolves when the boot asset group is in memory and every sync
    /// `get` on it works. Immediate on native, where files load from
    /// disk on demand.
    pub async fn await_boot() {
        #[cfg(target_arch = "wasm32")]
        web_assets::await_boot().await;
        #[cfg(not(target_arch = "wasm32"))]
        std::future::ready(()).await;
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Assets {
    pub fn boot_done() -> bool {
        true
    }

    /// Native reads assets from disk on demand, there is nothing to
    /// download. Kept so app code has one call site for every platform.
    pub async fn load_group(_group: &str) -> Result<()> {
        hreads::on_main(|| PROGRESS.trigger(1.0));
        std::future::ready(Ok(())).await
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use web_assets::start_boot_preload;

#[cfg(target_arch = "wasm32")]
impl Assets {
    pub fn boot_done() -> bool {
        web_assets::boot_done()
    }

    /// Downloads every asset of the group into the managed stores, so
    /// sync `get` works for all of them afterwards. Files already in
    /// memory are skipped by the in flight dedup in `download`.
    pub async fn load_group(group: &str) -> Result<()> {
        Self::await_boot().await;
        web_assets::load_group(group).await
    }

    /// Downloads every group in the manifest. The UI suite needs it,
    /// a browser serves sync `get` only from memory, while a native
    /// run reads any file from disk on demand.
    pub(crate) async fn load_all_groups() -> Result<()> {
        Self::await_boot().await;
        web_assets::load_all_groups().await
    }
}

/// A browser has no filesystem, so the sync managed `get` calls can
/// only be served from memory. The build writes `assets.json` next to
/// the asset files, and this module downloads a whole group of them
/// into the managed stores. The `boot` group loads before tests and
/// app loading screens let anything through, other groups load when
/// the app asks. Every file url carries its content hash, so a host
/// can serve the folder as immutable and a changed file busts exactly
/// one cache entry.
#[cfg(target_arch = "wasm32")]
mod web_assets {
    use std::sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    };

    use anyhow::{Result, anyhow};
    use hreads::on_main;
    use log::error;
    use refs::manage::{DataManager, fetch_bytes};
    use serde::Deserialize;

    use crate::{
        audio::Sound,
        window::{Font, image::Image},
    };

    static BOOT_DONE: AtomicBool = AtomicBool::new(false);
    static BOOT_EVENT: event_listener::Event = event_listener::Event::new();
    static MANIFEST: OnceLock<Manifest> = OnceLock::new();

    #[derive(Deserialize)]
    struct Manifest {
        files: Vec<ManifestFile>,
    }

    #[derive(Clone, Deserialize)]
    struct ManifestFile {
        kind:  String,
        name:  String,
        group: String,
        hash:  String,
    }

    pub(crate) fn boot_done() -> bool {
        BOOT_DONE.load(Ordering::Acquire)
    }

    /// The listener registers before the flag re-check, so a trigger
    /// firing between the two is never missed.
    pub(crate) async fn await_boot() {
        if boot_done() {
            return;
        }
        let listener = BOOT_EVENT.listen();
        if boot_done() {
            return;
        }
        listener.await;
    }

    /// Fetches the manifest and loads the boot group. Called once from
    /// `window_ready`. Finishes even on error, a missing manifest must
    /// not wedge the app, the failed assets just fall back to defaults.
    pub(crate) fn start_boot_preload() {
        hreads::spawn(async {
            let boot_started = web_time::Instant::now();
            match fetch_manifest().await {
                Ok(manifest) => {
                    let manifest = MANIFEST.get_or_init(|| manifest);
                    if let Err(err) = load_entries(manifest, "boot").await {
                        error!("Boot asset preload failed: {err}");
                    }
                }
                Err(err) => error!("Asset manifest fetch failed: {err}"),
            }

            BOOT_DONE.store(true, Ordering::Release);
            BOOT_EVENT.notify(usize::MAX);
            on_main(|| super::PROGRESS.trigger(1.0));

            log::debug!("Boot assets ready in {} ms", boot_started.elapsed().as_millis());
        });
    }

    pub(crate) async fn load_group(group: &str) -> Result<()> {
        let manifest = MANIFEST.get().ok_or_else(|| anyhow!("No asset manifest"))?;
        load_entries(manifest, group).await
    }

    /// Boot is already in memory when this runs, so only the lazy
    /// groups actually download.
    pub(crate) async fn load_all_groups() -> Result<()> {
        let manifest = MANIFEST.get().ok_or_else(|| anyhow!("No asset manifest"))?;

        let mut groups: Vec<&str> = manifest.files.iter().map(|f| f.group.as_str()).collect();
        groups.sort_unstable();
        groups.dedup();

        for group in groups {
            if group == "boot" {
                continue;
            }
            load_entries(manifest, group).await?;
        }

        Ok(())
    }

    async fn fetch_manifest() -> Result<Manifest> {
        let data = fetch_bytes("/assets/assets.json").await?;
        Ok(serde_json::from_slice(&data)?)
    }

    async fn load_entries(manifest: &Manifest, group: &str) -> Result<()> {
        let entries: Vec<ManifestFile> =
            manifest.files.iter().filter(|f| f.group == group).cloned().collect();

        let total = entries.len();

        if total == 0 {
            return Err(anyhow!("Asset group '{group}' is empty or unknown"));
        }

        // Kick every download as its own task, then await them in
        // order. The in flight dedup in `download` joins the second
        // await to the running task, so the group downloads
        // concurrently.
        for entry in entries.clone() {
            hreads::spawn(async move {
                if let Err(err) = download_entry(&entry).await {
                    // The awaiting pass below reports it as the group error.
                    log::debug!("Asset download task failed: {err}");
                }
            });
        }

        for (done, entry) in entries.iter().enumerate() {
            download_entry(entry).await?;

            let progress = (done + 1) as f32 / total as f32;
            on_main(move || super::PROGRESS.trigger(progress));
        }

        Ok(())
    }

    async fn download_entry(entry: &ManifestFile) -> Result<()> {
        let url = format!("/assets/{}/{}?h={}", entry.kind, entry.name, entry.hash);

        match entry.kind.as_str() {
            "images" => {
                Image::download(&entry.name, &url).await?;
            }
            "fonts" => {
                Font::download(&entry.name, &url).await?;
            }
            "sounds" => {
                Sound::download(&entry.name, &url).await?;
            }
            kind => return Err(anyhow!("Unknown asset kind: {kind}")),
        }

        Ok(())
    }
}
