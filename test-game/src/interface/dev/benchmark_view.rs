use std::{
    ops::Deref,
    process::exit,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread::sleep,
};

use sysinfo::{Component, Components, MINIMUM_CPU_UPDATE_INTERVAL, ProcessesToUpdate, System};
use test_engine::{
    Window,
    gm::LossyConvert,
    refs::Weak,
    ui::{
        Alert,
        Anchor::{Right, Top, Width, X},
        Button, CheckBox, CircleView, Color, Container, DrawingView, ImageView, Label, NumberView,
        ProgressView, ScrollView, Setup, Slider, Switch, TextAlignment, View, ViewCallbacks, ViewData,
        ViewFrame, ViewSubviews, view,
    },
};

const PANEL_COLS: usize = 10;
const PANEL_ROWS: usize = 8;
const PANEL_SIZE: f32 = 150.0;

/// One panel is added every frame until the rolling average of frame work
/// time crosses this. 16.6 ms is the 60 fps budget.
const LAG_THRESHOLD_MS: f32 = 16.0;
const ROLLING_WINDOW: usize = 10;

/// Safety bound so the run always terminates.
const MAX_PANELS: usize = 10_000;

/// A loaded or thermally throttled machine skews results - scripted runs are
/// rejected with this exit code and the bench harness fails immediately.
/// Pass `--no-guard` to run anyway (the load is still recorded).
pub const BENCH_REJECTED_EXIT_CODE: i32 = 75;

const MAX_CPU_USAGE: f32 = 15.0;
const MAX_LOAD_PER_CORE: f32 = 0.6;
const MAX_CPU_TEMP: f32 = 55.0;

static SYSTEM_AT_START: OnceLock<SystemLoad> = OnceLock::new();

struct SystemLoad {
    cpu_usage:     f32,
    load_per_core: f32,
    cpu_temp:      Option<f32>,
}

fn measure_system() -> SystemLoad {
    let mut system = System::new();
    system.refresh_cpu_usage();

    // The minimum over a window ignores transient spikes (our own process
    // startup, the previous run's decay) but a steady hog keeps every sample
    // high. A single sample rejects falsely right after another run exits.
    let mut cpu_usage = f32::MAX;
    for _ in 0..3 {
        sleep(MINIMUM_CPU_UPDATE_INTERVAL);
        system.refresh_cpu_usage();
        cpu_usage = cpu_usage.min(system.global_cpu_usage());
    }

    let cores: f32 = system.cpus().len().max(1).lossy_convert();
    let load_one: f32 = System::load_average().one.lossy_convert();

    let components = Components::new_with_refreshed_list();
    let cpu_temp = components
        .iter()
        .filter(|component| {
            let label = component.label().to_lowercase();
            label.contains("cpu") || label.contains("tdie") || label.contains("soc")
        })
        .filter_map(Component::temperature)
        .reduce(f32::max);

    SystemLoad {
        cpu_usage,
        load_per_core: load_one / cores,
        cpu_temp,
    }
}

/// Browsers do bursty background work even when idle - JS timers, media,
/// rendering of animated pages. They must be closed, not just quiet.
const BROWSERS: &[&str] = &[
    "google chrome",
    "chrome",
    "chromium",
    "safari",
    "firefox",
    "microsoft edge",
    "brave browser",
    "brave",
    "opera",
    "vivaldi",
    "arc",
    "zen",
    "yandex",
];

fn running_browsers() -> Vec<&'static str> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    // Match only at a word boundary: plain `contains` flags system daemons
    // like searchpartyd ("arc") and siriknowledged ("edge").
    let is_browser = |name: &str| {
        BROWSERS
            .iter()
            .find(|browser| {
                name == **browser || name.strip_prefix(*browser).is_some_and(|rest| rest.starts_with(' '))
            })
            .copied()
    };

    let mut found: Vec<&str> = system
        .processes()
        .values()
        .filter_map(|process| is_browser(&process.name().to_string_lossy().to_lowercase()))
        .collect();

    found.sort_unstable();
    found.dedup();
    found
}

