use std::sync::OnceLock;

use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
#[cfg(not_wasm)]
use chrono::Local;
#[cfg(not_wasm)]
use hreads::log_spawn;
use hreads::{from_main, on_main};
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
#[cfg(not_wasm)]
use log::info;
#[cfg(not_wasm)]
use mdns_sd::{ServiceDaemon, ServiceInfo};
use refs::manage::DataManager;
#[cfg(not_wasm)]
use tokio::net::TcpListener;

#[cfg(not_wasm)]
use crate::inspect::protocol::{SERVICE_TYPE, serve};
use crate::{
    audio::Sound,
    inspect::{
        edit_log,
        protocol::{AppCommand, EditEntry, InspectorCommand, TestFailureRepr, UIRequest, UIResponse},
        view_conversion::{ViewToInspect, weak_to_id},
    },
    ui::{
        Button, Input, Label, TextField, Touch, TouchEvent, UIManager, ViewData, ViewFrame, ViewSubviews,
        WeakView,
    },
    window::MouseButton,
};

pub struct InspectService;

static APP_STARTED: OnceLock<u64> = OnceLock::new();

impl InspectService {
    pub(crate) fn record_app_start() {
        APP_STARTED.get_or_init(current_unix_seconds);
    }

    #[cfg(not_wasm)]
    pub(crate) fn start_listening() {
        log_spawn(async {
            let listener = TcpListener::bind("0.0.0.0:0").await?;
            let port = listener.local_addr()?.port();

            let app_id = UIManager::app_instance_id();

            let mdns = ServiceDaemon::new()?;
            let service = ServiceInfo::new(
                SERVICE_TYPE,
                app_id,
                &format!("{app_id}.local."),
                "",
                port,
                &[("app_id", app_id)][..],
            )?
            .enable_addr_auto();
            mdns.register(service)?;

            info!("Inspect server on port: {port}");

            serve(listener, Self::process_command).await
        });
    }

    pub(crate) fn process_command(command: InspectorCommand) -> AppCommand {
        match command {
            InspectorCommand::PlaySound => {
                on_main(|| {
                    Sound::get("retro.wav").play();
                });

                AppCommand::Ok
            }
            InspectorCommand::Screenshot => match Self::screenshot() {
                Ok(screenshot) => screenshot,
                Err(err) => AppCommand::Error(format!("Screenshot failed: {err}")),
            },
            InspectorCommand::ListEdits => AppCommand::Edits(edit_log::all()),
            InspectorCommand::RunTests => Self::run_tests(),
            // Compiled in, so it reports when this running code was built, not
            // when the bundle around it was linked. See `test-engine/build.rs`.
            InspectorCommand::GetBuildTime => {
                AppCommand::BuildTime(env!("TE_BUILD_TIME").parse().expect("TE_BUILD_TIME is not a number"))
            }
            InspectorCommand::GetStartTime => {
                AppCommand::StartTime(*APP_STARTED.get().expect("App start time was not recorded"))
            }
            InspectorCommand::UI(ui) => Self::process_ui_command(ui),
        }
    }

    // Runs off the main thread, on a tokio task natively and on the inspect
    // worker on wasm, which is what the tests need since they drive the main
    // thread through `from_main`.
    fn run_tests() -> AppCommand {
        #[cfg(wasm)]
        Self::preload_assets();

        let report = crate::ui_test::run_all_tests();

        AppCommand::TestResults {
            total:    report.total,
            failures: report
                .failures
                .into_iter()
                .map(|f| TestFailureRepr {
                    name:   f.name,
                    detail: f.detail,
                })
                .collect(),
        }
    }

    /// A browser serves sync asset `get` only from memory, so every group
    /// downloads before the suite starts, same as the test autorun. Blocks the
    /// calling worker while the download runs on the main thread.
    #[cfg(wasm)]
    fn preload_assets() {
        let (send, recv) = std::sync::mpsc::channel();

        on_main(move || {
            hreads::spawn(async move {
                if let Err(err) = crate::assets::Assets::load_all_groups().await {
                    log::error!("Asset preload for tests failed: {err}");
                }
                send.send(()).expect("Asset preload signal receiver is gone");
            });
        });

        recv.recv().expect("Asset preload signal sender is gone");
    }

    fn screenshot() -> Result<AppCommand> {
        let shot = crate::AppRunner::take_screenshot()?;

        let mut bytes = Vec::with_capacity(shot.data.len() * 4);
        for color in &shot.data {
            bytes.extend_from_slice(&[color.r, color.g, color.b, 255]);
        }

        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(
            &bytes,
            shot.size.width,
            shot.size.height,
            ExtendedColorType::Rgba8,
        )?;

        Ok(AppCommand::Screenshot {
            width:      shot.size.width,
            height:     shot.size.height,
            png_base64: STANDARD.encode(&png),
        })
    }

