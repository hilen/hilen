//! Google API access for an app that keeps its Google tokens on the device,
//! behind the `google-access` feature. The server half is
//! `hilen_server::google_access`, the two only share the wire.
//!
//! The Google client and its secret live on the backend of the app. A sign in
//! runs in the browser against that backend, and the tokens come back sealed
//! for a key only this device has. [`GoogleAccess`] is the sign in and the
//! renewal of an access token. [`GoogleAccounts`] on top of it keeps 1 main
//! account and its linked accounts, the same on every device of the user.

mod accounts;
#[cfg(all(test, not_wasm))]
mod accounts_test;
mod client;
mod vault;
mod wire;

pub use accounts::{AccountInfo, GoogleAccounts};
pub use client::{AccessToken, GoogleAccess, RenewError};
pub use wire::{GoogleAccount, Role};
