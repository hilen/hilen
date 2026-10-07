//! Input that comes from the system, for a test that must not inject it.
//!
//! A text field on a phone is edited in a system text field with the screen
//! keyboard, and that keyboard is drawn by another process. No event the
//! engine sends to itself reaches it. So on a phone these calls ask a helper
//! outside the app for a real tap on the screen or on a key. On iOS the helper
//! is the `SystemInput` `XCUITest` of the mobile project, the simulator lane
//! starts it and passes its address in `HILEN_SYSTEM_INPUT`.
//!
//! Everywhere else a key reaches a field as an engine event, so the same
//! calls inject. One test then runs on every platform.

use std::{sync::Arc, thread::sleep, time::Duration};

use anyhow::{Result, bail};
use web_time::Instant;

use crate::{
    deps::hreads::{from_main, wait_for_next_frame},
    gm::ToF32,
    ui::{NamedKey, ScreenKeyboard, UIManager},
    ui_test::{inject_keys, inject_named_key, inject_touches},
};

/// How long a state may take to show up. A system keyboard animates in and
/// a key tap goes through 2 other processes.
const WAIT_TIMEOUT: Duration = Duration::from_secs(20);

/// A helper outside the app sends the touches and the keys.
pub fn system_input() -> bool {
    helper::address().is_some()
}

/// A phone with no helper cannot type at all, and a test that went on with
/// injected keys would check a field the keys never reached.
fn require_injected_input() -> Result<()> {
    if cfg!(mobile) {
        bail!("This test needs system input. On iOS run it through `make ui-ios`.");
    }
    Ok(())
}

/// A tap at `x y` in the points of the test canvas.
pub fn tap(x: impl ToF32, y: impl ToF32) -> Result<()> {
    let (x, y) = (x.to_f32(), y.to_f32());
    if system_input() {
        let (x, y) = helper::screen_point(x, y);
        helper::request(&format!("tap {x} {y}"))?;
        return Ok(());
    }
    require_injected_input()?;
    inject_touches(format!("{x} {y} b\n{x} {y} e"));
    Ok(())
}

/// Types `text` into the field that is edited now. On a phone every
/// character is a tap on a key of the screen keyboard, so the text can only
/// hold what its first 2 pages have.
pub fn type_text(text: &str) -> Result<()> {
    if system_input() {
        helper::request(&format!("type {text}"))?;
        return Ok(());
    }
    require_injected_input()?;
    inject_keys(text);
    Ok(())
}

/// The Return key of the screen keyboard, Enter everywhere else.
pub fn press_return() -> Result<()> {
    if system_input() {
        helper::request("key return")?;
        return Ok(());
    }
    require_injected_input()?;
    inject_named_key(NamedKey::Enter);
    Ok(())
}

/// How long the stand in for a screen keyboard takes to move, the time of
/// the real one.
const KEYBOARD_SECONDS: f32 = 0.25;

/// Waits for the screen keyboard to stand still, up or away. Call it after
/// the tap that opens or closes it. A phone moves the real keyboard. Where
/// there is none the engine is told about one with its top edge at `top`,
/// in the points of the test canvas, so the same test runs everywhere. The
/// real keyboard has its own height, read the edge with
/// `ScreenKeyboard::top`.
pub fn screen_keyboard(up: bool, top: impl ToF32) -> Result<()> {
    let top = top.to_f32();
    if system_input() {
        helper::request(if up { "keyboard 1" } else { "keyboard 0" })?;
    } else {
        require_injected_input()?;
        from_main(move || ScreenKeyboard::moves_to(up.then_some(top), KEYBOARD_SECONDS));
    }
    wait_until("the screen keyboard stands still", move || {
        !ScreenKeyboard::is_moving() && ScreenKeyboard::top().is_some() == up
    })
}

/// One request to the helper, for a check only the system can answer, like
/// whether the system field hides its text. Fails where there is no helper.
pub fn system_request(request: &str) -> Result<String> {
    helper::request(request)
}

/// Waits until `check` holds on the main thread. System input lands some
/// frames after the call that asked for it, injected input at once, so a
/// test waits for the state instead of counting frames.
pub fn wait_until(what: &str, check: impl Fn() -> bool + Send + Sync + 'static) -> Result<()> {
    let check = Arc::new(check);
    let started = Instant::now();

    loop {
        let this_check = check.clone();
        if from_main(move || this_check()) {
            return Ok(());
        }
        if started.elapsed() > WAIT_TIMEOUT {
            bail!("Waited {} s for: {what}", WAIT_TIMEOUT.as_secs());
        }
        sleep(Duration::from_millis(20));
        wait_for_next_frame();
    }
}

