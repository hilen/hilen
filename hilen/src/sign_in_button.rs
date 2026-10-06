//! The body of every sign in button, the Google and the Apple look. The login
//! buttons of `login` and the account buttons of `google_access` are this one
//! view written out per button by a macro, each with the call it starts.

use crate::{gm::color::Color, ui::DynamicColor};

pub(crate) const WAITING: &str = "Waiting for login";

/// An svg mark with the name its image is cached under.
pub(crate) type Mark = (&'static [u8], &'static str);

/// What differs between the buttons.
#[derive(Clone, Copy)]
pub(crate) struct Look {
    pub fill:       DynamicColor,
    pub border:     DynamicColor,
    pub text:       DynamicColor,
    pub faint_text: DynamicColor,
    pub sign_in:    &'static str,
    pub light_mark: Mark,
    pub dark_mark:  Mark,
}

const GOOGLE_MARK: Mark = (include_bytes!("login/google_g.svg"), "hilen_google_g.svg");

// The light and dark looks of the Google sign in button guidelines,
// https://developers.google.com/identity/branding-guidelines.
pub(crate) const GOOGLE: Look = Look {
    fill:       DynamicColor::new(Color::hex("#ffffff"), Color::hex("#131314")),
    border:     DynamicColor::new(Color::hex("#747775"), Color::hex("#8e918f")),
    text:       DynamicColor::new(Color::hex("#1f1f1f"), Color::hex("#e3e3e3")),
    faint_text: DynamicColor::new(Color::hex("#5f6368"), Color::hex("#9aa0a6")),
    sign_in:    "Sign in with Google",
    light_mark: GOOGLE_MARK,
    dark_mark:  GOOGLE_MARK,
};

// The black and the white button of the Sign in with Apple guidelines,
// https://developer.apple.com/design/human-interface-guidelines/sign-in-with-apple.
// Black on a light screen and white on a dark one, the logo in the other
// color.
#[cfg(feature = "login")]
pub(crate) const APPLE: Look = Look {
    fill:       DynamicColor::new(Color::hex("#000000"), Color::hex("#ffffff")),
    border:     DynamicColor::new(Color::hex("#000000"), Color::hex("#ffffff")),
    text:       DynamicColor::new(Color::hex("#ffffff"), Color::hex("#000000")),
    faint_text: DynamicColor::new(Color::hex("#a1a1a6"), Color::hex("#6e6e73")),
    sign_in:    "Sign in with Apple",
    light_mark: (
        include_bytes!("login/apple_logo_white.svg"),
        "hilen_apple_logo_white.svg",
    ),
    dark_mark:  (
        include_bytes!("login/apple_logo_black.svg"),
        "hilen_apple_logo_black.svg",
    ),
};

/// One button view per call, the same code under each. A generic view would
/// do it too, but then the view could not sit in a UI test.
///
/// `$start` is a fn that takes the callback of the sign in, `$cancel` a fn
/// that stops waiting for it. `$done` names the event of a finished sign in
/// and `$out` is what it carries.
macro_rules! sign_in_button {
    ($(#[$doc:meta])* $name:ident, $done:ident : $out:ty, $start:path, $cancel:path, $look:expr) => {
        $(#[$doc])*
        #[view]
        pub struct $name {
            /// The user finished the sign in, what it gave is stored already.
            pub $done:  Event<$out>,
            /// The login did not work, with a message to show. A cancel is
            /// not a failure.
            pub failed:    Event<String>,

            waiting:      bool,
            sign_in_text: Option<String>,
            waiting_text: Option<String>,

            #[init]
            mark:    ImageView,
            spinner: RingSpinner,
            title:   Label,
            cancel:  Button,
        }

        impl $name {
            pub fn is_waiting(&self) -> bool {
                self.waiting
            }

            /// The title of the button at rest, for an app in another
            /// language. English when not set.
            pub fn set_sign_in_text(&mut self, text: impl ToString) -> &mut Self {
                self.sign_in_text = Some(text.to_string());
                self.show_title();
                self
            }

            /// The title while the user is in the browser. English when not
            /// set. It has to end before the cancel button.
            pub fn set_waiting_text(&mut self, text: impl ToString) -> &mut Self {
                self.waiting_text = Some(text.to_string());
                self.show_title();
                self
            }

            /// The text of the cancel button, which is 64 points wide.
            /// English when not set.
            pub fn set_cancel_text(&mut self, text: impl ToString) -> &mut Self {
                self.cancel.set_text(text.to_string());
                self
            }

            fn show_title(&mut self) {
                let text = if self.waiting {
                    self.waiting_text.as_deref().unwrap_or($crate::sign_in_button::WAITING)
                } else {
                    self.sign_in_text.as_deref().unwrap_or($look.sign_in)
                };
                self.title.set_text(text);
            }

            fn start(mut self: Weak<Self>) {
                if self.waiting {
                    return;
                }
                self.set_waiting(true);

                $start(move |result| {
                    // The view can be long gone when the browser part ends.
                    if self.is_null() {
                        return;
                    }
                    self.set_waiting(false);
                    match result {
                        Ok(out) => self.$done.trigger(out),
                        Err(error) => self.failed.trigger(error.to_string()),
                    }
                });
            }

            fn stop(mut self: Weak<Self>) {
                $cancel();
                self.set_waiting(false);
            }

            fn set_waiting(&mut self, waiting: bool) {
                self.waiting = waiting;
                self.mark.set_hidden(waiting);
                self.spinner.set_hidden(!waiting);
                self.cancel.set_hidden(!waiting);
                self.show_title();
            }

            fn show_mark(&self) {
                let (data, name) = match Theme::current() {
                    Theme::Light => $look.light_mark,
                    Theme::Dark => $look.dark_mark,
                };
                self.mark.set_image(Image::from_file_data(data, name));
            }
        }

        #[cfg(feature = "ui-tests")]
        impl $name {
            /// The waiting look without a login behind it. A turning ring
            /// has no pixels a test could pin, so it holds still.
            pub(super) fn show_waiting_frozen(&mut self) {
                self.set_waiting(true);
                self.spinner.set_speed(0).set_angle(0);
            }
        }

        impl Setup for $name {
            fn setup(mut self: Weak<Self>) {
                self.set_color($look.fill);
                self.set_border_color($look.border);
                self.set_border_width(1).set_corner_radius(4);

                self.show_mark();
                self.mark.place().l(12).center_y().size(18, 18);

                self.spinner.set_ring_color($look.faint_text);
                self.spinner.place().l(12).center_y().size(18, 18);

                self.title.set_text_size(14).set_text_color($look.text);
                self.title.set_alignment(TextAlignment::Left);
                // Full width. The waiting text is short enough to end before
                // cancel.
                self.title.place().l(40).r(12).tb(0);

                self.cancel.set_text("Cancel");
                // A button brings its own fill, white also on the black
                // Apple button.
                self.cancel.set_color($look.fill);
                self.cancel.set_text_size(13).set_text_color($look.faint_text);
                self.cancel.place().r(4).tb(4).w(64);

                // The touch view that signed up last is asked first, and
                // `on_tap` is what signs a button up. So the whole view goes
                // first and cancel after it, the other way round this view
                // takes every tap on cancel.
                self.enable_touch();
                self.touch().up_inside.sub(self, move || self.start());
                self.cancel.on_tap(move || self.stop());

                self.set_waiting(false);
            }
        }

        impl ViewCallbacks for $name {
            fn theme_changed(&mut self) {
                self.show_mark();
            }
        }
    };
}
pub(crate) use sign_in_button;
