use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, on_main},
    inspect::ViewToInspect,
    refs::Weak,
    ui::{ChipPick, Chips, Color, Setup, ToolButton, ViewData, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
};

/// A tool button with an icon and a text, and a row of chips, which are
/// tool buttons too. The inspector tree gives each of them its text. Before
/// this the text sat only in a system label the tree leaves out, so
/// `hilen-inspect find` and `tap` could not reach a sidebar entry or a chip
/// by what it says.
#[view]
struct InspectToolButton {
    #[init]
    tool:  ToolButton,
    chips: Chips,
}

impl Setup for InspectToolButton {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::hex("#FFFFFF"));
        self.tool.set_text("Bed layout").set_icon("plus.svg");
        self.tool.set_frame((40, 40, 200, 40));
        self.chips.set_pick(ChipPick::Many);
        self.chips.set_texts(["Water", "Light"]);
        self.chips.place().t(120).l(40).w(300);
    }
}

impl ViewTest for InspectToolButton {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let (tool, chip, chips_tree) = from_main(move || {
            let tool = view.tool.view_to_inspect();
            let chip = view.chips.chip(1).map(|c| c.view_to_inspect());
            let chips = view.chips.view_to_inspect();
            let texts = (
                tool.text.clone(),
                chip.as_ref().and_then(|c| c.text.clone()),
                chips.subviews.iter().filter_map(|s| s.text.clone()).collect::<Vec<_>>(),
            );
            on_main(move || drop((tool, chip, chips)));
            texts
        });
        ensure!(
            tool.as_deref() == Some("Bed layout"),
            "the tool button gave {tool:?}"
        );
        ensure!(chip.as_deref() == Some("Light"), "the second chip gave {chip:?}");
        ensure!(
            chips_tree.contains(&"Water".to_string()),
            "the chips in the tree gave {chips_tree:?}"
        );
        checkpoint("a tool button with an icon and 2 chips, each found by its text")?;
        Ok(())
    }
}
