use anyhow::Result;
use ui_proc::view;

use crate::{
    deps::{refs::Weak, vents::Event},
    login::{Login, LoginUser, Provider},
    sign_in_button::{APPLE, GOOGLE, sign_in_button},
    ui::{
        Button, ImageView, Label, RingSpinner, Setup, TextAlignment, Theme, ViewCallbacks, ViewData,
        ViewTouch,
    },
    window::image::Image,
};

fn start_google(done: impl FnOnce(Result<LoginUser>) + Send + 'static) {
    Login::start(Provider::Google, done);
}

fn start_apple(done: impl FnOnce(Result<LoginUser>) + Send + 'static) {
    Login::start(Provider::Apple, done);
}

sign_in_button!(
    /// The whole Google login as one view. A tap opens the login page and the
    /// button waits with a spinner and a cancel until the user is done there.
    /// It wants a frame about 240 by 40 points.
    ///
    /// `Login::set_server` has to be called before the first tap.
    GoogleLoginButton,
    logged_in: LoginUser,
    start_google,
    Login::cancel,
    GOOGLE
);

sign_in_button!(
    /// The whole Apple login as one view, the twin of [`GoogleLoginButton`].
    /// Black on a light screen and white on a dark one. It wants a frame about
    /// 240 by 40 points.
    ///
    /// `Login::set_server` has to be called before the first tap, and the
    /// backend needs its Apple config.
    AppleLoginButton,
    logged_in: LoginUser,
    start_apple,
    Login::cancel,
    APPLE
);
