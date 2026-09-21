//! Google login for an app with a `hilen-server` backend, behind the `login`
//! feature. `GoogleLoginButton` is the ready view, `GoogleLogin` the calls
//! under it for an app that draws its own.

mod google_login;
mod google_login_button;
#[cfg(feature = "ui-tests")]
mod google_login_button_test;
mod wire;

pub use google_login::GoogleLogin;
pub use google_login_button::GoogleLoginButton;
pub use wire::LoginUser;
