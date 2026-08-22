// use std::{marker::PhantomData, str::from_utf8};
//
// use serde_json::{from_str, to_string};
//
// use crate::store::{
//     OnDisk,
//     encrypt::{EncryptionKey, decrypt, encrypt},
//     storable::Storable,
// };
//
// pub struct OnDiskEncrypted<T: Storable> {
//     inner: OnDisk<Vec<u8>>,
//     _p:    PhantomData<T>,
// }
//
// impl<T: Storable + Default> OnDiskEncrypted<T> {
//     pub const fn new(name: &'static str) -> Self {
//         Self {
//             inner: OnDisk::new(name),
//             _p:    PhantomData,
//         }
//     }
//
//     pub fn set(&self, val: impl Into<T>, key: &EncryptionKey) {
//         let val = val.into();
//         let string = to_string(&val).unwrap();
//         let encrypted = encrypt(string.as_bytes(), key);
//         self.inner.set(encrypted);
//     }
//
//     pub fn get(&self, key: &EncryptionKey) -> T {
//         let encrypted = self.inner.get_or_init();
//         if encrypted.is_empty() {
//             return T::default();
//         }
//         let string = decrypt(encrypted.as_slice(), key);
//         from_str(from_utf8(&string).unwrap()).unwrap()
//     }
//
//     pub fn reset(&self, key: &EncryptionKey) {
//         self.set(T::default(), key);
//     }
// }
//
// #[cfg(test)]
// mod test {
//     use anyhow::Result;
//     use fake::{Fake, Faker};
//     use serde::{Deserialize, Serialize};
//     use tokio::spawn;
//
//     use crate::store::{EncryptionKey, on_disk_encrypted::OnDiskEncrypted};
//
//     #[derive(Debug, PartialEq, Default, Serialize, Deserialize, Clone)]
//     struct Data {
//         number: i32,
//         string: String,
//     }
//
//     static STORED: OnDiskEncrypted<i32> =
// OnDiskEncrypted::new("stored_i32_encrypted_test");     static STORED_STRUCT:
// OnDiskEncrypted<Data> = OnDiskEncrypted::new("stored_struct_encrypted_test");
//
//     static KEY: EncryptionKey = [
//         1, 2, 3, 4, 5, 6, 7, 8, 9, 8, 7, 6, 5, 4, 3, 2, 1, 2, 5, 5, 5, 5, 5,
// 5, 5, 4, 3, 2, 1, 2, 3, 4, 5, 6,         7, 8, 9, 0, 9, 87, 6, 5, 3, 3,
//     ];
//
//     fn check_send<T: Send>(_send: &T) {}
//     fn check_sync<T: Sync>(_sync: &T) {}
//
//     #[tokio::test]
//     async fn encrypted_stored() -> Result<()> {
//         check_send(&STORED);
//         check_sync(&STORED);
//         check_send(&STORED_STRUCT);
//         check_sync(&STORED_STRUCT);
//
//         STORED.set(10, &KEY);
//         STORED.reset(&KEY);
//         assert_eq!(STORED.get(&KEY), i32::default());
//
//         for _ in 0..10 {
//             let rand: i32 = Faker.fake();
//
//             spawn(async move {
//                 STORED.set(rand, &KEY);
//             })
//             .await?;
//
//             spawn(async move {
//                 assert_eq!(STORED.get(&KEY), rand);
//             })
//             .await?;
//         }
//
//         let data = Data {
//             number: 555,
//             string: "Helloyyyy".to_string(),
//         };
//
//         STORED_STRUCT.set(data.clone(), &KEY);
//
//         let loaded_data = STORED_STRUCT.get(&KEY);
//
//         assert_eq!(data, loaded_data);
//
//         Ok(())
//     }
// }
