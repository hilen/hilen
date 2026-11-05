use anyhow::{Context, Result, anyhow, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use hilen::{
    dispatch::{from_main, on_main},
    gm::{
        Animation, LossyConvert,
        color::{BLACK, Color},
    },
    inspect::{
        AppCommand, InspectService, InspectorCommand, ViewRepr,
        protocol::{KeptFrameRepr, LayoutRepr, RecordRepr, RenderInfoRepr},
    },
    refs::Weak,
    ui::{Container, Label, Setup, Shadow, UIAnimation, UIManager, ViewData, ViewFrame, ViewTest, view},
    ui_test::{checkpoint, system_input::wait_until},
};
use image::{ImageFormat, load_from_memory_with_format};
use log::info;

const START_X: f32 = 20.0;
// The test view is 600 wide and draws nothing outside of it.
const TRAVEL: f32 = 400.0;
const DURATION: f32 = 2.0;
const CARD_Y: f32 = 120.0;
const SIDE: f32 = 120.0;

/// A card slides for 2 seconds while a record from a start to a stop
/// runs. The record has to hold every frame the app drew, in order, with
/// none lost, at 1 pixel per point, and the card has to move through it.
#[view]
struct InspectRecord {
    arrived: bool,
    #[init]
    card:    Container,
    status:  Label,
}

impl InspectRecord {
    fn slide(mut self: Weak<Self>) {
        self.arrived = false;
        let slide = UIAnimation::new(move |_, x| {
            self.card.set_x(x);
            self.card.set_color(Color::rgb(1.0, (x - START_X) / TRAVEL, 0.0));
        })
        .animation(Animation::new(START_X, START_X + TRAVEL, DURATION));
        slide.on_finish.sub(move || self.arrived = true);
        self.card.add_animation(slide);
    }
}

impl Setup for InspectRecord {
    fn setup(self: Weak<Self>) {
        self.card.set_frame((START_X, CARD_Y, SIDE, SIDE)).set_corner_radius(28);
        self.card.set_color(Color::rgb(1.0, 0.0, 0.0));
        self.card.set_shadow(Shadow {
            offset: (0, 14).into(),
            radius: 24.0,
            color:  BLACK.with_alpha(0.55),
        });
        self.status.set_text("the card slides for 2 s while a record runs").set_frame((
            20,
            CARD_Y + SIDE + 60.0,
            560,
            50,
        ));
    }
}

/// Sends a command and picks the wanted answer.
fn send<T>(command: InspectorCommand, pick: impl FnOnce(&AppCommand) -> Option<T>) -> Result<T> {
    let response = InspectService::process_command(command);
    let result = pick(&response).ok_or_else(|| match &response {
        AppCommand::Error(error) => anyhow!("{error}"),
        other => anyhow!("Unexpected inspect response: {other:?}"),
    });
    on_main(move || drop(response));
    result
}

fn record(command: InspectorCommand) -> Result<RecordRepr> {
    send(command, |response| match response {
        AppCommand::Record(record) => Some(record.clone()),
        _ => None,
    })
}

fn pictures(from: u32, count: u32) -> Result<Vec<KeptFrameRepr>> {
    send(
        InspectorCommand::RecordFrames { from, count },
        |response| match response {
            AppCommand::KeptFrames(frames) => Some(frames.clone()),
            _ => None,
        },
    )
}

fn layouts() -> Result<Vec<LayoutRepr>> {
    send(InspectorCommand::RecordLayouts, |response| match response {
        AppCommand::KeptLayouts(layouts) => Some(layouts.clone()),
        _ => None,
    })
}

/// The left edge of the card in a kept layout. The subviews of a
/// `ViewRepr` are `Own` pointers, so it is read and dropped on the main
/// thread.
fn card_x(layout: &LayoutRepr) -> Result<f32> {
    fn find(view: &ViewRepr) -> Option<f32> {
        if view.label.starts_with("InspectRecord.card") {
            return Some(view.frame.origin.x);
        }
        view.subviews.iter().find_map(|sub| find(sub))
    }
    let json = layout.json.clone();
    let x = from_main(move || -> Result<Option<f32>> {
        let root: ViewRepr = serde_json::from_str(&json)?;
        Ok(find(&root))
    })?;
    x.with_context(|| format!("No card in the layout of frame {}", layout.index))
}

/// The layouts of the slide: 1 for each frame where the tree changed. The
/// card moves in every frame, so nearly every frame has one, and the card
/// moves through them.
fn check_layouts(stopped: &RecordRepr) -> Result<()> {
    let layouts = layouts()?;
    info!(
        "Layouts of the slide: {} for {} frames",
        layouts.len(),
        stopped.frames.len()
    );
    ensure!(
        layouts.len() * 10 >= stopped.frames.len() * 9,
        "Only {} layouts for {} frames of a moving card",
        layouts.len(),
        stopped.frames.len()
    );
    ensure!(
        layouts
            .windows(2)
            .all(|pair| pair[0].index < pair[1].index && pair[0].ms <= pair[1].ms),
        "The layouts must come in the order of the frames"
    );
    ensure!(
        layouts
            .iter()
            .all(|layout| stopped.frames.iter().any(|frame| frame.index == layout.index)),
        "A layout names a frame the record does not have"
    );
    let first = card_x(&layouts[0])?;
    let last = card_x(&layouts[layouts.len() - 1])?;
    ensure!(
        last - first >= TRAVEL * 0.8,
        "The card went from {first} to {last} in the layouts, it slides {TRAVEL}"
    );
    Ok(())
}

/// The left edge of the card in a kept picture, in points. A kept picture
/// has 1 pixel per point. Only the card has a full color on its middle
/// row, its shadow and the window behind it are pale.
fn card(frame: &KeptFrameRepr) -> Result<f32> {
    let qoi = STANDARD.decode(&frame.qoi_base64)?;
    let picture = load_from_memory_with_format(&qoi, ImageFormat::Qoi)?.to_rgb8();
    ensure!(
        picture.dimensions() == (frame.info.width, frame.info.height),
        "The picture is {:?}, its frame says {}x{}",
        picture.dimensions(),
        frame.info.width,
        frame.info.height
    );
    let row: u32 = (CARD_Y + SIDE / 2.0).lossy_convert();
    let edge = (0..picture.width())
        .find(|column| {
            let color = picture.get_pixel(*column, row).0;
            color.iter().max() > color.iter().min().map(|low| low.saturating_add(150)).as_ref()
        })
        .with_context(|| format!("No card in frame {}", frame.info.index))?;
    Ok(edge.lossy_convert())
}

/// The record of the slide: every drawn frame, in order, none lost.
fn check_slide(view: Weak<InspectRecord>) -> Result<RecordRepr> {
    let started = record(InspectorCommand::RecordStart)?;
    ensure!(started.running, "A started record must run");
    let twice = record(InspectorCommand::RecordStart).unwrap_err().to_string();
    ensure!(twice.contains("already runs"), "{twice}");

    from_main(move || view.slide());
    wait_until("the card arrives", move || view.arrived)?;
    let stopped = record(InspectorCommand::RecordStop)?;

    let frames = &stopped.frames;
    let gaps: Vec<f32> = frames.windows(2).map(|pair| pair[1].ms - pair[0].ms).collect();
    let mut sorted = gaps.clone();
    sorted.sort_by(f32::total_cmp);
    let middle = sorted.get(sorted.len() / 2).copied().unwrap_or_default();
    let longest = sorted.last().copied().unwrap_or_default();
    info!(
        "Record of the slide: {} frames, drawn {}, recorded {}, lost {}, waited at {}, the middle gap {middle:.1} \
         ms, the longest {longest:.1} ms",
        frames.len(),
        stopped.drawn,
        stopped.recorded,
        stopped.skipped,
        stopped.stalled
    );

    ensure!(!stopped.running, "A stopped record still runs");
    ensure!(stopped.skipped == 0, "{} frames were lost", stopped.skipped);
    ensure!(
        stopped.recorded == stopped.drawn,
        "The app drew {} frames, the record took {}",
        stopped.drawn,
        stopped.recorded
    );
    ensure!(
        frames.len() == usize::try_from(stopped.recorded)?,
        "A record shorter than 10 seconds keeps every frame, {} of {}",
        frames.len(),
        stopped.recorded
    );
    // 2 seconds of 60 frames a second are 120, a slow machine gets fewer.
    ensure!(
        frames.len() >= 90,
        "Only {} frames in a slide of 2 seconds",
        frames.len()
    );
    ensure!(
        frames
            .iter()
            .enumerate()
            .all(|(place, frame)| usize::try_from(frame.index) == Ok(place)),
        "The frames must count from 0 with none left out"
    );
    ensure!(
        gaps.iter().all(|gap| *gap >= 0.0),
        "The time of a frame went back"
    );
    // The frames come at the step of a screen, also in a headless run.
    ensure!(
        (14.0..20.0).contains(&middle),
        "The middle gap between 2 frames is {middle} ms, a frame of 60 a second is 16.7 ms"
    );
    Ok(stopped)
}

impl ViewTest for InspectRecord {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let nothing = record(InspectorCommand::RecordStop).unwrap_err().to_string();
        ensure!(nothing.contains("No record runs"), "{nothing}");

        let stopped = check_slide(view)?;
        check_layouts(&stopped)?;
        let frames = &stopped.frames;
        let (first, last) = (&frames[0], &frames[frames.len() - 1]);

        // A kept picture has 1 pixel per point, whatever the screen scale.
        let pixels = send(InspectorCommand::Screenshot, |response| match response {
            AppCommand::Screenshot { width, .. } => Some(*width),
            _ => None,
        })?;
        let scale = from_main(UIManager::scale);
        let pixels: f32 = pixels.lossy_convert();
        let points: u32 = (pixels / scale).round().lossy_convert();
        ensure!(
            first.width == points,
            "A kept frame is {} wide, the window is {points} points at scale {scale}",
            first.width
        );

        // The pictures come in pieces, and the card moves through them.
        let begin = pictures(first.index, 5)?;
        ensure!(begin.len() == 5, "5 pictures asked, got {}", begin.len());
        ensure!(begin[0].info.index == first.index && begin[4].info.index == first.index + 4);
        let end = pictures(last.index, 5)?;
        ensure!(
            end.len() == 1,
            "Only 1 frame is left from the last one on, got {}",
            end.len()
        );
        let (from, to) = (card(&begin[0])?, card(&end[0])?);
        ensure!(
            from < START_X + TRAVEL * 0.2 && to > START_X + TRAVEL * 0.9,
            "The card must go from the left to the right over the record, it is at {from} and at {to}"
        );
        ensure!(pictures(first.index, 0).is_err(), "0 pictures is no request");
        checkpoint("a record of the slide is kept, the card is at the right end")?;

        // A new record drops the kept one.
        record(InspectorCommand::RecordStart)?;
        let again = record(InspectorCommand::RecordStop)?;
        ensure!(
            again.frames.len() < frames.len() && again.frames.first().is_none_or(|frame| frame.index == 0),
            "A new record must start empty, it has {} frames",
            again.frames.len()
        );
        check_layouts_only(view)?;
        check_damage_switch()
    }
}

