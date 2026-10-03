//! Login with Google or Apple for an app with a `hilen-server` backend, behind
//! the `login` feature. `GoogleLoginButton` and `AppleLoginButton` are the
//! ready views, `Login` the calls under them for an app that draws its own.

#[cfg(feature = "ui-tests")]
mod apple_login_button_test;
mod client;
#[cfg(feature = "ui-tests")]
mod google_login_button_test;
mod login_button;
mod wire;

pub use client::{Login, Provider};
pub use login_button::{AppleLoginButton, GoogleLoginButton};
pub use wire::LoginUser;
