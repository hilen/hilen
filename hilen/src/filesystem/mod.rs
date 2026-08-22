mod paths;
mod read;

pub use self::paths::Paths;
pub(crate) use self::read::read_bytes;
#[cfg(android)]
pub(crate) use self::read::set_android_app;
pub use crate::assets::Assets;
