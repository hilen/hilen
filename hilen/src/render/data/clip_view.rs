use bytemuck::{Pod, Zeroable};

use crate::gm::flat::Size;

/// The uniform of `ui_clip.wgsl`.
#[repr(C)]
#[derive(Debug, Default, PartialEq, Copy, Clone, Zeroable, Pod)]
pub(crate) struct ClipView {
    pub resolution: Size,
    pub threshold:  f32,
    #[allow(clippy::pub_underscore_fields)]
    pub _padding:   u32,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        // Web requirements
        assert_eq!(size_of::<ClipView>() % 16, 0);
    }
}
