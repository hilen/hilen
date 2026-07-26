use anyhow::Result;
use parking_lot::Mutex;
use test_engine::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Setup, Theme, ThemeMode, UIManager, ViewData, ViewTest, view},
    ui_test::{capture_screenshot, check_colors, human_mode, set_record_probe_count},
};

use super::ScrollTables;

static PREVIOUS_THEME_MODE: Mutex<ThemeMode> = Mutex::new(ThemeMode::System);

const CLEAN_TEXT: &str = include_str!("text_corruption_colors.txt");

#[view]
struct TextCorruption {
    #[init]
    view: ScrollTables,
}

impl Setup for TextCorruption {
    fn setup(self: Weak<Self>) {
        self.view.place().back();
    }
}

impl ViewTest for TextCorruption {
    fn before_start() {
        *PREVIOUS_THEME_MODE.lock() = from_main(Theme::mode);
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn canvas() -> (u32, u32) {
        (640, 1000)
    }

    fn perform_test(_view: Weak<Self>) -> Result<()> {
        // Cleanup must run even when the check fails. A leaked scale
        // override or theme poisons every test after this one.
        let check = (|| {
            set_record_probe_count(1280);
            from_main(|| UIManager::override_scale(2));
            wait_for_next_frame();
            wait_for_next_frame();
            capture_screenshot()?;
            check_colors(CLEAN_TEXT)
        })();

        if !human_mode() {
            let theme_mode = *PREVIOUS_THEME_MODE.lock();
            from_main(move || {
                UIManager::override_scale(1);
                Theme::set_mode(theme_mode);
            });
        }

        check
    }
}
