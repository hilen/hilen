use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Button, Setup, ViewFrame, ViewTest, view},
    ui_test::helpers::check_colors,
};

const TEXT_SIZE: f32 = 24.0;
const PAD_X: f32 = 16.0;
const PAD_Y: f32 = 8.0;
const GAP: f32 = 12.0;
const LEFT: f32 = 20.0;
const TOP: f32 = 40.0;

/// A row of buttons each sized by its own title through
/// `Button::content_size`, the way filter tabs and toolbars are built. A
/// button without a title measures zero. The row is laid out again after
/// a title changes, so the measure follows the text.
#[view]
struct ButtonTitleSize {
    #[init]
    short:  Button,
    medium: Button,
    long:   Button,
    blank:  Button,
}

/// Sizes the button to its title plus padding at `x`, returns where the
/// next one starts.
fn fit(button: Weak<Button>, x: f32) -> f32 {
    let size = button.content_size();
    let width = (size.width + PAD_X * 2.0).ceil();
    let height = (size.height + PAD_Y * 2.0).ceil();
    button.set_frame((x, TOP, width, height));
    x + width + GAP
}

fn fit_row(view: Weak<ButtonTitleSize>) {
    let x = fit(view.short, LEFT);
    let x = fit(view.medium, x);
    fit(view.long, x);
}

impl Setup for ButtonTitleSize {
    fn setup(self: Weak<Self>) {
        for button in [self.short, self.medium, self.long, self.blank] {
            button.set_text_size(TEXT_SIZE);
        }
        self.short.set_text("OK");
        self.medium.set_text("Cancel");
        self.long.set_text("Delete everything");
        fit_row(self);

        self.blank.set_frame((LEFT, 120, 60, 40));
    }
}

fn check_fitted(button: Weak<Button>) -> Result<()> {
    let size = button.content_size();
    ensure!(
        size.width > 0.0 && size.height > 0.0,
        "{:?} measured no area for its title",
        button.text()
    );
    let expected_width = (size.width + PAD_X * 2.0).ceil();
    let expected_height = (size.height + PAD_Y * 2.0).ceil();
    ensure!(
        (button.width() - expected_width).abs() < f32::EPSILON
            && (button.height() - expected_height).abs() < f32::EPSILON,
        "{:?} frame {}x{} does not match its measured title {expected_width}x{expected_height}",
        button.text(),
        button.width(),
        button.height()
    );
    Ok(())
}

fn check_row(view: Weak<ButtonTitleSize>) -> Result<()> {
    check_fitted(view.short)?;
    check_fitted(view.medium)?;
    check_fitted(view.long)?;
    ensure!(
        view.short.max_x() < view.medium.x() && view.medium.max_x() < view.long.x(),
        "the fitted buttons overlap"
    );
    Ok(())
}

impl ViewTest for ButtonTitleSize {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            check_row(view)?;
            ensure!(
                view.short.width() < view.medium.width() && view.medium.width() < view.long.width(),
                "a longer title must measure wider, got {} {} {}",
                view.short.width(),
                view.medium.width(),
                view.long.width()
            );
            ensure!(
                view.blank.content_size().width == 0.0,
                "a button without a title must measure zero, got {:?}",
                view.blank.content_size()
            );
            Ok(())
        })?;

        check_colors(FITTED_ROW)?;

        let short_before = from_main(move || view.short.width());

        from_main(move || {
            view.short.set_text("Confirm");
            fit_row(view);
            check_row(view)
        })?;

        let short_after = from_main(move || view.short.width());
        ensure!(
            short_after > short_before,
            "a longer title must widen the button, {short_before} stayed {short_after}"
        );

        check_colors(REFITTED_ROW)?;

        Ok(())
    }
}

const FITTED_ROW: &str = r"
      56   56 - #080808
     232   56 - #242424
     260   56 - #777777
     280   56 - #000000
     364   56 - #343434
      44   60 - #ffffff
     120   60 - #ffffff
     260   60 - #777777
     388   60 - #c5c5c5
     164   64 - #ffffff
     176   64 - #070707
     248   64 - #000000
     260   64 - #777777
     292   64 - #070707
     312   64 - #070707
     344   64 - #adadad
     364   64 - #343434
     396   64 - #929292
      56   68 - #080808
     152   68 - #3a3a3a
     232   68 - #242424
     260   68 - #777777
     344   68 - #adadad
     388   68 - #c5c5c5
     396   68 - #929292
     412   68 - #cdcdcd
      60  120 - #ffffff
      20  124 - #ffffff
      76  156 - #ffffff
     368  376 - #597c95
     148  592 - #597c95
     592  592 - #597c95
";

const REFITTED_ROW: &str = r"
     136   40 - #ffffff
     236   56 - #000000
     316   56 - #5b5b5b
     424   56 - #000000
      44   60 - #ffffff
      88   60 - #474747
     172   60 - #ffffff
     196   60 - #cdcdcd
     288   60 - #999999
     396   60 - #282828
     464   60 - #000000
     104   64 - #b7b7b7
     308   64 - #070707
     336   64 - #ffffff
     360   64 - #000000
     396   64 - #282828
     416   64 - #a1a1a1
      68   68 - #010101
      88   68 - #474747
      96   68 - #d6d6d6
     104   68 - #b7b7b7
     196   68 - #cdcdcd
     204   68 - #989898
     316   68 - #5b5b5b
     396   68 - #282828
     432   68 - #010101
     452   68 - #424242
      76  128 - #ffffff
      20  156 - #ffffff
     364  376 - #597c95
     140  592 - #597c95
     592  592 - #597c95
";
