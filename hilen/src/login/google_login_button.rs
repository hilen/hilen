use ui_proc::view;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::color::Color,
    login::{GoogleLogin, LoginUser},
    ui::{Button, DynamicColor, ImageView, Label, RingSpinner, Setup, TextAlignment, ViewData, ViewTouch},
    window::image::Image,
};

// The light and dark looks of the Google sign in button guidelines,
// https://developers.google.com/identity/branding-guidelines.
const FILL: DynamicColor = DynamicColor::new(Color::hex("#ffffff"), Color::hex("#131314"));
const BORDER: DynamicColor = DynamicColor::new(Color::hex("#747775"), Color::hex("#8e918f"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1f1f1f"), Color::hex("#e3e3e3"));
const FAINT_TEXT: DynamicColor = DynamicColor::new(Color::hex("#5f6368"), Color::hex("#9aa0a6"));

const SIGN_IN: &str = "Sign in with Google";
const WAITING: &str = "Waiting for login";

/// The whole Google login as one view. A tap opens the login page and the
/// button waits with a spinner and a cancel until the user is done there.
/// It wants a frame about 240 by 40 points.
///
/// `GoogleLogin::set_server` has to be called before the first tap.
#[view]
pub struct GoogleLoginButton {
    /// The user finished the login. The session is already stored.
    pub logged_in: Event<LoginUser>,
    /// The login did not work, with a message to show. A cancel is not a
    /// failure.
    pub failed:    Event<String>,

    waiting: bool,

    #[init]
    mark:    ImageView,
    spinner: RingSpinner,
    title:   Label,
    cancel:  Button,
}

impl GoogleLoginButton {
    pub fn is_waiting(&self) -> bool {
        self.waiting
    }

    fn start(mut self: Weak<Self>) {
        if self.waiting {
            return;
        }
        self.set_waiting(true);

        GoogleLogin::start(move |result| {
            // The view can be long gone when the browser part ends.
            if self.is_null() {
                return;
            }
            self.set_waiting(false);
            match result {
                Ok(user) => self.logged_in.trigger(user),
                Err(error) => self.failed.trigger(error.to_string()),
            }
        });
    }

    fn stop(mut self: Weak<Self>) {
        GoogleLogin::cancel();
        self.set_waiting(false);
    }

    fn set_waiting(&mut self, waiting: bool) {
        self.waiting = waiting;
        self.mark.set_hidden(waiting);
        self.spinner.set_hidden(!waiting);
        self.cancel.set_hidden(!waiting);
        self.title.set_text(if waiting { WAITING } else { SIGN_IN });
    }
}

#[cfg(feature = "ui-tests")]
impl GoogleLoginButton {
    /// The waiting look without a login behind it. A turning ring has no
    /// pixels a test could pin, so it holds still.
    pub(super) fn show_waiting_frozen(&mut self) {
        self.set_waiting(true);
        self.spinner.set_speed(0).set_angle(0);
    }
}

impl Setup for GoogleLoginButton {
    fn setup(mut self: Weak<Self>) {
        self.set_color(FILL);
        self.set_border_color(BORDER);
        self.set_border_width(1).set_corner_radius(4);

        self.mark.set_image(Image::from_file_data(
            include_bytes!("google_g.svg"),
            "hilen_google_g.svg",
        ));
        self.mark.place().l(12).center_y().size(18, 18);

        self.spinner.set_ring_color(FAINT_TEXT);
        self.spinner.place().l(12).center_y().size(18, 18);

        self.title.set_text_size(14).set_text_color(TEXT);
        self.title.set_alignment(TextAlignment::Left);
        // Full width. The waiting text is short enough to end before cancel.
        self.title.place().l(40).r(12).tb(0);

        self.cancel.set_text("Cancel");
        self.cancel.set_text_size(13).set_text_color(FAINT_TEXT);
        self.cancel.place().r(4).tb(4).w(64);

        // The touch view that signed up last is asked first, and `on_tap` is
        // what signs a button up. So the whole view goes first and cancel
        // after it, the other way round this view takes every tap on cancel.
        self.enable_touch();
        self.touch().up_inside.sub(self, move || self.start());
        self.cancel.on_tap(move || self.stop());

        self.set_waiting(false);
    }
}
