mod ax;
mod capture;
mod diff;
mod report;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use clap::{Args, Parser, Subcommand};
use image::RgbImage;

use crate::diff::{Options, Rect};

/// Pixel parity tool for ports. Capture app windows from the screen,
/// then diff two same size captures into ranked difference regions.
/// Both captures go through the display pipeline, so it also catches
/// what framebuffer comparison cannot, like colorspace bugs.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// The comparison knobs, shared by `diff` and `run`.
#[derive(Args)]
struct Tuning {
    /// Cell size of the comparison grid in pixels.
    #[arg(long, default_value_t = 8)]
    cell:            u32,
    /// Max per channel delta of a cell mean before it is different.
    #[arg(long, default_value_t = 3)]
    cell_tolerance:  u8,
    /// Max per channel delta of one pixel before it is different.
    #[arg(long, default_value_t = 12)]
    pixel_tolerance: u8,
    /// Percent of differing pixels marking a cell even when means agree.
    #[arg(long, default_value_t = 35)]
    pixel_percent:   u32,
    /// Region to skip as x,y,w,h in pixels, repeatable.
    #[arg(long)]
    ignore:          Vec<String>,
    /// Pixels per point of the captures, for the point coordinates.
    #[arg(long, default_value_t = 2)]
    scale:           u32,
}

#[derive(Subcommand)]
enum Command {
    /// Capture one app window from the screen into a png.
    Capture {
        /// Owner app name, case insensitive, or a window id.
        query: String,
        /// Output png path.
        #[arg(short, long)]
        out:   PathBuf,
    },
    /// Resize one app window to a size in points, title bar included.
    Resize {
        /// Owner app name, case insensitive, or a window id.
        query: String,
        /// Target size as `WxH` in points.
        size:  String,
    },
    /// Diff two same size captures, print ranked regions, write a heatmap.
    Diff {
        a:          PathBuf,
        b:          PathBuf,
        /// Heatmap png, the second image with regions outlined in red.
        #[arg(short, long)]
        out:        Option<PathBuf>,
        /// Rows to drop from the top of the first image, its title bar.
        #[arg(long, default_value_t = 0)]
        crop_top_a: u32,
        /// Rows to drop from the top of the second image, its title bar.
        #[arg(long, default_value_t = 0)]
        crop_top_b: u32,
        #[command(flatten)]
        tuning:     Tuning,
    },
    /// The whole parity check in one go, resize both app windows to the
    /// same size, capture both, diff, then restore the original sizes.
    Run {
        /// First app, owner name or window id, usually the original.
        a:        String,
        /// Second app, owner name or window id, usually the port.
        b:        String,
        /// Window size both apps get, as `WxH` in points.
        #[arg(long, default_value = "1280x832")]
        size:     String,
        /// Directory for the captures and the heatmap.
        #[arg(short, long, default_value = ".")]
        out:      PathBuf,
        /// Rows to drop from the top of both captures, the title bars.
        #[arg(long, default_value_t = 0)]
        crop_top: u32,
        #[command(flatten)]
        tuning:   Tuning,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Capture { query, out } => capture::capture(&query, &out),
        Command::Resize { query, size } => {
            let (width, height) = parse_size(&size)?;
            let window = capture::find_window(&query)?;
            ax::AxWindow::find(window.pid, window.id)?.set_size(width, height)
        }
        Command::Diff {
            a,
            b,
            out,
            crop_top_a,
            crop_top_b,
            tuning,
        } => {
            let image_a = load(&a, crop_top_a)?;
            let image_b = load(&b, crop_top_b)?;
            run_diff(&image_a, &image_b, out.as_deref(), &tuning)
        }
        Command::Run {
            a,
            b,
            size,
            out,
            crop_top,
            tuning,
        } => {
            let (width, height) = parse_size(&size)?;
            let window_a = capture::find_window(&a)?;
            let window_b = capture::find_window(&b)?;
            ensure!(
                window_a.id != window_b.id,
                "both queries hit the same window {} of {}",
                window_a.id,
                window_a.owner,
            );
            let ax_a = ax::AxWindow::find(window_a.pid, window_a.id)?;
            let ax_b = ax::AxWindow::find(window_b.pid, window_b.id)?;
            let old_size_a = ax_a.size()?;
            let old_size_b = ax_b.size()?;
            let old_position_a = ax_a.position()?;
            let old_position_b = ax_b.position()?;
            // macOS clamps a window frame to the screen, so park both at
            // the top left corner before resizing or a window near the
            // right edge silently gets less width than asked.
            ax_a.set_position(0.0, 40.0)?;
            ax_b.set_position(0.0, 40.0)?;
            ax_a.set_size(width, height)?;
            ax_b.set_size(width, height)?;
            // Give both apps a beat to relayout and redraw.
            std::thread::sleep(std::time::Duration::from_millis(900));

            let path_a = out.join("pixdiff_a.png");
            let path_b = out.join("pixdiff_b.png");
            let captured = capture::capture_window(&window_a, &path_a)
                .and_then(|()| capture::capture_window(&window_b, &path_b));
            ax_a.set_size(old_size_a.0, old_size_a.1)?;
            ax_b.set_size(old_size_b.0, old_size_b.1)?;
            ax_a.set_position(old_position_a.0, old_position_a.1)?;
            ax_b.set_position(old_position_b.0, old_position_b.1)?;
            captured?;

            let image_a = load(&path_a, crop_top)?;
            let image_b = load(&path_b, crop_top)?;
            println!("a: {}", path_a.display());
            println!("b: {}", path_b.display());
            run_diff(
                &image_a,
                &image_b,
                Some(out.join("pixdiff_heatmap.png").as_path()),
                &tuning,
            )
        }
    }
}

