use bytemuck::{Pod, Zeroable};

use crate::gm::flat::Size;

#[repr(C)]
#[derive(Debug, Default, PartialEq, Copy, Clone, Zeroable, Pod)]
pub struct RectView {
    pub resolution: Size,
    #[allow(clippy::pub_underscore_fields)]
    pub _padding:   u64,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        // Web requirements
        assert_eq!(size_of::<RectView>() % 16, 0);
    }
}
