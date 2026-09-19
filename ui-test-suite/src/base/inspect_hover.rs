use anyhow::{Result, anyhow};
use hilen::{
    dispatch::{from_main, on_main},
    inspect::{
        AppCommand, InspectService,
        protocol::{UIRequest, UIResponse},
        weak_to_id,
    },
    refs::Weak,
    ui::{
        Button, Container, CursorIcon, Hover, Setup, Tooltip, UIManager, View, ViewData, ViewFrame, ViewTest,
        ViewTooltip, ViewTouch, view,
    },
    ui_test::checkpoint,
};

#[view]
struct InspectHover {
    enters:    u32,
    exits:     u32,
    taps:      u32,
    #[init]
    button:    Button,
    cover:     Button,
    hidden:    Button,
    offscreen: Button,
    plain:     Container,
}

impl Setup for InspectHover {
    fn setup(mut self: Weak<Self>) {
        self.button.set_text("Hover me").set_frame((20, 20, 180, 60));
        self.button.set_hover_cursor(CursorIcon::Pointer);
        self.button.set_tooltip("Inspector tooltip");
        self.button.touch().hovered.val(self, move |hovered| {
            if hovered {
                self.enters += 1;
            } else {
                self.exits += 1;
            }
        });
        self.button.on_tap(move || self.taps += 1);
        self.cover.set_text("Cover").set_frame((20, 20, 180, 60));
        self.cover.enable_hover().set_hidden(true);
        self.hidden.set_frame((20, 120, 180, 60)).set_hidden(true);
        self.offscreen.set_frame((-200, -200, 100, 60));
        self.plain.set_frame((240, 20, 100, 60));
    }
}

fn hover(view_id: Option<String>, wait_ms: u32) -> Result<String> {
    let response = InspectService::process_command(UIRequest::Hover { view_id, wait_ms }.into());
    let result = match &response {
        AppCommand::UI(UIResponse::SendUI { note, .. }) => Ok(note.clone().unwrap_or_default()),
        AppCommand::Error(error) => Err(anyhow!("{error}")),
        other => Err(anyhow!("Unexpected inspect response: {other:?}")),
    };
    on_main(move || drop(response));
    result
}

impl ViewTest for InspectHover {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        hover(None, 0)?;
        from_main(move || {
            let mut view = view;
            view.enters = 0;
            view.exits = 0;
        });
        let id = from_main(move || weak_to_id(view.button.weak_view()));
        let counts = move || from_main(move || (view.enters, view.exits, view.taps));

        assert!(hover(Some(id.clone()), 0)?.contains(&id));
        assert_eq!(counts(), (1, 0, 0));
        assert!(from_main(move || view.button.is_hovered()));
        assert_eq!(from_main(Hover::cursor), CursorIcon::Pointer);
        assert!(from_main(|| Tooltip::shown().is_null()));

        // Staying over the same view does not fire another enter. Waiting
        // returns only after the real tooltip timer can show its view.
        assert!(hover(Some(id.clone()), 600)?.contains(&id));
        assert_eq!(counts(), (1, 0, 0));
        assert!(from_main(|| Tooltip::shown().is_ok()));
        checkpoint("Hover and tooltip without a click")?;

        assert_eq!(hover(None, 0)?, "hovered none");
        assert_eq!(counts(), (1, 1, 0));
        assert!(!from_main(move || view.button.is_hovered()));
        assert!(from_main(|| Tooltip::shown().is_null()));
        assert_eq!(from_main(Hover::cursor), CursorIcon::Default);
        checkpoint("Pointer cleared and tooltip hidden")?;

        // The reply names the actual topmost hover receiver, even when
        // the caller targeted the covered button.
        from_main(move || {
            view.cover.set_hidden(false);
        });
        let cover_id = from_main(move || weak_to_id(view.cover.weak_view()));
        assert!(hover(Some(id.clone()), 0)?.contains(&cover_id));
        assert!(!from_main(move || view.button.is_hovered()));
        assert!(from_main(move || view.cover.is_hovered()));
        hover(None, 0)?;
        from_main(move || {
            view.cover.set_hidden(true);
        });

        let hidden = from_main(move || weak_to_id(view.hidden.weak_view()));
        assert!(hover(Some(hidden), 0).unwrap_err().to_string().contains("hidden"));
        let offscreen = from_main(move || weak_to_id(view.offscreen.weak_view()));
        assert!(hover(Some(offscreen), 0).unwrap_err().to_string().contains("outside"));
        assert!(hover(Some("missing-view".into()), 0).is_err());
        assert!(hover(Some(id.clone()), 60_001).is_err());
        let plain = from_main(move || weak_to_id(view.plain.weak_view()));
        assert_eq!(hover(Some(plain), 0)?, "hovered none");

        from_main(|| UIManager::set_scale(2));
        assert!(hover(Some(id), 0)?.contains("button"));
        assert!(from_main(move || view.button.is_hovered()));
        assert_eq!(
            from_main(UIManager::cursor_position),
            from_main(move || view.button.absolute_frame().center())
        );
        hover(None, 600)?;
        assert!(from_main(|| Tooltip::shown().is_null()));
        from_main(|| UIManager::set_scale(1));
        assert_eq!(from_main(move || view.taps), 0);
        Ok(())
    }
}
