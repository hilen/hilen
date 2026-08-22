#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]

pub trait CheckedConvert<T> {
    fn checked_convert(self) -> T;
}

impl CheckedConvert<u32> for i32 {
    fn checked_convert(self) -> u32 {
        assert!(self >= 0, "Checked convert sign loss");
        self as u32
    }
}

pub const fn checked_usize_to_u32(val: usize) -> u32 {
    assert!(val <= u32::MAX as usize);
    val as u32
}