/// Measures the system load (always recorded for the results JSON) and rejects
/// the run when it is busy or hot. `force` (the `--no-guard` flag) downgrades
/// the rejection to a warning so a knowingly-loaded machine can still run.
pub fn guard_benchmark(force: bool) {
    let load = measure_system();
    let mut reasons = vec![];

    let browsers = running_browsers();
    if !browsers.is_empty() {
        reasons.push(format!("browser running, close it: {}", browsers.join(", ")));
    }

    if load.cpu_usage > MAX_CPU_USAGE {
        reasons.push(format!("cpu usage {:.0}% > {MAX_CPU_USAGE}%", load.cpu_usage));
    }

    if load.load_per_core > MAX_LOAD_PER_CORE {
        reasons.push(format!(
            "load average {:.2} per core > {MAX_LOAD_PER_CORE}",
            load.load_per_core
        ));
    }

    if let Some(temp) = load.cpu_temp
        && temp > MAX_CPU_TEMP
    {
        reasons.push(format!("cpu temperature {temp:.0} C > {MAX_CPU_TEMP} C"));
    }

    let busy = load.cpu_usage > MAX_CPU_USAGE || load.load_per_core > MAX_LOAD_PER_CORE;

    SYSTEM_AT_START
        .set(load)
        .unwrap_or_else(|_| panic!("system load measured twice"));

    if reasons.is_empty() {
        return;
    }

    eprintln!("System is busy or hot:");
    for reason in &reasons {
        eprintln!("  {reason}");
    }

    if force {
        eprintln!("  --no-guard set, running anyway - these numbers are not evidence");
        return;
    }

    if busy {
        let to_kill = processes_to_kill();
        if to_kill.is_empty() {
            eprintln!("  no heavy processes found, the machine is likely still cooling down");
        } else {
            eprintln!("  consider killing: {to_kill}");
        }
    }

    exit(BENCH_REJECTED_EXIT_CODE);
}