#[cfg(not_wasm)]
mod helper {
    use std::{
        env::var,
        io::{BufRead, BufReader, Write},
        net::TcpStream,
        thread::sleep,
        time::Duration,
    };

    use anyhow::{Context, Result, anyhow, bail};
    use parking_lot::Mutex;
    use web_time::Instant;

    use crate::{deps::hreads::from_main, ui::UIManager};

    /// The helper starts next to the app and may still be booting when
    /// the first test asks for it.
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(180);

    static CONNECTION: Mutex<Option<BufReader<TcpStream>>> = Mutex::new(None);

    pub(super) fn address() -> Option<String> {
        var("HILEN_SYSTEM_INPUT").ok()
    }

    /// Canvas points to screen points. The canvas is counted in window
    /// pixels at the UI scale of the run, the helper taps in the points
    /// of the screen.
    pub(super) fn screen_point(x: f32, y: f32) -> (f32, f32) {
        // The screen scale is read from the window, which lives on the
        // main thread.
        let ratio = UIManager::scale() / from_main(UIManager::display_scale);
        (x * ratio, y * ratio)
    }

    fn connect(address: &str) -> Result<BufReader<TcpStream>> {
        let started = Instant::now();
        loop {
            match TcpStream::connect(address) {
                Ok(stream) => return Ok(BufReader::new(stream)),
                Err(err) if started.elapsed() > CONNECT_TIMEOUT => {
                    return Err(err).context(format!("No system input helper at {address}"));
                }
                Err(_) => sleep(Duration::from_millis(500)),
            }
        }
    }

    pub(super) fn request(line: &str) -> Result<String> {
        let address = address().ok_or_else(|| anyhow!("No system input helper for `{line}`"))?;

        let mut connection = CONNECTION.lock();
        if connection.is_none() {
            *connection = Some(connect(&address)?);
        }
        let stream = connection.as_mut().expect("connected above");

        stream.get_mut().write_all(format!("{line}\n").as_bytes())?;

        let mut reply = String::new();
        stream.read_line(&mut reply)?;
        let reply = reply.trim_end();

        if let Some(answer) = reply.strip_prefix("ok") {
            return Ok(answer.trim().to_string());
        }
        bail!("System input `{line}` failed: {reply}")
    }
}

/// A page has no socket and no helper, its keys are engine events.
#[cfg(wasm)]
mod helper {
    use anyhow::{Result, bail};

    pub(super) fn address() -> Option<String> {
        None
    }

    pub(super) fn screen_point(x: f32, y: f32) -> (f32, f32) {
        (x, y)
    }

    pub(super) fn request(line: &str) -> Result<String> {
        bail!("No system input helper for `{line}`")
    }
}

/// What the screen shows inside an area, read from a capture of the whole
/// screen. A capture of the engine frame holds no system view, this one
/// does. The box is in canvas points from the corner of the area.
#[derive(Debug, Clone, Copy)]
pub struct ScreenInk {
    pub left:   f32,
    pub top:    f32,
    pub right:  f32,
    pub bottom: f32,
    /// The color farthest from the background, as red, green and blue.
    pub color:  [u8; 3],
}

/// The box and the color of what is drawn over the background of the area
/// `x y width height`, in canvas points. Needs the helper, see the module.
pub fn screen_ink(x: f32, y: f32, width: f32, height: f32) -> Result<ScreenInk> {
    let (left, top) = helper::screen_point(x, y);
    let (width, height) = helper::screen_point(width, height);
    let reply = helper::request(&format!("ink {left} {top} {width} {height}"))?;

    let parts: Vec<&str> = reply.split(' ').collect();
    let &[left, top, right, bottom, red, green, blue] = parts.as_slice() else {
        bail!("Unexpected ink reply `{reply}`");
    };

    // The reply is in screen pixels, a canvas point is that many of them.
    let scale = UIManager::scale();
    Ok(ScreenInk {
        left:   left.parse::<f32>()? / scale,
        top:    top.parse::<f32>()? / scale,
        right:  right.parse::<f32>()? / scale,
        bottom: bottom.parse::<f32>()? / scale,
        color:  [red.parse()?, green.parse()?, blue.parse()?],
    })
}
