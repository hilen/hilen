//! The login session token of a hilen app, sealed on disk. `hilen` keeps the
//! `Login` token here, and a tool with no window reads the same login
//! through this crate without the engine.

#[cfg(feature = "device-key")]
mod device_key;
mod encrypt;
mod secret_store;
mod session_key;
mod session_store;

#[cfg(feature = "device-key")]
pub use self::device_key::DeviceKey;
pub use self::{secret_store::SecretStore, session_store::SessionStore};
