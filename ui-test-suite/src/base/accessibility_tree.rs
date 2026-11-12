use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::{Own, Weak},
    ui::{
        Accessibility, AccessibleRole, Button, CheckBox, Container, Label, Setup, Slider, Switch, TextField,
        ToolButton, ViewAccessibility, ViewData, ViewTest, ViewTouch, view,
    },
    ui_test::checkpoint,
};

/// The views a screen reader gets, in reading order with a role and a
/// label, and a press on a node taps its view. A hidden view is left out. In
/// a browser the nodes are DOM elements of a hidden layer over the canvas.
#[view]
struct AccessibilityTree {
    taps: Own<Vec<&'static str>>,

    #[init]
    title:  Label,
    name:   TextField,
    save:   Button,
    close:  ToolButton,
    sound:  CheckBox,
    lights: Switch,
    level:  Slider,
    ghost:  Button,
    card:   Container,
}

impl Setup for AccessibilityTree {
    fn setup(self: Weak<Self>) {
        self.title.set_text("Settings").place().t(10).l(10).size(300, 40);
        self.name.set_placeholder("Name").place().t(60).l(10).size(300, 40);
        self.save.set_text("Save").place().t(110).l(10).size(140, 40);
        self.close.set_icon("bin.svg").set_accessibility_label("Delete");
        self.close.place().t(110).l(170);
        self.sound.set_accessibility_label("Sound on new alerts");
        self.sound.place().t(160).l(10).size(30, 30);
        self.lights.set_accessibility_label("Grow lights");
        self.lights.place().t(200).l(10).size(60, 32);
        self.level
            .set_accessibility_label("Light level")
            .set_horizontal()
            .set_range(0, 100)
            .set_value(40);
        self.level.place().t(250).l(10).size(300, 32);
        self.ghost.set_text("Ghost").place().t(300).l(10).size(140, 40);
        self.ghost.set_hidden(true);
        self.card.set_color("#DDEEFF").set_accessibility_label("Basil, zone 3");
        self.card.place().t(350).l(10).size(300, 60);
        self.card.enable_touch();

        let mut taps = self.taps.weak();
        self.save.on_tap(move || taps.push("save"));
        self.card.touch().up_inside.sub(self, move || taps.push("card"));
    }
}

impl ViewTest for AccessibilityTree {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        settle();
        let nodes = from_main(Accessibility::nodes);
        let seen: Vec<(AccessibleRole, String)> =
            nodes.iter().map(|node| (node.role, node.label.clone())).collect();
        let expected = vec![
            (AccessibleRole::Text, "Settings".to_string()),
            (AccessibleRole::TextBox, "Name".to_string()),
            (AccessibleRole::Button, "Save".to_string()),
            (AccessibleRole::Button, "Delete".to_string()),
            (AccessibleRole::CheckBox, "Sound on new alerts".to_string()),
            (AccessibleRole::Switch, "Grow lights".to_string()),
            (AccessibleRole::Slider, "Light level".to_string()),
            (AccessibleRole::Button, "Basil, zone 3".to_string()),
        ];
        ensure!(seen == expected, "nodes {seen:#?}");
        ensure!(
            nodes[6].value.as_deref() == Some("40"),
            "slider value {:?}",
            nodes[6].value
        );
        ensure!(
            nodes[4].checked == Some(false),
            "check box state {:?}",
            nodes[4].checked
        );
        checkpoint("8 nodes in reading order, the hidden Ghost button is left out")?;

        from_main(move || {
            let nodes = Accessibility::nodes();
            nodes[2].activate();
            nodes[7].activate();
            nodes[4].activate();
            Accessibility::announce("New alert in zone 3");
        });
        settle();
        from_main(move || {
            ensure!(*view.taps == ["save", "card"], "taps {:?}", *view.taps);
            ensure!(view.sound.on(), "the press turned the check box on");
            ensure!(Accessibility::nodes()[4].checked == Some(true));
            Ok(())
        })?;
        checkpoint("pressing the nodes tapped Save, the card and the check box")?;

        #[cfg(target_arch = "wasm32")]
        {
            for _ in 0..30 {
                wait_for_next_frame();
            }
            let count = from_main(|| {
                web_sys::window()
                    .and_then(|window| window.document())
                    .and_then(|document| document.get_element_by_id("hilen-accessibility"))
                    .map_or(0, |layer| layer.child_element_count())
            });
            ensure!(count == 8, "the DOM layer has {count} elements");
        }
        Ok(())
    }
}

fn settle() {
    for _ in 0..3 {
        wait_for_next_frame();
    }
}
