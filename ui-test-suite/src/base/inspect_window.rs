use anyhow::{Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use hilen::{
    dispatch::{from_main, on_main, wait_for_next_frame},
    gm::LossyConvert,
    inspect::{
        AppCommand, InspectService, InspectorCommand, ViewRepr,
        protocol::{Key, UIRequest, UIResponse, WindowRepr},
        weak_to_id,
    },
    refs::{Own, Weak},
    ui::{
        Button, Color, Label, Modifiers, Setup, UIManager, View, ViewData, ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::checkpoint,
    window::{AppWindow, WindowLevel, WindowPlace, WindowSpec},
};

const TITLE: &str = "Inspect window";
const SECOND: Color = Color::rgb(0.9, 0.5, 0.1);

/// The content of the second window, a button and a key binding that count.
#[view]
struct InspectWindowContent {
    taps: u32,
    keys: u32,

    #[init]
    button: Button,
    title:  Label,
}

impl Setup for InspectWindowContent {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.title.set_text("second window").set_text_size(24);
        self.title.set_frame((20, 0, 360, 40));

        self.button.set_text("second").set_text_size(24);
        self.button.set_color(SECOND);
        self.button.set_frame((50, 60, 200, 100));
        self.button.on_tap(move || self.taps += 1);
        UIManager::keymap().add(self, 'k', move || self.keys += 1);
    }
}

/// A button in the main window and a second window with its own button.
/// `Windows` lists both, and `InWindow` with the number of the second one
/// reads its tree, taps its button, types into it and takes its picture,
/// while the main window and its button take nothing. A command that is not
/// about views, a resize, is refused there. Before `InWindow` the inspector
/// saw only the main window.
#[view]
struct InspectWindow {
    taps:    u32,
    keys:    u32,
    content: Weak<InspectWindowContent>,

    #[init]
    button: Button,
}

impl Setup for InspectWindow {
    fn setup(mut self: Weak<Self>) {
        self.button.set_text("main").set_text_size(24);
        self.button.set_frame((50, 60, 200, 100));
        self.button.on_tap(move || self.taps += 1);
        UIManager::keymap().add(self, 'k', move || self.keys += 1);
    }
}

impl InspectWindow {
    /// The taps and keys of the main window and of the second one.
    fn counts(self: Weak<Self>) -> [u32; 4] {
        from_main(move || [self.taps, self.keys, self.content.taps, self.content.keys])
    }
}

fn open(mut view: Weak<InspectWindow>) -> Result<AppWindow> {
    from_main(move || {
        AppWindow::open(
            WindowSpec {
                title:       TITLE.into(),
                place:       WindowPlace::Frame((0, 0, 400, 300).into()),
                level:       WindowLevel::Normal,
                decorations: true,
            },
            move || {
                let content = InspectWindowContent::new();
                view.content = content.weak();
                content
            },
        )
    })
}

/// The answer to `command`, run in `window` when there is one. The answer
/// can hold `Own` pointers, which drop on the main thread like the
/// transports do.
fn ask<T: Send + 'static>(
    window: Option<usize>,
    command: InspectorCommand,
    read: impl FnOnce(&AppCommand) -> T,
) -> T {
    let command = match window {
        Some(window) => InspectorCommand::InWindow {
            window,
            command: Box::new(command),
        },
        None => command,
    };
    let answer = InspectService::process_command(command);
    let value = read(&answer);
    on_main(move || drop(answer));
    value
}

fn error_of(answer: &AppCommand) -> Option<String> {
    match answer {
        AppCommand::Error(error) => Some(error.clone()),
        _ => None,
    }
}

fn has_view(view: &ViewRepr, id: &str) -> bool {
    view.id == id || view.subviews.iter().any(|sub: &Own<ViewRepr>| has_view(sub, id))
}

/// Whether the tree of `window` holds the view `id`.
fn tree_has(window: Option<usize>, id: &str) -> Result<bool> {
    let id = id.to_string();
    ask(window, UIRequest::GetUI.into(), move |answer| match answer {
        AppCommand::UI(UIResponse::SendUI { root, .. }) => Ok(has_view(root, &id)),
        other => bail!("unexpected answer to the tree: {other:?}"),
    })
}

