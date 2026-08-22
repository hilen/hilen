use serde::{Serialize, de::DeserializeOwned};

pub trait Storable: Serialize + DeserializeOwned {}

impl<T: Serialize + DeserializeOwned + Send> Storable for T {}