/// 100% here is one core, unlike the normalized total in the rejection reason.
fn processes_to_kill() -> String {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_processes(ProcessesToUpdate::All, true);

    let own_pid = sysinfo::get_current_pid().ok();

    let mut processes: Vec<_> = system
        .processes()
        .values()
        .filter(|p| Some(p.pid()) != own_pid && p.cpu_usage() > 25.0)
        .collect();

    processes.sort_by(|a, b| b.cpu_usage().total_cmp(&a.cpu_usage()));

    processes
        .iter()
        .take(3)
        .map(|p| format!("{} {:.0}%", p.name().to_string_lossy(), p.cpu_usage()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Step between rows in the final report. Every frame is recorded, printing
/// all of them would flood the console.
const REPORT_EVERY: usize = 50;

static PANEL_INDEX: AtomicUsize = AtomicUsize::new(0);

const TEXTS: &[&str] = &["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf"];

/// Random-looking but a pure function of the input - panel N has the same
/// color in every run. splitmix64 finalizer.
fn shade(i: usize, salt: usize) -> Color {
    let mut x = (i as u64).wrapping_add((salt as u64) << 32);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;

    let channel = |byte: u64| -> f32 {
        let v: f32 = ((x >> (byte * 8)) & 0xFF).lossy_convert();
        0.15 + 0.7 * (v / 255.0)
    };
    Color::rgb(channel(0), channel(1), channel(2))
}

/// Rounding must happen in f64: rounded f32 values still print with long
/// garbage tails in JSON.
fn round2(value: f32) -> f64 {
    (f64::from(value) * 100.0).round() / 100.0
}

fn count_views(view: &dyn View) -> usize {
    1 + view.subviews().iter().map(|sub| count_views(sub.deref())).sum::<usize>()
}

struct StepResult {
    panels: usize,
    views:  usize,
    avg_ms: f32,
    gpu_ms: f32,
}

#[view]
pub struct UIBenchmarkView {
    panels:            usize,
    views_per_panel:   usize,
    reported:          bool,
    last_render_frame: u64,
    skip_frames:       u32,
    results:           Vec<StepResult>,
}

impl Setup for UIBenchmarkView {
    fn setup(self: Weak<Self>) {
        Window::set_vsync(false);
        PANEL_INDEX.store(0, Ordering::Relaxed);
    }
}

impl ViewCallbacks for UIBenchmarkView {
    fn update(&mut self) {
        if self.reported {
            return;
        }

        // An occluded window skips rendering and frame_work_time goes stale.
        // Counting such frames would inflate the result with unmeasured panels.
        let render_frame = Window::render_frame();
        if render_frame == self.last_render_frame {
            return;
        }
        self.last_render_frame = render_frame;

        // The frame right after adding panels contains their creation cost.
        // Only clean render-only frames are recorded, so the metric measures
        // rendering, not view construction.
        if self.skip_frames > 0 {
            self.skip_frames -= 1;
            return;
        }

        if self.panels > 0 {
            self.results.push(StepResult {
                panels: self.panels,
                views:  self.panels * self.views_per_panel,
                avg_ms: Window::current().frame_work_time() * 1000.0,
                gpu_ms: Window::current().frame_gpu_time() * 1000.0,
            });

            if (self.rolling_ms() >= LAG_THRESHOLD_MS && self.results.len() >= ROLLING_WINDOW)
                || self.panels >= MAX_PANELS
            {
                self.reported = true;
                self.report();
                return;
            }
        }

        for _ in 0..self.step_size() {
            self.add_panel();
        }

        if self.views_per_panel == 0 {
            self.views_per_panel = (count_views(self) - 1) / self.panels;
        }

        self.skip_frames = 1;
    }
}

impl UIBenchmarkView {
    /// Far from the lag threshold whole batches are added per frame to keep
    /// the run short. Near it one panel per frame, so the stop point keeps
    /// single-panel precision.
    fn step_size(&self) -> usize {
        let ratio = self.rolling_ms() / LAG_THRESHOLD_MS;
        if ratio < 0.5 {
            32
        } else if ratio < 0.8 {
            8
        } else if ratio < 0.95 {
            4
        } else {
            1
        }
    }

    fn rolling_ms(&self) -> f32 {
        self.rolling_avg(|r| r.avg_ms)
    }

    fn rolling_gpu_ms(&self) -> f32 {
        self.rolling_avg(|r| r.gpu_ms)
    }

    fn rolling_avg(&self, field: impl Fn(&StepResult) -> f32) -> f32 {
        let window = self.results.len().min(ROLLING_WINDOW);
        if window == 0 {
            return 0.0;
        }
        let sum: f32 = self.results[self.results.len() - window..].iter().map(field).sum();
        let len: f32 = window.lossy_convert();
        sum / len
    }

    // The grid covers the window, then panels stack on top of it in layers
    // shifted by a deterministic offset. Off-screen views are culled from
    // rendering, so panels must stay inside the window to keep costing.
    fn add_panel(&mut self) {
        let per_layer = PANEL_COLS * PANEL_ROWS;

        let col: f32 = (self.panels % PANEL_COLS).lossy_convert();
        let row: f32 = (self.panels / PANEL_COLS % PANEL_ROWS).lossy_convert();
        let layer = self.panels / per_layer;

        let shift_x: f32 = (layer * 13 % 120).lossy_convert();
        let shift_y: f32 = (layer * 29 % 120).lossy_convert();

        let panel = self.add_view::<BenchPanel>();
        panel.set_frame((
            col * PANEL_SIZE + shift_x,
            row * PANEL_SIZE + shift_y,
            PANEL_SIZE,
            PANEL_SIZE,
        ));

        self.panels += 1;
    }

    fn sampled(&self) -> impl Iterator<Item = &StepResult> {
        let last_index = self.results.len() - 1;
        self.results
            .iter()
            .enumerate()
            .filter(move |(index, _)| index % REPORT_EVERY == 0 || *index == last_index)
            .map(|(_, result)| result)
    }

    fn report(&self) {
        println!();
        println!("UI benchmark:");
        println!(
            "{:<7} {:>7} {:>8} {:>8} {:>9}",
            "panels", "views", "cpu_ms", "gpu_ms", "fps"
        );

        for result in self.sampled() {
            let fps = if result.avg_ms > 0.0 {
                1000.0 / result.avg_ms
            } else {
                0.0
            };
            println!(
                "{:<7} {:>7} {:>8.3} {:>8.3} {:>9.1}",
                result.panels, result.views, result.avg_ms, result.gpu_ms, fps
            );
        }

        self.write_json();

        let last = self.results.last().expect("benchmark finished with no results");
        let rolling = self.rolling_ms();
        Alert::show(format!(
            "{} panels, {} views: {rolling:.2} ms cpu, {:.2} ms gpu ({:.0} fps)",
            last.panels,
            last.views,
            self.rolling_gpu_ms(),
            if rolling > 0.0 { 1000.0 / rolling } else { 0.0 }
        ));

        // Scripted run: UI_BENCHMARK=1 cargo run -p test-game. Exit so the
        // caller's pipe gets the report. From the menu the app stays alive.
        if std::env::var("UI_BENCHMARK").is_ok() {
            test_engine::AppRunner::stop();
        }
    }

    fn write_json(&self) {
        let Ok(path) = std::env::var("UI_BENCHMARK_JSON") else {
            return;
        };

        let last = self.results.last().expect("benchmark finished with no results");
        let system = SYSTEM_AT_START.get();

        let json = serde_json::json!({
            "panels": last.panels,
            "views": last.views,
            "ms": round2(self.rolling_ms()),
            "gpu_ms": round2(self.rolling_gpu_ms()),
            "cpu_usage": system.map(|s| round2(s.cpu_usage)),
            "load_per_core": system.map(|s| round2(s.load_per_core)),
            "cpu_temp": system.and_then(|s| s.cpu_temp).map(round2),
        });

        let json = serde_json::to_string_pretty(&json).expect("failed to serialize benchmark results");

        if let Err(err) = std::fs::write(&path, json) {
            eprintln!("Failed to write benchmark JSON to {path}: {err}");
        }
    }
}

#[view]
struct BenchPanel {
    #[init]
    title:      Label,
    body:       Label,
    button:     Button,
    image:      ImageView,
    switch:     Switch,
    check:      CheckBox,
    slider:     Slider,
    progress:   ProgressView,
    circle:     CircleView,
    number:     NumberView,
    drawing:    DrawingView,
    badge:      Container,
    rel_box:    Container,
    halves:     Container,
    footer:     Container,
    custom_box: Container,
    scroll:     ScrollView,
}

impl Setup for BenchPanel {
    fn setup(mut self: Weak<Self>) {
        let i = PANEL_INDEX.fetch_add(1, Ordering::Relaxed);

        self.set_gradient(shade(i, 0), shade(i, 1));
        self.set_corner_radius(4);
        self.set_border_color(shade(i, 2));
        self.set_border_width(1);

        self.title
            .set_text(TEXTS[i % TEXTS.len()])
            .set_text_color(shade(i, 13))
            .set_text_size(9)
            .set_alignment(TextAlignment::Left);
        self.title.set_color(shade(i, 14));
        self.title.place().lrt(2).h(14).max_width(400).min_height(10);

        self.body
            .set_text("multi\nline\ntext")
            .set_text_color(shade(i, 15))
            .set_multiline(true)
            .set_text_size(8);
        self.body.set_color(shade(i, 16));
        self.body.place().same([Width, X], self.title).anchor(Top, self.title, 2).h(30);

        self.button.set_text("tap").set_text_color(shade(i, 17)).set_text_size(8);
        self.button.set_color(shade(i, 18));
        self.button.place().bl(2).size(46, 14);

        self.image.set_image("round.png");
        self.image.set_color(shade(i, 19)).set_border_color(shade(i, 20));
        self.image.place().br(2).size(18, 18);

        self.switch.set_off_color(shade(i, 21));
        self.switch.place().tr(2).size(34, 16);
        if i.is_multiple_of(2) {
            self.switch.set_on(true);
        }

        self.check.set_color(shade(i, 22)).set_border_color(shade(i, 23));
        self.check
            .place()
            .same([Width, X], self.switch)
            .anchor(Top, self.switch, 2)
            .h(16);
        if i.is_multiple_of(3) {
            self.check.set_on(true);
        }

        self.slider.set_color(shade(i, 24));
        self.slider.place().r(2).center_y().size(12, 44);

        self.progress.set_color(shade(i, 25));
        self.progress.place().lr(2).b(18).h(4);
        let progress: f32 = (i % 10).lossy_convert();
        self.progress.set_progress(progress / 10.0);

        self.circle.set_radius(6);
        self.circle.set_circle_color(shade(i, 3));
        self.circle.place().between(self.button, self.image);

        self.number.set_color(shade(i, 26));
        self.number.place().center().size(24, 28);
        let value: f32 = (i % 50).lossy_convert();
        self.number.set_value(value);

        self.drawing.place().l(2).center_y().size(26, 26);
        self.drawing.add_path([(0, 0), (13, 22), (26, 0)], shade(i, 4));

        self.badge.set_color(shade(i, 5)).set_corner_radius(3);
        self.badge.place().size(7, 7).between_super(self.button, Right);

        self.rel_box.set_color(shade(i, 6));
        self.rel_box.place().bl(20).relative(Width, self.title, 0.4).h(6);

        self.halves.set_color(shade(i, 27));
        self.halves.place().lr(2).t(16).h(8);
        let left = self.halves.add_view::<Container>();
        left.set_color(shade(i, 7));
        left.place().left_half();
        let right = self.halves.add_view::<Container>();
        right.set_color(shade(i, 8));
        right.place().right_half();

        self.footer.set_color(shade(i, 28));
        self.footer.place().lr(2).b(8).h(8).distribute_ratio([1.0, 2.0, 3.0]);
        for salt in 9..12 {
            self.footer.add_view::<Container>().set_color(shade(i, salt));
        }

        self.custom_box.set_color(shade(i, 12));
        let width: f32 = (10 + (i * 7) % 20).lossy_convert();
        self.custom_box
            .place()
            .custom(move |rect| *rect = (2.0, 30.0, width, 6.0).into());

        self.scroll.set_color(shade(i, 29));
        self.scroll.place().t(2).r(40).size(30, 40);
        self.scroll.set_content_size((30, 120));
        let scroll_label = self.scroll.add_view::<Label>();
        scroll_label.set_text("scroll").set_text_color(shade(i, 30)).set_text_size(8);
        scroll_label.set_color(shade(i, 31));
        scroll_label.place().tl(1).size(28, 20);
    }
}