    fn process_ui_command(command: UIRequest) -> AppCommand {
        match command {
            UIRequest::SetScale(scale) => {
                from_main(move || {
                    UIManager::set_scale(scale);
                });

                // send_ui dispatches a fresh from_main, so the snapshot runs
                // one frame later, after layout applied the new scale.
                Self::send_ui()
            }
            UIRequest::GetUI => Self::send_ui(),
            UIRequest::EditRule {
                view_id,
                rule_index,
                offset,
                enabled,
            } => Self::apply(move || {
                let view = find_view(&view_id)?;

                let rules_count = view.place().get_rules().len();
                if rule_index >= rules_count {
                    return Err(format!(
                        "Rule index {rule_index} is out of range, view has {rules_count} rules"
                    ));
                }

                let mut rule = view.place().edit_rule(rule_index);
                let old = format!("offset: {}, enabled: {}", rule.offset(), rule.enabled);
                rule.set_offset(offset);
                rule.enabled = enabled;

                Ok(entry(
                    view,
                    format!("rule {rule_index}"),
                    old,
                    format!("offset: {offset}, enabled: {enabled}"),
                ))
            }),
            UIRequest::SetText { view_id, text } => Self::apply(move || {
                let view = find_view(&view_id)?;
                let old = set_text(view, &text)?;
                Ok(entry(view, "text", old, text))
            }),
            UIRequest::SetColor { view_id, color } => Self::apply(move || {
                let view = find_view(&view_id)?;
                let old = *view.color();
                view.set_color(color);
                Ok(entry(view, "color", format!("{old}"), format!("{color}")))
            }),
            UIRequest::Tap { view_id } => {
                let result = from_main(move || -> Result<(), String> {
                    let view = find_view(&view_id)?;

                    if view.is_hidden_in_tree() {
                        return Err(format!("View {} is hidden", view.label()));
                    }

                    // The touch pipeline takes physical pixels and converts
                    // to points itself, so the center scales up first.
                    let position = view.absolute_frame().center() * UIManager::scale();

                    for event in [TouchEvent::Began, TouchEvent::Ended] {
                        Input::process_touch_event(Touch {
                            id: 1,
                            position,
                            event,
                            button: MouseButton::Left,
                        });
                    }

                    Ok(())
                });

                match result {
                    // The snapshot runs a frame later, so a page swap or
                    // modal the tap triggered is already in the tree.
                    Ok(()) => Self::send_ui(),
                    Err(err) => AppCommand::Error(err),
                }
            }
        }
    }

    // Applies an edit on the main thread. On success records it to the edit
    // log and replies with a fresh tree. send_ui dispatches its own
    // from_main, so the snapshot runs one frame later, after layout.
    fn apply(edit: impl FnOnce() -> Result<EditEntry, String> + Send + 'static) -> AppCommand {
        match from_main(edit) {
            Ok(entry) => {
                edit_log::record(entry);
                Self::send_ui()
            }
            Err(err) => AppCommand::Error(err),
        }
    }

    fn send_ui() -> AppCommand {
        from_main(|| {
            let scale = UIManager::scale();
            let root = UIManager::root_view().view_to_inspect();
            UIResponse::SendUI { scale, root }.into()
        })
    }
}

fn entry(view: WeakView, what: impl ToString, old: impl ToString, new: impl ToString) -> EditEntry {
    EditEntry {
        timestamp: timestamp(),
        view:      view.label().to_string(),
        view_id:   weak_to_id(view),
        what:      what.to_string(),
        old:       old.to_string(),
        new:       new.to_string(),
    }
}

fn current_unix_seconds() -> u64 {
    #[cfg(not_wasm)]
    {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System clock is before the unix epoch")
            .as_secs()
    }
    #[cfg(wasm)]
    {
        use crate::gm::LossyConvert;

        hreads::now().lossy_convert()
    }
}

fn timestamp() -> String {
    #[cfg(not_wasm)]
    {
        Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }
    #[cfg(wasm)]
    {
        String::from(web_sys::js_sys::Date::new_0().to_iso_string())
    }
}

fn set_text(view: WeakView, text: &str) -> Result<String, String> {
    if let Some(label) = view.downcast::<Label>() {
        let old = label.text().to_string();
        label.set_text(text);
        return Ok(old);
    }
    if let Some(button) = view.downcast::<Button>() {
        let old = button.text().to_string();
        button.set_text(text);
        return Ok(old);
    }
    if let Some(field) = view.downcast::<TextField>() {
        let old = field.text().to_string();
        field.set_text(text);
        return Ok(old);
    }
    Err(format!("View {} has no text", view.label()))
}

fn find_view(id: &str) -> Result<WeakView, String> {
    fn search(view: WeakView, id: &str) -> Option<WeakView> {
        if weak_to_id(view) == id {
            return Some(view);
        }

        view.subviews().iter().find_map(|sub| search(sub.weak(), id))
    }

    search(UIManager::root_view(), id).ok_or_else(|| format!("View not found: {id}"))
}
