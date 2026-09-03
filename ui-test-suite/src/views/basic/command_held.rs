use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLUE, Container, ModifiersState, Setup, UIManager, ViewData, ViewTest, ViewTouch, view},
    ui_test::{inject_modifiers, inject_touches},
};

/// A tap handler asks `UIManager::command_held` to tell a plain click from
/// a command click, the multi select gesture. The touch itself carries no
/// modifier state, so this query is the only way an app can see it.
#[view]
struct CommandHeld {
    plain:   usize,
    command: usize,

    #[init]
    target: Container,
}

impl Setup for CommandHeld {
    fn setup(self: Weak<Self>) {
        self.target.set_color(BLUE);
        self.target.place().tl(100).size(200, 200);
        self.target.enable_touch();
        self.target.touch().up_inside.sub(self, move || {
            let mut this = self;
            if UIManager::command_held() {
                this.command += 1;
            } else {
                this.plain += 1;
            }
        });
    }
}

impl ViewTest for CommandHeld {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("200 200 b\n200 200 e");
        from_main(move || {
            assert_eq!(view.plain, 1);
            assert_eq!(view.command, 0);
        });

        inject_modifiers(ModifiersState::SUPER);
        inject_touches("200 200 b\n200 200 e");
        from_main(move || {
            assert_eq!(view.plain, 1);
            assert_eq!(view.command, 1);
        });

        // Ctrl counts as the command key too, the Windows and Linux side.
        inject_modifiers(ModifiersState::CONTROL);
        inject_touches("200 200 b\n200 200 e");
        from_main(move || {
            assert_eq!(view.plain, 1);
            assert_eq!(view.command, 2);
        });

        inject_modifiers(ModifiersState::empty());
        inject_touches("200 200 b\n200 200 e");
        from_main(move || {
            assert_eq!(view.plain, 2);
            assert_eq!(view.command, 2);
        });

        Ok(())
    }
}
