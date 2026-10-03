//! The login session token of a hilen app, sealed on disk. `hilen` keeps the
//! `Login` token here, and a tool with no window reads the same login
//! through this crate without the engine.

mod encrypt;
mod session_key;
mod session_store;

pub use self::session_store::SessionStore;