fn run_diff(a: &RgbImage, b: &RgbImage, out: Option<&Path>, tuning: &Tuning) -> Result<()> {
    ensure!(
        a.dimensions() == b.dimensions(),
        "captures differ in size, {:?} vs {:?}, redo them at the same window size",
        a.dimensions(),
        b.dimensions(),
    );
    let options = Options {
        cell:            tuning.cell,
        cell_tolerance:  tuning.cell_tolerance,
        pixel_tolerance: tuning.pixel_tolerance,
        pixel_percent:   tuning.pixel_percent,
        ignore:          tuning
            .ignore
            .iter()
            .map(|spec| parse_rect(spec))
            .collect::<Result<Vec<Rect>>>()?,
    };
    let regions = diff::run(a, b, &options);
    report::print(&regions, tuning.scale);
    if let Some(out) = out {
        report::heatmap(b, &regions)
            .save(out)
            .with_context(|| format!("writing heatmap to {}", out.display()))?;
        println!("heatmap: {}", out.display());
    }
    ensure!(regions.is_empty(), "{} differences", regions.len());
    Ok(())
}

fn load(path: &Path, crop_top: u32) -> Result<RgbImage> {
    let image = image::open(path)
        .with_context(|| format!("reading {}", path.display()))?
        .into_rgb8();
    if crop_top == 0 {
        return Ok(image);
    }
    let (width, height) = image.dimensions();
    ensure!(crop_top < height, "crop top {crop_top} exceeds height {height}");
    Ok(image::imageops::crop_imm(&image, 0, crop_top, width, height - crop_top).to_image())
}

fn parse_size(spec: &str) -> Result<(f64, f64)> {
    let Some((width, height)) = spec.split_once('x') else {
        bail!("size must be WxH in points, got {spec:?}");
    };
    let parse = |part: &str| part.trim().parse::<u32>().with_context(|| format!("bad size {spec:?}"));
    Ok((f64::from(parse(width)?), f64::from(parse(height)?)))
}

fn parse_rect(spec: &str) -> Result<Rect> {
    let parts: Vec<u32> = spec
        .split(',')
        .map(|part| part.trim().parse::<u32>().with_context(|| format!("bad ignore rect {spec:?}")))
        .collect::<Result<Vec<u32>>>()?;
    let [x, y, w, h] = parts.as_slice() else {
        bail!("ignore rect must be x,y,w,h, got {spec:?}");
    };
    Ok(Rect {
        x: *x,
        y: *y,
        w: *w,
        h: *h,
    })
}