fn tap(window: Option<usize>, id: &str) -> Option<String> {
    let request = UIRequest::Tap {
        view_id:   id.to_string(),
        modifiers: Modifiers::empty(),
        right:     false,
        force:     false,
    };
    ask(window, request.into(), error_of)
}

fn second_window(windows: &[WindowRepr]) -> Result<usize> {
    let Some(second) = windows.iter().find(|window| window.title == TITLE) else {
        bail!("the windows list has no {TITLE}: {windows:?}");
    };
    ensure!(
        windows.first().is_some_and(|main| main.number == 0),
        "the main window comes first as 0: {windows:?}"
    );
    ensure!(
        (second.width, second.height) == (400.0, 300.0),
        "the second window is 400 by 300 points: {second:?}"
    );
    Ok(second.number)
}

fn check_screenshot(number: usize, window: AppWindow) -> Result<()> {
    let (width, height, png) = ask(
        Some(number),
        InspectorCommand::Screenshot,
        |answer| match answer {
            AppCommand::Screenshot {
                width,
                height,
                png_base64,
            } => Ok((*width, *height, png_base64.clone())),
            other => bail!("unexpected answer to the screenshot: {other:?}"),
        },
    )?;
    let scale = from_main(move || window.within(UIManager::scale))?;
    let picture = image::load_from_memory(&STANDARD.decode(png)?)?.to_rgba8();
    ensure!(
        (picture.width(), picture.height()) == (width, height),
        "the png is {}x{}, the answer says {width}x{height}",
        picture.width(),
        picture.height()
    );
    let at = |x: f32, y: f32| -> [u8; 3] {
        let pixel = picture.get_pixel((x * scale).lossy_convert(), (y * scale).lossy_convert());
        [pixel[0], pixel[1], pixel[2]]
    };
    let corner = at(55.0, 65.0);
    let background = at(350.0, 250.0);
    ensure!(
        background == [255, 255, 255],
        "the picture is not of the second window, its background is {background:?}"
    );
    ensure!(
        corner != background,
        "the button of the second window is not in its picture, {corner:?}"
    );
    Ok(())
}

impl ViewTest for InspectWindow {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let window = open(view)?;
        while !from_main(move || window.is_ready()) {
            wait_for_next_frame();
        }
        checkpoint("a second window with its own button")?;

        let windows = ask(None, InspectorCommand::Windows, |answer| match answer {
            AppCommand::Windows(windows) => Ok(windows.clone()),
            other => bail!("unexpected answer to windows: {other:?}"),
        })?;
        let number = second_window(&windows)?;

        let (main_button, second_button) = from_main(move || {
            (
                weak_to_id(view.button.weak_view()),
                weak_to_id(view.content.button.weak_view()),
            )
        });

        ensure!(
            tree_has(Some(number), &second_button)? && !tree_has(Some(number), &main_button)?,
            "the tree of window {number} holds its own button only"
        );
        ensure!(
            tree_has(None, &main_button)? && !tree_has(None, &second_button)?,
            "the tree with no window is the main one"
        );

        if let Some(error) = tap(Some(number), &second_button) {
            bail!("a tap on the button of window {number} was refused: {error}");
        }
        ensure!(
            view.counts() == [0, 0, 1, 0],
            "the tap reaches the second window only, counts {:?}",
            view.counts()
        );
        ensure!(
            tap(Some(number), &main_button).is_some(),
            "a view of the main window is not found in window {number}"
        );

        let keys = UIRequest::Keys {
            keys:      vec![Key::Char('k')],
            modifiers: Modifiers::empty(),
        };
        if let Some(error) = ask(Some(number), keys.into(), error_of) {
            bail!("keys in window {number} failed: {error}");
        }
        ensure!(
            view.counts() == [0, 0, 1, 1],
            "a key reaches the binding of the second window only, counts {:?}",
            view.counts()
        );

        check_screenshot(number, window)?;

        let resize = UIRequest::Resize {
            width:  500.0,
            height: 400.0,
        };
        ensure!(
            ask(Some(number), resize.into(), error_of).is_some(),
            "a resize is for the main window only"
        );

        from_main(move || window.close());
        wait_for_next_frame();
        let error = ask(Some(number), UIRequest::GetUI.into(), error_of);
        ensure!(
            error.is_some_and(|error| error.contains("No window")),
            "a closed window answers with an error"
        );
        Ok(())
    }
}
