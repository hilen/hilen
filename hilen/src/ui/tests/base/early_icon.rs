use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{
        color::{RED, WHITE},
        flat::Size,
    },
    image::svg_sources,
    ui::{ImageView, Setup, UIManager, ViewFrame, ViewTest, view},
    ui_test::check_colors,
};

/// No asset group has this file, so the icon is asked for before its bytes,
/// like a tab bar made in the setup of a page before the boot group landed.
const NAME: &str = "early-icon-test.svg";

/// A black square, the tint makes it red.
const SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><rect width="24" height="24" fill="#000000"/></svg>"##;

const WAITING: &str = r"
    148  148 - #ffffff
";

const LANDED: &str = r"
    148  148 - #ff0000
    110  110 - #ff0000
    186  186 - #ff0000
";

/// The control center and beekeeper asked for their tab icons before the
/// boot group of the page landed. The engine stored the trash can picture
/// under the name of each icon and kept it for good.
#[view]
struct EarlyIcon {
    #[init]
    icon: ImageView,
}

impl Setup for EarlyIcon {
    fn setup(self: Weak<Self>) {
        UIManager::set_clear_color(WHITE);
        self.icon.set_frame((100, 100, 96, 96));
        self.icon.set_icon(NAME, RED);
    }
}

impl ViewTest for EarlyIcon {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let size = from_main(move || view.icon.image().size);
        ensure!(
            size == Size::new(1, 1),
            "an icon asked for before its file should wait see-through, its image is {size:?}"
        );
        check_colors(WAITING)?;

        // What a download of the file does when its bytes come, see
        // `image/pending.rs`.
        from_main(|| {
            svg_sources::store(NAME, SVG);
            crate::image::early::landed(NAME, SVG);
        });
        wait_for_next_frame();
        wait_for_next_frame();

        let size = from_main(move || view.icon.image().size);
        ensure!(
            size != Size::new(1, 1),
            "the same image should hold the real picture after the file came, it is still {size:?}"
        );
        check_colors(LANDED)?;

        Ok(())
    }
}
