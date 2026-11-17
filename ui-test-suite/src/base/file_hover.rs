use std::path::Path;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::color::{GREEN, LIGHT_GRAY},
    refs::Weak,
    ui::{Label, Setup, UIManager, ViewData, ViewFrame, ViewTest, view},
    ui_test::{checkpoint, inject_file_drop, inject_file_hover, inject_file_hover_end},
};

const NOTHING_HELD: &str = "drop zone: nothing held";

/// An app shows a drop mark while a file is held over its window. Proves
/// that the hold reaches the app with the path, that it ends when the file
/// leaves, and that a drop ends the hold before the drop itself arrives,
/// so the mark is hidden in 1 place.
#[view]
struct FileHover {
    events: String,

    #[init]
    title: Label,
    zone:  Label,
    order: Label,
}

fn name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().to_string())
}

impl FileHover {
    fn note(mut self: Weak<Self>, event: &str) {
        if !self.events.is_empty() {
            self.events.push(' ');
        }
        self.events.push_str(event);
        self.order.set_text(format!("events: {}", self.events));
    }
}

impl Setup for FileHover {
    fn setup(self: Weak<Self>) {
        self.title.set_frame((20, 40, 560, 40));
        self.title.set_text("a file held over the window");

        self.zone.set_frame((20, 100, 560, 200));
        self.zone.set_text(NOTHING_HELD);
        self.zone.set_color(LIGHT_GRAY);

        self.order.set_frame((20, 320, 560, 40));
        self.order.set_text("events: none");

        UIManager::on_hover_file(self, move |path| {
            self.zone.set_text(format!("drop zone: holds {}", name(&path)));
            self.zone.set_color(GREEN);
            self.note("hover");
        });
        UIManager::on_hover_file_end(self, move || {
            self.zone.set_text(NOTHING_HELD);
            self.zone.set_color(LIGHT_GRAY);
            self.note("end");
        });
        UIManager::on_drop_file(self, move |path| {
            self.zone.set_text(format!("drop zone: got {}", name(&path)));
            self.note("drop");
        });
    }
}

impl ViewTest for FileHover {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let zone = move || from_main(move || view.zone.text.clone());
        let events = move || from_main(move || view.events.clone());

        inject_file_hover("/films/Dune");
        ensure!(
            zone() == "drop zone: holds Dune",
            "the hold arrived, got {:?}",
            zone()
        );
        checkpoint("the zone is green and holds Dune")?;

        inject_file_hover_end();
        ensure!(zone() == NOTHING_HELD, "the hold ended, got {:?}", zone());
        checkpoint("the zone is gray again, nothing held")?;

        inject_file_hover("/films/Dune");
        inject_file_drop("/films/Dune");
        ensure!(
            zone() == "drop zone: got Dune",
            "the drop arrived, got {:?}",
            zone()
        );
        ensure!(
            events() == "hover end hover end drop",
            "a drop ends the hold first, got {:?}",
            events()
        );
        checkpoint("the zone is gray and got Dune, the hold ended before the drop")?;
        Ok(())
    }
}
