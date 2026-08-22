// use aes_gcm::{
//     Aes256Gcm,
//     aead::{Aead, KeyInit, Nonce},
// };
//
// const AES_KEY_SIZE: usize = 32;
// const NONCE_SIZE: usize = 12;
// pub const KEY_SIZE: usize = AES_KEY_SIZE + NONCE_SIZE;
//
// pub type EncryptionKey = [u8; KEY_SIZE];
//
// fn to_key_nonce(key: &EncryptionKey) -> (aes_gcm::Key<Aes256Gcm>,
// Nonce<Aes256Gcm>) {     let nonce = &key[AES_KEY_SIZE..];
//     let key = &key[..AES_KEY_SIZE];
//
//     (
//         *aes_gcm::Key::<Aes256Gcm>::from_slice(key),
//         *Nonce::<Aes256Gcm>::from_slice(nonce),
//     )
// }
//
// pub fn encrypt(data: &[u8], key: &EncryptionKey) -> Vec<u8> {
//     let (key, nonce) = to_key_nonce(key);
//     let cipher = Aes256Gcm::new(&key);
//     cipher.encrypt(&nonce, data).unwrap()
// }
//
// pub fn decrypt(data: &[u8], key: &EncryptionKey) -> Vec<u8> {
//     let (key, nonce) = to_key_nonce(key);
//     let cipher = Aes256Gcm::new(&key);
//     cipher.decrypt(&nonce, data).unwrap()
// }
//
// #[cfg(test)]
// mod test {
//
//     use rand::{RngCore, rng};
//
//     use crate::store::{
//         EncryptionKey,
//         encrypt::{KEY_SIZE, decrypt, encrypt},
//     };
//
//     #[test]
//     fn test() {
//         let mut key: EncryptionKey = [0; KEY_SIZE];
//         rng().fill_bytes(&mut key);
//
//         let encrypted = encrypt(
//             b"SOKOLLL!! fjdsa fjasd;k flkdsa hfjklsda lfdkkadshksalkjaskjd
// jljljsdslkjsksj",             &key,
//         );
//
//         let decrypted = decrypt(&encrypted, &key);
//
//         assert_eq!(
//             decrypted,
//             b"SOKOLLL!! fjdsa fjasd;k flkdsa hfjklsda lfdkkadshksalkjaskjd
// jljljsdslkjsksj"         );
//     }
// }
