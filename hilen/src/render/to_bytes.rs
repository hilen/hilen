use bytemuck::Pod;

pub(crate) trait ToBytes {
    fn to_bytes(&self) -> &[u8];
}

impl<T: Pod> ToBytes for [T] {
    fn to_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self)
    }
}

impl<T: Pod> ToBytes for T {
    fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}
