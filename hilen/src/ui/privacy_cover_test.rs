use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak, vents::OnceEvent},
    gm::flat::Size,
    system::app_activity::{self, ActivityChange},
    ui::{Color, Label, ModalView, Setup, UIColor, UIManager, ViewData, ViewTest, privacy_cover, view},
    ui_test::{check_colors, checkpoint},
};

const CLEAR_COLOR: Color = Color::rgb(0.1, 0.35, 0.3);
const PAGE: Color = Color::rgb(0.95, 0.85, 0.6);
const PANEL: Color = Color::rgb(0.85, 0.3, 0.3);
const MODAL: Color = Color::rgb(0.3, 0.45, 0.85);

const ACTIVE: &str = r"
 276   48 - #b5a373
 292   56 - #b5a373
 324   56 - #b5a373
  20  128 - #a33a3a
 216  132 - #170808
 384  132 - #7d2c2c
 324  136 - #792b2b
 304  140 - #8b3232
 380  140 - #a33a3a
 236  144 - #a33a3a
 256  144 - #a33a3a
 276  144 - #441818
 352  148 - #000000
 576  176 - #a33a3a
 120  224 - #4d73d9
 260  292 - #21325e
 324  292 - #0b111f
 260  296 - #21325e
 256  300 - #10172c
 260  300 - #21325e
 284  300 - #4d73d9
 296  300 - #365198
 304  300 - #000000
 348  300 - #365198
 216  304 - #16203d
 224  304 - #4d73d9
 260  304 - #21325e
 396  304 - #000000
 476  376 - #4d73d9
   4  512 - #b5a373
 304  552 - #302b1e
 592  592 - #b5a373
";

const DESKTOP_FOCUS_LOST: &str = r"
 312   48 - #000000
 272   60 - #010101
 576  100 - #a33a3a
 384  132 - #7d2c2c
 324  136 - #792b2b
 220  140 - #a33a3a
 304  140 - #8b3232
 360  140 - #a33a3a
 276  144 - #441818
  20  176 - #a33a3a
 476  240 - #4d73d9
 260  292 - #21325e
 260  296 - #21325e
 256  300 - #10172c
 260  300 - #21325e
 284  300 - #4d73d9
 296  300 - #365198
 304  300 - #000000
 348  300 - #365198
 216  304 - #16203d
 224  304 - #4d73d9
 260  304 - #21325e
 396  304 - #000000
 120  376 - #4d73d9
 592  420 - #b5a373
   4  432 - #b5a373
 352  544 - #716547
 332  548 - #75694a
 352  548 - #716547
 220  552 - #b5a373
 280  552 - #b5a373
 380  552 - #000000
";

const COVERED: &str = r"
   4    4 - #1a594d
 444    4 - #1a594d
 592    4 - #1a594d
 296    8 - #1a594d
 148   12 - #1a594d
 228   84 - #1a594d
  12  148 - #1a594d
 444  152 - #1a594d
 592  152 - #1a594d
 156  156 - #1a594d
 300  156 - #1a594d
  84  228 - #1a594d
 228  228 - #1a594d
 372  228 - #1a594d
   8  296 - #1a594d
 448  296 - #1a594d
 156  300 - #1a594d
 300  300 - #1a594d
 592  300 - #1a594d
 228  372 - #1a594d
 372  372 - #1a594d
 516  372 - #1a594d
   4  444 - #1a594d
 152  444 - #1a594d
 444  444 - #1a594d
 296  448 - #1a594d
 588  448 - #1a594d
 448  588 - #1a594d
   4  592 - #1a594d
 152  592 - #1a594d
 300  592 - #1a594d
 592  592 - #1a594d
";

const ACTIVE_AGAIN: &str = r"
 312   48 - #000000
 272   60 - #010101
 576  100 - #a33a3a
 384  132 - #7d2c2c
 324  136 - #792b2b
 220  140 - #a33a3a
 252  140 - #a33a3a
 304  140 - #8b3232
 360  140 - #a33a3a
 276  144 - #441818
  20  176 - #a33a3a
 476  220 - #4d73d9
 260  292 - #21325e
 260  296 - #21325e
 256  300 - #10172c
 260  300 - #21325e
 296  300 - #365198
 304  300 - #000000
 348  300 - #365198
 216  304 - #16203d
 224  304 - #4d73d9
 260  304 - #21325e
 320  304 - #4d73d9
 380  304 - #4d73d9
 476  376 - #4d73d9
   4  432 - #b5a373
 256  548 - #010101
 376  548 - #000000
 220  552 - #b5a373
 296  552 - #b5a373
 328  552 - #b19f70
 592  592 - #b5a373
";

