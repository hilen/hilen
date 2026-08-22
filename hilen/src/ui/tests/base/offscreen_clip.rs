use anyhow::Result;

use crate::{
    deps::refs::Weak,
    ui::{ScrollView, Setup, ViewData, ViewTest, view},
};

/// A clip-to-bounds view placed fully past the screen edges must still render.
/// Its clipped width and height go negative, and converting that Rect to
/// Rect<u32> used to panic and abort the first frame on iOS.
#[view]
struct OffscreenClip {
    #[init]
    scroll: ScrollView,
}

impl Setup for OffscreenClip {
    fn setup(mut self: Weak<Self>) {
        self.scroll.set_content_size((200, 200));
        self.scroll.place().size(100, 100).tl(5000);
    }
}

impl ViewTest for OffscreenClip {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        // Reaching here means draw_view rendered the off-screen clip view
        // without panicking on the negative-size Rect -> Rect<u32> convert.
        Ok(())
    }
}
