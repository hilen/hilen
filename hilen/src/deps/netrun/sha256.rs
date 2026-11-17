use data_encoding::HEXLOWER;
use ring::digest::{SHA256, digest};

/// The SHA-256 of `bytes` in lowercase hex, like `sha256sum` prints it.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    HEXLOWER.encode(digest(&SHA256, bytes).as_ref())
}

#[cfg(test)]
mod test {
    use super::sha256_hex;

    #[test]
    fn matches_sha256sum() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