const COVER_OFF: &str = r"
 592    4 - #b5a373
 312   48 - #000000
 272   60 - #010101
 384  132 - #7d2c2c
 324  136 - #792b2b
 576  136 - #a33a3a
 220  140 - #a33a3a
 304  140 - #8b3232
 360  140 - #a33a3a
 288  144 - #a33a3a
 256  148 - #000000
  20  176 - #a33a3a
 476  220 - #4d73d9
 260  292 - #21325e
 260  296 - #21325e
 256  300 - #10172c
 260  300 - #21325e
 296  300 - #365198
 304  300 - #000000
 348  300 - #365198
 216  304 - #16203d
 224  304 - #4d73d9
 260  304 - #21325e
 380  304 - #4d73d9
 476  364 - #4d73d9
 592  448 - #b5a373
 264  544 - #010101
 192  552 - #b5a373
 232  552 - #211d15
 296  552 - #b5a373
 328  552 - #201c14
 384  552 - #b5a373
";

/// A page with a balance on it and a modal over it, the 2 layers the cover
/// has to hide.
#[view]
struct PrivacyCover {
    #[init]
    title:   Label,
    balance: Label,
    status:  Label,
}

impl Setup for PrivacyCover {
    fn setup(self: Weak<Self>) {
        UIManager::set_clear_color(CLEAR_COLOR);
        self.set_color(PAGE);
        self.title.set_text("page").set_text_size(28);
        self.title.place().lrt(20).h(60);
        self.balance.set_text("balance 1.234").set_text_size(28).set_color(PANEL);
        self.balance.place().lr(20).t(100).h(80);
        self.status.set_text_size(20);
        self.status.place().lrb(20).h(60);
    }
}

#[view]
struct SecretModal {
    event: OnceEvent<()>,

    #[init]
    text: Label,
}

impl ModalView for SecretModal {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (360, 160).into()
    }

    fn modal_scrim_color() -> UIColor {
        Color::rgba(0.0, 0.0, 0.0, 0.25).into()
    }
}

impl Setup for SecretModal {
    fn setup(self: Weak<Self>) {
        self.set_color(MODAL);
        self.text.set_text("modal: send review").set_text_size(24);
        self.text.place().back();
    }
}

impl ViewTest for PrivacyCover {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let restore = Restore {
            enabled: from_main(privacy_cover::enabled),
        };
        let modal = from_main(move || {
            privacy_cover::set_enabled(true);
            privacy_cover::set_test_phone(Some(false));
            view.status.set_text("active");
            SecretModal::prepare_modally()
        });
        checkpoint("active: the page and the modal show")?;
        check_colors(ACTIVE)?;

        status(view, "desktop, focus lost", ActivityChange::Focused(false));
        ensure!(
            !from_main(privacy_cover::shown),
            "a desktop focus loss showed the cover"
        );
        checkpoint("a desktop window that lost the focus is not covered")?;
        check_colors(DESKTOP_FOCUS_LOST)?;
        from_main(|| app_activity::update(ActivityChange::Focused(true)));

        from_main(|| privacy_cover::set_test_phone(Some(true)));
        status(view, "phone, resigned active", ActivityChange::Resumed(false));
        ensure!(
            from_main(privacy_cover::shown),
            "a phone that resigned active showed no cover"
        );
        checkpoint("a phone that resigned active: only the clear color")?;
        check_colors(COVERED)?;

        status(view, "phone, active again", ActivityChange::Resumed(true));
        ensure!(
            !from_main(privacy_cover::shown),
            "the cover stayed on an active app"
        );
        checkpoint("active again: the page and the modal are back")?;
        check_colors(ACTIVE_AGAIN)?;

        from_main(|| privacy_cover::set_enabled(false));
        status(view, "cover off, resigned active", ActivityChange::Resumed(false));
        ensure!(!from_main(privacy_cover::shown), "a cover that is off was shown");
        checkpoint("an app with no cover is seen when it resigns active")?;
        check_colors(COVER_OFF)?;

        from_main(move || modal.hide_modal(()));
        drop(restore);
        Ok(())
    }
}

fn status(view: Weak<PrivacyCover>, text: &'static str, change: ActivityChange) {
    from_main(move || {
        view.status.set_text(text);
        app_activity::update(change);
    });
}

/// Puts the activity, the platform rule and the switch of the app back,
/// also when a check fails.
struct Restore {
    enabled: bool,
}

impl Drop for Restore {
    fn drop(&mut self) {
        let enabled = self.enabled;
        from_main(move || {
            app_activity::update(ActivityChange::Focused(true));
            app_activity::update(ActivityChange::Visible(true));
            app_activity::update(ActivityChange::Resumed(true));
            privacy_cover::set_test_phone(None);
            privacy_cover::set_enabled(enabled);
        });
    }
}
