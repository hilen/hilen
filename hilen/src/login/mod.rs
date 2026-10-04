//! Login with Google or Apple for an app with a `hilen-server` backend, behind
//! the `login` feature. `GoogleLoginButton` and `AppleLoginButton` are the
//! ready views, `Login` the calls under them for an app that draws its own.
//! A device with no keyboard, a TV, logs in through a phone: `QrCodeView`
//! shows the link of `Login::start_with_code`.

#[cfg(feature = "ui-tests")]
mod apple_login_button_test;
mod client;
#[cfg(feature = "ui-tests")]
mod google_login_button_test;
mod login_button;
#[cfg(feature = "ui-tests")]
mod login_button_texts_test;
mod qr_code_view;
#[cfg(feature = "ui-tests")]
mod qr_code_view_test;
mod wire;

pub use client::{Login, LoginCode, Provider};
pub use login_button::{AppleLoginButton, GoogleLoginButton};
pub use qr_code_view::QrCodeView;
pub use wire::LoginUser;
