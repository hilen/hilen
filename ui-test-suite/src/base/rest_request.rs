use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use hilen::{
    Window,
    dispatch::{from_main, on_main},
    net::{Request, RestAPI},
    refs::Weak,
    ui::{Button, Label, Setup, Spinner, ViewFrame, ViewTest, async_link_button, view},
    ui_test::inject_touches,
};
use serde::Deserialize;

static REST_API: RestAPI = RestAPI::new("https://jsonplaceholder.typicode.com/");
static NOT_REQUESTED: AtomicBool = AtomicBool::new(false);

#[view]
struct RestRequest {
    #[init]
    button: Button,
    label:  Label,
}

impl RestRequest {
    async fn tapped(self: Weak<Self>) -> Result<()> {
        #[derive(Debug, Deserialize)]
        struct User {}

        static REQUEST: Request<(), Vec<User>> = REST_API.get("users");

        let spin = Spinner::lock();

        let users = REQUEST.await?;

        drop(spin);

        assert_eq!(users.len(), 10);

        on_main(move || {
            self.label.set_text(users.len());
            NOT_REQUESTED.store(false, Ordering::Relaxed);
        });

        Ok(())
    }
}

impl Setup for RestRequest {
    fn setup(self: Weak<Self>) {
        NOT_REQUESTED.store(true, Ordering::Relaxed);

        self.button.set_frame((50, 50, 100, 100));
        self.button.set_text("Send");

        self.label.set_frame((200, 50, 100, 100));
        self.label.set_text("Label");

        async_link_button!(self.button, tapped);
    }
}

impl ViewTest for RestRequest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Talks to a real REST endpoint over the network, so a headless run
        // builds the view and stops before the tap that sends the request.
        if Window::headless() {
            return Ok(());
        }

        inject_touches(
            "
                111  63   b
                111  63   e

            ",
        );

        while NOT_REQUESTED.load(Ordering::Relaxed) {
            std::hint::spin_loop();
        }

        let value = from_main(move || view.label.text.clone());

        assert_eq!(value, "10");

        Ok(())
    }
}
