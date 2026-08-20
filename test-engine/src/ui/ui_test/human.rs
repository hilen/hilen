use std::{
    env::var,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, channel},
    },
    thread::sleep,
    time::Duration,
};

use hreads::from_main;
use log::warn;
use parking_lot::Mutex;
use plat::Platform;

use crate::{
    gm::{
        LossyConvert,
        color::{BLACK, CLEAR, Color, U8Color, WHITE},
    },
    ui::{
        Button, Container, Label, Setup, TouchStack, UIManager, ViewData, ViewFrame, ViewSubviews, WeakView,
    },
    ui_test::TEST_NAME,
    window::Window,
};

static HUMAN_MODE: AtomicBool = AtomicBool::new(false);
static ADVANCE: OnceLock<(Sender<()>, Mutex<Receiver<()>>)> = OnceLock::new();

/// Slows down injections and holds after each test until the user advances,
/// space on desktop, a tap on a phone, so a human can watch the tests run.
/// Enabled by `--human` in ui-test, `TE_HUMAN` on a device, `te_human` in
/// the browser.
pub fn enable_human_mode() {
    HUMAN_MODE.store(true, Ordering::Relaxed);
}

pub fn human_mode() -> bool {
    HUMAN_MODE.load(Ordering::Relaxed)
}

fn delay() -> Duration {
    let ms = var("UI_TEST_HUMAN_DELAY").ok().and_then(|ms| ms.parse().ok()).unwrap_or(400);
    Duration::from_millis(ms)
}

pub(crate) fn human_pause() {
    if human_mode() {
        sleep(delay());
    }
}

/// Shorter pause for moved touches, a full delay per move would
/// stretch a recorded drag into minutes.
pub(crate) fn human_pause_quick() {
    if human_mode() {
        sleep(delay() / 8);
    }
}

/// Holds with `label` as the prompt, so a test can make a
/// state visible that no injection paces, like a browser URL change
/// that would otherwise flash by. A no-op outside human mode.
pub fn human_checkpoint(label: &str) {
    if !human_mode() {
        return;
    }

    hold(format!("{label} - continue"));
}

pub(crate) fn hold_for_human() {
    if !human_mode() {
        return;
    }

    let test_name = TEST_NAME.lock().clone();
    hold(format!("{test_name}: OK - continue"));
}

/// Size of the swatch showing a probe's color, and of the outline drawn
/// around the probed pixel. The outline is a black square in a white one,
/// so it stays visible on any background.
const SWATCH: f32 = 8.0;
const OUTLINE: f32 = 12.0;

/// Marks every checked pixel with a square around it, the pixel in the
/// center, puts a swatch of the color that probe pins just outside the
/// square's top right corner, and holds until the user advances.
///
/// The outline alone says where a probe sits, not what it asserts, and
/// that is the half that matters. A probe pinning the background beside
/// a glyph looks exactly like one pinning the glyph.
pub(crate) fn show_probes(probes: &[((u32, u32), U8Color)], test_name: &str, index: usize) {
    let probes = probes.to_vec();

    let markers = from_main(move || {
        let mut markers: Vec<WeakView> = vec![];

        let mut add = |frame: (f32, f32, f32, f32), fill, border| {
            let mut view = Container::new();
            view.set_z_position(0.1);
            view.set_color(fill)
                .set_border_color(border)
                .set_border_width(1)
                .set_frame(frame);
            markers.push(UIManager::root_view().add_subview_to_root(view));
        };

        for ((x, y), color) in probes {
            let x: f32 = x.lossy_convert();
            let y: f32 = y.lossy_convert();

            for (size, border) in [(OUTLINE, WHITE), (OUTLINE - 2.0, BLACK)] {
                add((x - size / 2.0, y - size / 2.0, size, size), CLEAR, border);
            }

            // Outside the outline, so it never covers the probed pixel.
            let corner = OUTLINE / 2.0;
            add(
                (x + corner, y - corner - SWATCH, SWATCH, SWATCH),
                color.into(),
                WHITE,
            );
        }

        markers
    });

    hold(format!("{test_name} check {index}: continue"));

    from_main(move || {
        for mut marker in markers {
            marker.remove_from_superview();
        }
    });
}

/// Holds until the user advances. On desktop the prompt is the window
/// title and space advances. A phone has no window title and no key to
/// press, so there the prompt is a bar over the bottom of the canvas and
/// a tap anywhere advances. The overlay is its own touch layer, so the
/// tap never reaches the views under review.
fn hold(prompt: String) {
    Window::set_title(prompt.clone());

    let overlay = if Platform::MOBILE {
        Some(show_tap_prompt(prompt))
    } else {
        None
    };

    wait_for_advance();

    if let Some(mut overlay) = overlay {
        from_main(move || {
            TouchStack::pop_layer(overlay.weak_view());
            overlay.remove_from_superview();
        });
    }
}

const PROMPT_BAR_HEIGHT: f32 = 40.0;

fn show_tap_prompt(prompt: String) -> WeakView {
    from_main(move || {
        let mut overlay = Button::new();
        overlay.set_z_position(0.05);

        let overlay = UIManager::root_view().add_subview_to_root(overlay);
        overlay.place().back();
        // After the add. A button paints itself white in its setup, which
        // runs on add and would cover the view under review.
        overlay.set_color(CLEAR);

        // The layer first, then the tap. Enabling touch registers the view
        // in the layer its superview chain reaches, so the overlay must be
        // in the tree and its own layer must already be on the stack.
        TouchStack::push_layer(overlay);

        let sender = advance().0.clone();
        overlay.downcast_view::<Button>().unwrap().on_tap(move || {
            if sender.send(()).is_err() {
                warn!("Failed to send human continue signal");
            }
        });

        let bar = overlay.add_view::<Label>();
        bar.set_color(Color::rgba(0.0, 0.0, 0.0, 0.6))
            .set_text_color(WHITE)
            .set_text_size(16)
            .set_text(prompt);
        bar.place().lrb(0).h(PROMPT_BAR_HEIGHT);

        overlay
    })
}

fn advance() -> &'static (Sender<()>, Mutex<Receiver<()>>) {
    ADVANCE.get_or_init(|| {
        let (sender, receiver) = channel();
        let key_sender = sender.clone();

        from_main(move || {
            UIManager::keymap().add(UIManager::root_view(), ' ', move || {
                if key_sender.send(()).is_err() {
                    warn!("Failed to send human continue signal");
                }
            });
        });

        (sender, Mutex::new(receiver))
    })
}

fn wait_for_advance() {
    let receiver = advance().1.lock();

    while receiver.try_recv().is_ok() {}

    if receiver.recv().is_err() {
        warn!("Failed to receive human continue signal");
    }
}
