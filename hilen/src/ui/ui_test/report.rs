use std::{
    fmt::{self, Write},
    ops::Deref,
};

use anyhow::Result;

use crate::{
    deps::hreads::{from_main, is_main_thread},
    gm::flat::Rect,
    ui::{UIManager, View, ViewData, ViewFrame, ViewSubviews},
    ui_test::TEST_NAME,
};

const MAX_CHILDREN: usize = 30;

pub fn failure_report() -> Result<String> {
    if is_main_thread() {
        return Ok(r"No failure report. Collecting it waits for the next frame,
but frames only run when the main thread is free - calling this from the main thread would hang forever."
            .to_string());
    }

    let test_name = TEST_NAME.lock().clone();

    let mut report = String::new();

    let (resolution, scale) = from_main(|| (UIManager::window_resolution(), UIManager::scale()));

    writeln!(report, "Window resolution: {resolution:?}, scale: {scale}")?;
    writeln!(report, "{}", save_failure_screenshot(&test_name))?;
    writeln!(report, "View tree (label - frame - absolute frame):")?;
    report.push_str(&from_main(dump_view_tree)?);

    Ok(report)
}

#[cfg(not_wasm)]
fn save_failure_screenshot(test_name: &str) -> String {
    use std::env::temp_dir;

    use crate::{AppRunner, deps::hreads::wait_for_next_frame};

    wait_for_next_frame();

    let screenshot = match AppRunner::take_screenshot() {
        Ok(screenshot) => screenshot,
        Err(e) => return format!("Failed to take failure screenshot: {e}"),
    };

    let path = temp_dir().join(format!("ui_test_{}.png", test_name.replace(' ', "_")));

    match screenshot.save(&path) {
        Ok(()) => format!("Failure screenshot: {}", path.display()),
        Err(e) => format!("Failed to save failure screenshot: {e}"),
    }
}

// `temp_dir` is a hard panic on wasm, a page has nowhere to save a file,
// so the frame goes to the driver over the inspect socket instead.
#[cfg(all(wasm, feature = "inspect"))]
fn save_failure_screenshot(test_name: &str) -> String {
    use base64::{Engine, engine::general_purpose::STANDARD};

    use crate::{
        AppRunner,
        deps::hreads::wait_for_next_frame,
        inspect::{InspectService, protocol::AppCommand, web_transport::push},
    };

    wait_for_next_frame();

    let screenshot = match AppRunner::take_screenshot() {
        Ok(screenshot) => screenshot,
        Err(e) => return format!("Failed to take failure screenshot: {e}"),
    };

    match InspectService::encode_png(&screenshot) {
        Ok(png) => {
            push(AppCommand::FailureScreenshot {
                test:       test_name.to_string(),
                png_base64: STANDARD.encode(&png),
            });
            format!("Failure screenshot for {test_name} sent to the driver, see target/web-test/failures/")
        }
        Err(e) => format!("Failed to encode failure screenshot: {e}"),
    }
}

#[cfg(all(wasm, not(feature = "inspect")))]
fn save_failure_screenshot(test_name: &str) -> String {
    format!("No failure screenshot for {test_name}, the browser has no filesystem and no inspect socket")
}

fn dump_view_tree() -> Result<String> {
    let mut out = String::new();
    dump_view(UIManager::root_view().deref(), 0, &mut out)?;
    Ok(out)
}

fn dump_view(view: &dyn View, depth: usize, out: &mut String) -> fmt::Result {
    let indent = "  ".repeat(depth);
    let hidden = if view.is_hidden() { " [hidden]" } else { "" };

    writeln!(
        out,
        "{indent}{} - {} - {}{hidden}",
        view.label(),
        rect_str(view.frame()),
        rect_str(view.absolute_frame()),
    )?;

    let subviews = view.subviews();

    for sub in subviews.iter().take(MAX_CHILDREN) {
        dump_view(sub.deref(), depth + 1, out)?;
    }

    if subviews.len() > MAX_CHILDREN {
        writeln!(out, "{indent}  ... and {} more", subviews.len() - MAX_CHILDREN)?;
    }

    Ok(())
}

fn rect_str(rect: &Rect) -> String {
    format!(
        "[{}, {}, {}, {}]",
        rect.origin.x, rect.origin.y, rect.size.width, rect.size.height
    )
}
