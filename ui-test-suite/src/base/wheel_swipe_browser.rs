use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, ScrollView, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::{checkpoint, inject_touches, system_input::wait_until},
};
use web_sys::{WheelEvent, WheelEventInit};

/// A sideways swipe of 2 fingers on a trackpad goes back or forward in the
/// browser only when no listener cancels its wheel events. Over empty space
/// the page leaves them to the browser. A list that can move that way keeps
/// them, a vertical move and a pinch stay with the page too.
#[view]
struct WheelSwipeBrowser {
    #[init]
    list:   ScrollView,
    empty:  Label,
    status: Label,
}

impl Setup for WheelSwipeBrowser {
    fn setup(self: Weak<Self>) {
        self.list.place().l(20).t(80).size(300, 200);
        self.list.set_content_size((900, 200));
        let wide = self.list.add_view::<Label>();
        wide.set_text("sideways list, 900 points wide");
        wide.place().l(0).t(0).size(900, 200);

        self.empty.set_text("empty space");
        self.empty.place().l(420).t(80).size(300, 200);

        self.status.place().l(20).t(20).size(700, 40);
    }
}

/// True when the browser keeps the event, false when a listener cancelled it.
fn send(delta_x: f64, delta_y: f64, ctrl: bool) -> bool {
    let init = WheelEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_delta_x(delta_x);
    init.set_delta_y(delta_y);
    init.set_delta_mode(WheelEvent::DOM_DELTA_PIXEL);
    init.set_ctrl_key(ctrl);
    let event = WheelEvent::new_with_event_init_dict("wheel", &init).expect("Failed to make a wheel event");
    web_sys::window()
        .expect("Failed to get browser window")
        .document()
        .expect("Failed to get browser document")
        .query_selector("canvas")
        .expect("Failed to query the canvas")
        .expect("No canvas on the page")
        .dispatch_event(&event)
        .expect("Failed to send a wheel event")
}

fn status(view: Weak<WheelSwipeBrowser>, text: &'static str) -> Result<()> {
    from_main(move || {
        view.status.set_text(text);
    });
    checkpoint(text)
}

impl ViewTest for WheelSwipeBrowser {
    fn canvas() -> (u32, u32) {
        (800, 320)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("570 180 m");
        from_main(|| -> Result<()> {
            ensure!(
                send(-40.0, 0.0, false),
                "a back swipe over empty space was cancelled"
            );
            ensure!(
                send(40.0, 0.0, false),
                "a forward swipe over empty space was cancelled"
            );
            ensure!(!send(0.0, 40.0, false), "the browser kept a vertical move");
            ensure!(!send(-40.0, 0.0, true), "the browser kept a pinch");
            Ok(())
        })?;
        status(view, "over empty space a sideways swipe goes to the browser")?;

        inject_touches("170 180 m");
        from_main(|| -> Result<()> {
            ensure!(
                send(-40.0, 0.0, false),
                "a back swipe over a list at its left edge was cancelled"
            );
            ensure!(
                !send(40.0, 0.0, false),
                "the browser kept a move the list can take"
            );
            Ok(())
        })?;
        wait_until("the list moves right", move || view.list.content_offset_x() < 0.0)?;
        status(view, "the list moved, it takes the sideways move")?;

        from_main(|| -> Result<()> {
            ensure!(
                !send(-40.0, 0.0, false),
                "the browser kept a move back of a list that moved"
            );
            Ok(())
        })?;
        status(view, "away from its edge the list keeps both ways")
    }
}