fn render_info(command: InspectorCommand) -> Result<RenderInfoRepr> {
    send(command, |response| match response {
        AppCommand::RenderInfo(info) => Some(info.clone()),
        _ => None,
    })
}

/// The switch of partial frames: off draws every frame whole and says why,
/// on lets frames be partial again. A page has no other way to try it.
fn check_damage_switch() -> Result<()> {
    let off = render_info(InspectorCommand::SetDamage { on: false })?;
    ensure!(!off.damage_on, "The switch did not turn partial frames off");
    ensure!(!off.backend.is_empty(), "No backend in the render info");
    wait_until("a frame drawn whole by the switch", || {
        render_info(InspectorCommand::RenderInfo).is_ok_and(|info| {
            info.frames.last().is_some_and(|frame| {
                frame.whole.as_deref() == Some("the inspector turned partial frames off")
            })
        })
    })?;
    let on = render_info(InspectorCommand::SetDamage { on: true })?;
    ensure!(on.damage_on, "The switch did not turn partial frames back on");
    Ok(())
}

/// A record of layouts alone: no picture, the app draws every frame, and
/// the moving card gives a layout in nearly each of them.
fn check_layouts_only(view: Weak<InspectRecord>) -> Result<()> {
    let started = record(InspectorCommand::RecordLayoutsStart)?;
    ensure!(started.running, "A started record of layouts must run");
    from_main(move || view.slide());
    wait_until("the card arrives", move || view.arrived)?;
    let stopped = record(InspectorCommand::RecordStop)?;
    info!("Record of layouts alone: {} frames drawn", stopped.drawn);
    ensure!(
        stopped.frames.is_empty(),
        "A record of layouts kept {} pictures",
        stopped.frames.len()
    );
    ensure!(
        stopped.drawn >= 90,
        "Only {} frames drawn in a slide of 2 seconds",
        stopped.drawn
    );
    let layouts = layouts()?;
    ensure!(
        layouts.len() * 10 >= usize::try_from(stopped.drawn)? * 9,
        "Only {} layouts for {} frames of a moving card",
        layouts.len(),
        stopped.drawn
    );
    // The first layout is taken before the slide starts, the card still
    // stands where the slide before left it.
    let places = layouts.iter().map(card_x).collect::<Result<Vec<f32>>>()?;
    let low = places.iter().copied().fold(f32::MAX, f32::min);
    let high = places.iter().copied().fold(f32::MIN, f32::max);
    ensure!(
        high - low >= TRAVEL * 0.8,
        "The card went only from {low} to {high} in the layouts, it slides {TRAVEL}"
    );
    Ok(())
}
