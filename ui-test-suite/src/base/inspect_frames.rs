use anyhow::{Context, Result, anyhow, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use hilen::{
    dispatch::{from_main, on_main, wait_for_next_frame},
    gm::{
        Animation, Clock, LossyConvert,
        color::{BLACK, Color},
    },
    inspect::{
        AppCommand, InspectService, InspectorCommand,
        protocol::{FrameRepr, UIRequest},
        weak_to_id,
    },
    refs::Weak,
    ui::{
        Button, Container, Label, ModifiersState, Setup, Shadow, UIAnimation, UIManager, View, ViewData,
        ViewFrame, ViewTest, view,
    },
    ui_test::{checkpoint, system_input::wait_until},
    window::Window,
};
use image::load_from_memory;

const START_X: f32 = 20.0;
// The test view is 600 wide and draws nothing outside of it.
const TRAVEL: f32 = 300.0;
const DURATION: f32 = 0.5;
const CARD_Y: f32 = 120.0;
const SIDE: f32 = 120.0;

// 0.5 s is 30 frames of the stepped clock, so 15 steps are half the way.
const HALF_FRAMES: u32 = 15;

/// A tap starts an animation: a card slides to the right, its colors go
/// once around the rainbow, a bar grows behind it and a label counts the
/// way. A screenshot some frames later shows 1 place of it by luck. The
/// record has to hold every frame from the frame of the tap on, and a
/// paused app has to move the card only when it is stepped.
#[view]
struct InspectFrames {
    arrived: bool,
    #[init]
    button:  Button,
    trail:   Container,
    card:    Container,
    percent: Label,
    status:  Label,
}

impl InspectFrames {
    /// Everything the animation draws, from the place of the card.
    fn show(mut self: Weak<Self>, x: f32) {
        let way = (x - START_X) / TRAVEL;
        let top = rainbow(way);
        self.card.set_x(x);
        self.card.set_gradient(top, rainbow(way + 0.15));
        self.trail.set_frame((START_X, CARD_Y + SIDE + 30.0, x - START_X + SIDE, 12.0));
        self.trail.set_gradient(rainbow(way + 0.5), top);
        self.percent.set_text(format!("{:.0} % of the way", way * 100.0));
    }
}

impl Setup for InspectFrames {
    fn setup(mut self: Weak<Self>) {
        self.button.set_text("Tap me").set_frame((20, 20, 200, 60));
        self.button.on_tap(move || {
            let slide = UIAnimation::new(move |_, x| self.show(x)).animation(Animation::new(
                START_X,
                START_X + TRAVEL,
                DURATION,
            ));
            slide.on_finish.sub(move || self.arrived = true);
            self.card.add_animation(slide);
        });
        self.card.set_frame((START_X, CARD_Y, SIDE, SIDE)).set_corner_radius(28);
        self.card.set_shadow(Shadow {
            offset: (0, 14).into(),
            radius: 24.0,
            color:  BLACK.with_alpha(0.55),
        });
        self.trail.set_corner_radius(6);
        self.percent.set_frame((20, CARD_Y + SIDE + 56.0, 560, 50));
        self.status.set_text("a tap slides the card for 0.5 s").set_frame((
            20,
            CARD_Y + SIDE + 110.0,
            560,
            50,
        ));
        self.show(START_X);
    }
}

/// A full color for every point of the way, red at 0 and at 1.
fn rainbow(way: f32) -> Color {
    let sixth = way.rem_euclid(1.0) * 6.0;
    let rise = sixth.fract();
    let fall = 1.0 - rise;
    match sixth {
        ..1.0 => Color::rgb(1.0, rise, 0.0),
        ..2.0 => Color::rgb(fall, 1.0, 0.0),
        ..3.0 => Color::rgb(0.0, 1.0, rise),
        ..4.0 => Color::rgb(0.0, fall, 1.0),
        ..5.0 => Color::rgb(rise, 0.0, 1.0),
        _ => Color::rgb(1.0, 0.0, fall),
    }
}

/// The card as a PNG the app sent shows it: its left edge in points and
/// the color just inside that edge. The picture is in pixels. Only the
/// card has a full color on its middle row, its shadow and the window
/// behind it are pale.
fn card(png_base64: &str) -> Result<(f32, [u8; 3])> {
    let picture = load_from_memory(&STANDARD.decode(png_base64)?)?.to_rgb8();
    let scale = from_main(UIManager::scale);
    let row: u32 = ((CARD_Y + SIDE / 2.0) * scale).lossy_convert();
    let edge = (0..picture.width())
        .find(|column| {
            let color = picture.get_pixel(*column, row).0;
            color.iter().max() > color.iter().min().map(|low| low.saturating_add(150)).as_ref()
        })
        .context("No card in the picture")?;
    let inside: u32 = (10.0 * scale).lossy_convert();
    let color = picture.get_pixel(edge + inside, row).0;
    let edge: f32 = edge.lossy_convert();
    Ok((edge / scale, color))
}

/// How far 2 colors are from each other, the sum over the 3 channels.
fn distance(first: [u8; 3], second: [u8; 3]) -> u32 {
    first.iter().zip(second).map(|(a, b)| u32::from(a.abs_diff(b))).sum()
}

/// Sends a command and picks the wanted answer. An answer can hold `Own`
/// pointers, which must drop on the main thread like the transports do.
fn send<T>(command: InspectorCommand, pick: impl FnOnce(&AppCommand) -> Option<T>) -> Result<T> {
    let response = InspectService::process_command(command);
    let result = pick(&response).ok_or_else(|| match &response {
        AppCommand::Error(error) => anyhow!("{error}"),
        other => anyhow!("Unexpected inspect response: {other:?}"),
    });
    on_main(move || drop(response));
    result
}

fn record(input: Option<UIRequest>, wait_ms: u32) -> Result<Vec<FrameRepr>> {
    let command = InspectorCommand::Record {
        frames: 8,
        input,
        wait_ms,
    };
    send(command, |response| match response {
        AppCommand::Frames { frames, .. } => Some(frames.clone()),
        _ => None,
    })
}

fn paused(command: InspectorCommand) -> Result<u64> {
    send(command, |response| match response {
        AppCommand::Paused { frame } => Some(*frame),
        _ => None,
    })
}

fn screenshot() -> Result<(f32, [u8; 3])> {
    let png = send(InspectorCommand::Screenshot, |response| match response {
        AppCommand::Screenshot { png_base64, .. } => Some(png_base64.clone()),
        _ => None,
    })?;
    card(&png).context("screenshot")
}

fn tap(view_id: &str) -> UIRequest {
    UIRequest::Tap {
        view_id:   view_id.into(),
        modifiers: ModifiersState::empty(),
        right:     false,
        force:     false,
    }
}

/// The animation runs on the real clock here. The first picture is the
/// frame of the tap, and the card moves on in every next one.
fn check_record(view: Weak<InspectFrames>, id: &str) -> Result<()> {
    {
        let frames = record(Some(tap(id)), 0)?;
        let cards = frames
            .iter()
            .map(|frame| card(&frame.png_base64).with_context(|| format!("recorded frame {}", frame.index)))
            .collect::<Result<Vec<_>>>()?;
        ensure!(cards.len() == 8, "8 frames asked, got {}", cards.len());
        ensure!(
            cards[0].0 < START_X + TRAVEL * 0.2,
            "The record must start at the frame of the tap, the card is already at {}",
            cards[0].0
        );
        ensure!(
            cards.windows(2).all(|pair| pair[1].0 >= pair[0].0) && cards[7].0 > cards[0].0,
            "The card must move on from frame to frame, got {cards:?}"
        );
        ensure!(
            distance(cards[0].1, cards[7].1) > 100,
            "The card must change its color over the record, got {cards:?}"
        );
        // The frames come at the step of a screen, also in a headless run
        // and from a covered window, where nothing else paces them. 7
        // steps of 60 a second are 117 ms.
        ensure!(
            frames[7].ms > 100.0 && cards[7].0 - cards[0].0 > TRAVEL * 0.15,
            "8 frames must cover 7 steps of a screen, the last is at {} ms, got {cards:?}",
            frames[7].ms
        );
        let indexes: Vec<u32> = frames.iter().map(|frame| frame.index).collect();
        ensure!(
            indexes == [0, 1, 2, 3, 4, 5, 6, 7],
            "Frames count from the tap, the frame of the tap is 0, got {indexes:?}"
        );
        ensure!(
            frames.windows(2).all(|pair| pair[1].ms >= pair[0].ms),
            "The time of a frame went back"
        );
        wait_until("the card arrives", move || view.arrived)?;
        checkpoint("8 frames recorded after the tap, the card is at the right end")
    }
}

impl ViewTest for InspectFrames {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let id = from_main(move || weak_to_id(view.button.weak_view()));

        // A record nobody starts ends by itself and leaves room for the
        // next one.
        let error = record(None, 100).unwrap_err().to_string();
        ensure!(error.contains("No input came"), "{error}");

        check_record(view, &id)?;

        from_main(move || {
            let mut view = view;
            view.arrived = false;
            view.show(START_X);
        });
        wait_for_next_frame();
        wait_for_next_frame();

        ensure!(paused(InspectorCommand::Pause)? == 0);
        ensure!(from_main(Clock::is_stepped));

        // The tap is taken at once, but a paused app runs no frame, so
        // the card does not move.
        let drawn = from_main(Window::render_frame);
        send(tap(&id).into(), |response| {
            matches!(response, AppCommand::UI(_)).then_some(())
        })?;
        wait_for_next_frame();
        ensure!(
            from_main(Window::render_frame) == drawn,
            "A paused app drew a frame"
        );
        // A screenshot draws again and steps nothing.
        let (start, start_color) = screenshot()?;
        ensure!((start - START_X).abs() < 1.0);
        checkpoint("paused and tapped, the card has not moved")?;

        ensure!(paused(InspectorCommand::Step { frames: 1 })? == 1);
        let (first, first_color) = screenshot()?;
        ensure!(
            (first - (START_X + TRAVEL / 30.0)).abs() < 1.5,
            "1 step is 1 frame of 30, the card is at {first}"
        );
        ensure!(
            distance(start_color, first_color) > 20,
            "1 step must change the color, {start_color:?} and {first_color:?}"
        );

        let half = u64::from(HALF_FRAMES);
        ensure!(
            paused(InspectorCommand::Step {
                frames: HALF_FRAMES - 1,
            })? == half
        );
        let (middle, middle_color) = screenshot()?;
        ensure!(
            (middle - (START_X + TRAVEL / 2.0)).abs() < 1.5,
            "15 steps are half the way, the card is at {middle}"
        );
        // Half around the rainbow from red is cyan.
        ensure!(
            middle_color[0] < 60 && middle_color[2] > 200,
            "Half the way is cyan, got {middle_color:?}"
        );
        checkpoint("paused, 15 steps after the tap, the card is half way and cyan")?;

        ensure!(
            paused(InspectorCommand::Step {
                frames: HALF_FRAMES + 1,
            })? == half * 2 + 1
        );
        ensure!(
            from_main(move || view.arrived),
            "30 steps and 1 end the animation"
        );
        ensure!(screenshot()?.0 > START_X + TRAVEL * 0.9);
        ensure!(paused(InspectorCommand::Step { frames: 0 }).is_err());

        send(InspectorCommand::Resume, |response| {
            matches!(response, AppCommand::Ok).then_some(())
        })?;
        ensure!(!from_main(Clock::is_stepped));
        ensure!(paused(InspectorCommand::Step { frames: 1 }).is_err());
        wait_for_next_frame();
        ensure!(
            from_main(Window::render_frame) > drawn,
            "A resumed app draws again"
        );
        checkpoint("resumed, the card is at the right end")?;

        Ok(())
    }
}
