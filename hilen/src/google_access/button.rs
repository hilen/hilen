use ui_proc::view;

use crate::{
    deps::{refs::Weak, vents::Event},
    google_access::{AccountInfo, GoogleAccess, GoogleAccounts},
    sign_in_button::{GOOGLE, Look, sign_in_button},
    ui::{
        Button, ImageView, Label, RingSpinner, Setup, TextAlignment, Theme, ViewCallbacks, ViewData,
        ViewTouch,
    },
    window::image::Image,
};

const LINK: Look = Look {
    sign_in: "Add a Google account",
    ..GOOGLE
};

sign_in_button!(
    /// The sign in of the main Google account as one view, in the look of the
    /// Google button guidelines. A tap opens the sign in page and the button
    /// waits with a spinner and a cancel until the user is done there. Then
    /// `signed_in` gets the main account and its linked ones. It wants a
    /// frame about 240 by 40 points.
    ///
    /// `GoogleAccess::set_server` has to be called before the first tap.
    GoogleSignInButton,
    signed_in: Vec<AccountInfo>,
    GoogleAccounts::sign_in,
    GoogleAccess::cancel,
    GOOGLE
);

sign_in_button!(
    /// Links one more Google account to the main one, the twin of
    /// [`GoogleSignInButton`]. `linked` gets all accounts. An account that
    /// has to sign in again does it with this button too.
    GoogleLinkButton,
    linked: Vec<AccountInfo>,
    GoogleAccounts::link,
    GoogleAccess::cancel,
    LINK
);
