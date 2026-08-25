use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::{Weak, manage::DataManager},
    ui::{Font, Label, Setup, ViewFrame, ViewTest, view},
};

#[view]
struct LabelMeasure {
    #[init]
    label: Label,
}

impl Setup for LabelMeasure {
    fn setup(self: Weak<Self>) {
        self.label.set_frame((20, 20, 400, 80));
        self.label.set_text("Grumpy wizards").set_text_size(40);
    }
}

impl ViewTest for LabelMeasure {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let short = from_main(move || view.label.content_size());

        assert!(
            short.width > 0.0 && short.height > 0.0,
            "text measured as empty: {short:?}"
        );
        assert!(
            short.width < 400.0,
            "short text measured wider than its frame: {short:?}"
        );

        let long = from_main(move || {
            view.label.set_text("Grumpy wizards make toxic brew");
            view.label.content_size()
        });

        assert!(
            long.width > short.width,
            "longer text did not measure wider: {long:?} vs {short:?}"
        );

        let big = from_main(move || {
            view.label.set_text_size(80);
            view.label.content_size()
        });

        let ratio = big.width / long.width;
        assert!(
            (1.8..2.2).contains(&ratio),
            "doubled text size did not double the width: {ratio}"
        );

        let mono = from_main(move || {
            view.label.set_text_size(40);
            view.label.set_font(Font::get("DroidSansMono.ttf"));
            view.label.content_size()
        });

        assert!(
            (mono.width - long.width).abs() > f32::EPSILON,
            "different font measured the same width: {mono:?}"
        );

        let (narrow, wide) = from_main(move || {
            view.label.set_font(Font::roboto());
            view.label.set_multiline(true);
            (view.label.size_for_width(200.0), view.label.size_for_width(500.0))
        });

        assert!(
            narrow.height > wide.height,
            "narrower bound did not wrap to more lines: {narrow:?} vs {wide:?}"
        );
        assert!(
            narrow.width <= 200.0,
            "wrapped text exceeded the width bound: {narrow:?}"
        );

        let empty = from_main(move || {
            view.label.set_text("");
            view.label.content_size()
        });

        assert!(
            empty.width == 0.0 && empty.height == 0.0,
            "empty text measured a size: {empty:?}"
        );

        Ok(())
    }
}
