//! The painters of the skeuomorphic hardware panel the `DrawingLayers`
//! test draws. Every element is stacked fills and strokes in one
//! `DrawingView`, so each one exercises painter order and the gradient
//! paints.

use crate::{
    deps::refs::Weak,
    gm::{
        color::{BLACK, Color, WHITE},
        flat::{FillRule, LineCap, Paint, StrokeStyle, VectorPath},
    },
    ui::DrawingView,
};

pub(super) const SHADE: Color = Color::hex("#0d0f12");
pub(super) const LED_GREEN: Color = Color::hex("#2eea62");

pub(super) fn circle(
    mut drawing: Weak<DrawingView>,
    center: (f32, f32),
    radius: f32,
    paint: impl Into<Paint>,
) {
    drawing.add_fill(&VectorPath::circle(center, radius), paint, FillRule::NonZero);
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> VectorPath {
    VectorPath::polygon([(x, y), (x + width, y), (x + width, y + height), (x, y + height)])
}

fn line(
    mut drawing: Weak<DrawingView>,
    from: (f32, f32),
    to: (f32, f32),
    paint: impl Into<Paint>,
    width: f32,
) {
    drawing.add_stroke(
        &VectorPath::polyline([from, to]),
        paint,
        StrokeStyle::width(width).cap(LineCap::Round),
    );
}

/// The machined knob: soft shadow, a vertical metal ramp under a conic
/// sheen, the knurled grip, the recess whose wall shades the top of
/// the face dome, a gloss fade, a rim light and the pointer groove.
pub(super) fn knob(mut drawing: Weak<DrawingView>, x: f32, y: f32) {
    circle(
        drawing,
        (x, y + 8.0),
        110.0,
        Paint::radial((x, y + 8.0), 110.0, BLACK.with_alpha(0.55), BLACK.with_alpha(0.0)),
    );
    circle(
        drawing,
        (x, y),
        90.0,
        Paint::linear(
            (x, y - 90.0),
            (x, y + 90.0),
            Color::hex("#f6f8fa"),
            Color::hex("#d3d7db"),
        )
        .stop(Color::hex("#c9ced4"), 0.3)
        .stop(Color::hex("#878e96"), 0.72)
        .grain(0.05),
    );
    circle(
        drawing,
        (x, y),
        90.0,
        Paint::conic((x, y), 6.0, WHITE.with_alpha(0.0), WHITE.with_alpha(0.25)).grain(0.12),
    );
    drawing.add_stroke(
        &VectorPath::circle((x, y), 86),
        Paint::conic((x, y), 48.0, SHADE.with_alpha(0.0), SHADE.with_alpha(0.35)),
        StrokeStyle::width(5),
    );
    drawing.add_stroke(
        &VectorPath::circle((x, y), 89.5),
        SHADE.with_alpha(0.6),
        StrokeStyle::width(1.5),
    );
    circle(drawing, (x, y + 1.0), 73.0, Color::hex("#0e1013"));
    circle(
        drawing,
        (x, y - 1.0),
        71.0,
        Paint::radial(
            (x - 24.0, y - 32.0),
            113.0,
            Color::hex("#515863"),
            Color::hex("#22252a"),
        )
        .stop(Color::hex("#3d434c"), 0.55)
        .grain(0.04),
    );
    circle(
        drawing,
        (x, y - 1.0),
        71.0,
        Paint::linear(
            (x, y - 71.0),
            (x, y - 3.0),
            SHADE.with_alpha(0.4),
            SHADE.with_alpha(0.0),
        ),
    );
    circle(
        drawing,
        (x, y - 1.0),
        70.0,
        Paint::linear(
            (x, y - 70.0),
            (x, y + 28.0),
            WHITE.with_alpha(0.12),
            WHITE.with_alpha(0.0),
        ),
    );
    drawing.add_stroke(
        &VectorPath::arc((x, y - 1.0), 69.0, -2.5, 1.6),
        WHITE.with_alpha(0.25),
        StrokeStyle::width(1.5).cap(LineCap::Round),
    );
    line(drawing, (x, y - 59.0), (x, y - 34.0), SHADE.with_alpha(0.55), 4.0);
    line(drawing, (x, y - 58.0), (x, y - 36.0), Color::hex("#e0e3e7"), 2.0);
}

/// A recessed screw, see the slot over the machined dome. The strokes
/// over the dome are the exact stroke over fill case that used to
/// disappear.
pub(super) fn screw(mut drawing: Weak<DrawingView>, x: f32, y: f32, angle: f32) {
    circle(
        drawing,
        (x, y + 3.0),
        34.0,
        Paint::radial((x, y + 3.0), 34.0, BLACK.with_alpha(0.5), BLACK.with_alpha(0.0)),
    );
    circle(
        drawing,
        (x, y),
        27.0,
        Paint::linear(
            (x, y - 27.0),
            (x, y + 27.0),
            Color::hex("#07080a"),
            Color::hex("#24272c"),
        ),
    );
    drawing.add_stroke(
        &VectorPath::arc((x, y), 25.5, 0.5, 2.1),
        WHITE.with_alpha(0.12),
        StrokeStyle::width(2).cap(LineCap::Round),
    );

    circle(
        drawing,
        (x, y),
        22.0,
        Paint::radial(
            (x - 6.0, y - 7.0),
            33.0,
            Color::hex("#d8dde2"),
            Color::hex("#5f666e"),
        )
        .stop(Color::hex("#a9b0b7"), 0.5)
        .grain(0.09),
    );
    circle(
        drawing,
        (x, y),
        22.0,
        Paint::conic((x, y), 5.0, WHITE.with_alpha(0.0), WHITE.with_alpha(0.22)).grain(0.14),
    );
    circle(
        drawing,
        (x, y),
        22.0,
        Paint::linear(
            (x, y + 4.0),
            (x, y + 20.0),
            SHADE.with_alpha(0.0),
            SHADE.with_alpha(0.3),
        ),
    );
    drawing.add_stroke(
        &VectorPath::circle((x, y), 21.0),
        SHADE.with_alpha(0.4),
        StrokeStyle::width(1.5),
    );
    drawing.add_stroke(
        &VectorPath::arc((x, y), 19.0, -2.9, 1.2),
        WHITE.with_alpha(0.4),
        StrokeStyle::width(1.5).cap(LineCap::Round),
    );

    let (sin, cos) = angle.to_radians().sin_cos();
    let slot = |length: f32, offset: f32| {
        VectorPath::polyline([
            (length.mul_add(-cos, x), length.mul_add(-sin, y) + offset),
            (length.mul_add(cos, x), length.mul_add(sin, y) + offset),
        ])
    };
    drawing.add_stroke(
        &slot(17.0, 0.0),
        Paint::linear(
            (x, y - 3.0),
            (x, y + 3.0),
            Color::hex("#080a0c"),
            Color::hex("#22262b"),
        ),
        StrokeStyle::width(5).cap(LineCap::Round),
    );
    drawing.add_stroke(
        &slot(16.5, -2.0),
        BLACK.with_alpha(0.4),
        StrokeStyle::width(1.5).cap(LineCap::Round),
    );
    drawing.add_stroke(
        &slot(16.0, 3.2),
        WHITE.with_alpha(0.4),
        StrokeStyle::width(1.5).cap(LineCap::Round),
    );
}

/// An indicator lamp. The lens is an emitter, a symmetric radial from
/// a hot core to a deep rim, no offset shading and no bubble specular.
/// A lit lamp gets two bloom layers, an internal glow ring and light
/// spilling onto its collar, an unlit one only the dark lens.
pub(super) fn lamp(mut drawing: Weak<DrawingView>, x: f32, y: f32, glow: Color, lens: Paint, lit: bool) {
    if lit {
        circle(
            drawing,
            (x, y),
            70.0,
            Paint::radial((x, y), 70.0, glow.with_alpha(0.25), glow.with_alpha(0.0)),
        );
        circle(
            drawing,
            (x, y),
            36.0,
            Paint::radial((x, y), 36.0, glow.with_alpha(0.45), glow.with_alpha(0.0)),
        );
    }
    circle(
        drawing,
        (x, y + 3.0),
        34.0,
        Paint::radial((x, y + 3.0), 34.0, BLACK.with_alpha(0.5), BLACK.with_alpha(0.0)),
    );
    circle(
        drawing,
        (x, y),
        30.0,
        Paint::linear(
            (x, y - 30.0),
            (x, y + 30.0),
            Color::hex("#eef1f4"),
            Color::hex("#d3d7db"),
        )
        .stop(Color::hex("#c9ced4"), 0.3)
        .stop(Color::hex("#878e96"), 0.72)
        .grain(0.05),
    );
    circle(
        drawing,
        (x, y),
        30.0,
        Paint::conic((x, y), 6.0, WHITE.with_alpha(0.0), WHITE.with_alpha(0.25)).grain(0.12),
    );
    drawing.add_stroke(
        &VectorPath::circle((x, y), 29.5),
        SHADE.with_alpha(0.6),
        StrokeStyle::width(1.5),
    );
    circle(drawing, (x, y + 1.0), 23.0, Color::hex("#0a0c0e"));
    circle(drawing, (x, y), 22.0, lens);
    if lit {
        circle(
            drawing,
            (x, y),
            22.0,
            Paint::radial((x, y), 22.0, glow.with_alpha(0.0), glow.with_alpha(0.0))
                .stop(glow.with_alpha(0.3), 0.9),
        );
        drawing.add_stroke(
            &VectorPath::circle((x, y), 23.5),
            glow.with_alpha(0.3),
            StrokeStyle::width(1.5),
        );
    }
    drawing.add_stroke(
        &VectorPath::circle((x, y), 22),
        SHADE.with_alpha(0.5),
        StrokeStyle::width(1),
    );
}

pub(super) fn green_lens(x: f32, y: f32) -> Paint {
    Paint::radial((x, y), 22.0, Color::hex("#f2fff5"), Color::hex("#064018"))
        .stop(Color::hex("#52f07e"), 0.3)
        .stop(Color::hex("#16c948"), 0.55)
        .stop(Color::hex("#0a7a2c"), 0.85)
}

pub(super) fn red_lens(x: f32, y: f32) -> Paint {
    Paint::radial((x, y), 22.0, Color::hex("#5c1512"), Color::hex("#1d0605"))
        .stop(Color::hex("#47100e"), 0.4)
        .stop(Color::hex("#300b09"), 0.7)
}

/// The VU meter: a beveled dark frame, a cream backing shaded at the
/// bottom, tick marks and a red zone on an arc, the needle over them,
/// its pivot cap, an inner top shadow and a glass gloss over the top
/// half.
pub(super) fn vu_meter(mut drawing: Weak<DrawingView>, x: f32, y: f32, width: f32) {
    // x, y is the top left of the frame, 125 tall.
    drawing.add_fill(
        &rect(x, y, width, 125.0),
        Paint::linear(
            (x, y),
            (x, y + 125.0),
            Color::hex("#101214"),
            Color::hex("#33373c"),
        ),
        FillRule::NonZero,
    );
    line(
        drawing,
        (x + 2.0, y + 124.0),
        (x + width - 2.0, y + 124.0),
        WHITE.with_alpha(0.1),
        2.0,
    );
    drawing.add_fill(
        &rect(x + 10.0, y + 10.0, width - 20.0, 105.0),
        Paint::linear(
            (x, y + 10.0),
            (x, y + 115.0),
            Color::hex("#ece6d6"),
            Color::hex("#cfc7b2"),
        ),
        FillRule::NonZero,
    );
    drawing.add_fill(
        &rect(x + 10.0, y + 10.0, width - 20.0, 105.0),
        Paint::linear(
            (x, y + 10.0),
            (x, y + 34.0),
            BLACK.with_alpha(0.35),
            BLACK.with_alpha(0.0),
        ),
        FillRule::NonZero,
    );

    let pivot = (x + width / 2.0, y + 113.0);
    let polar = |radius: f32, degrees: f32| {
        let (sin, cos) = degrees.to_radians().sin_cos();
        (radius.mul_add(cos, pivot.0), radius.mul_add(sin, pivot.1))
    };
    for i in 0u8..5 {
        let degrees = f32::from(i).mul_add(17.5, -125.0);
        line(
            drawing,
            polar(85.0, degrees),
            polar(97.0, degrees),
            Color::hex("#3a382f"),
            2.0,
        );
    }
    drawing.add_stroke(
        &VectorPath::arc(pivot, 90.0, (-71.0f32).to_radians(), 15.0f32.to_radians()),
        Color::hex("#c22a1e"),
        StrokeStyle::width(3.5),
    );
    line(drawing, pivot, polar(95.0, -65.0), Color::hex("#26241e"), 2.5);
    circle(
        drawing,
        pivot,
        6.0,
        Paint::radial(
            (pivot.0 - 2.0, pivot.1 - 2.0),
            9.0,
            Color::hex("#4a4e55"),
            Color::hex("#17191c"),
        ),
    );
    drawing.add_fill(
        &rect(x + 10.0, y + 10.0, width - 20.0, 50.0),
        Paint::linear(
            (x, y + 10.0),
            (x, y + 60.0),
            WHITE.with_alpha(0.16),
            WHITE.with_alpha(0.0),
        ),
        FillRule::NonZero,
    );
}

const SEGMENTS: [((f32, f32), (f32, f32)); 7] = [
    ((3.0, 0.0), (31.0, 0.0)),    // A, top
    ((34.0, 3.0), (34.0, 27.0)),  // B, top right
    ((34.0, 33.0), (34.0, 57.0)), // C, bottom right
    ((3.0, 60.0), (31.0, 60.0)),  // D, bottom
    ((0.0, 33.0), (0.0, 57.0)),   // E, bottom left
    ((0.0, 3.0), (0.0, 27.0)),    // F, top left
    ((3.0, 30.0), (31.0, 30.0)),  // G, middle
];

fn seg_digit(drawing: Weak<DrawingView>, x: f32, y: f32, mask: u8) {
    for (i, ((x0, y0), (x1, y1))) in SEGMENTS.iter().enumerate() {
        let on = mask >> i & 1 == 1;
        let color = if on {
            Color::hex("#3bff77")
        } else {
            Color::hex("#132318")
        };
        line(drawing, (x + x0, y + y0), (x + x1, y + y1), color, 6.0);
    }
}

/// The digit display: a beveled dark window and two seven segment
/// digits, the off segments faintly visible like a real LED display,
/// glowing "42" over them, under a glass gloss.
pub(super) fn seven_segment(mut drawing: Weak<DrawingView>, x: f32, y: f32) {
    // x, y is the top left of the 260 by 80 frame.
    drawing.add_fill(
        &rect(x, y, 260.0, 80.0),
        Paint::linear(
            (x, y),
            (x, y + 80.0),
            Color::hex("#101214"),
            Color::hex("#33373c"),
        ),
        FillRule::NonZero,
    );
    line(
        drawing,
        (x + 2.0, y + 79.0),
        (x + 258.0, y + 79.0),
        WHITE.with_alpha(0.1),
        2.0,
    );
    drawing.add_fill(
        &rect(x + 8.0, y + 8.0, 244.0, 64.0),
        Paint::linear(
            (x, y + 8.0),
            (x, y + 72.0),
            Color::hex("#060d08"),
            Color::hex("#0b160e"),
        ),
        FillRule::NonZero,
    );
    // 4 lights B, C, F, G. 2 lights A, B, D, E, G.
    seg_digit(drawing, x + 95.0, y + 10.0, 0b110_0110);
    seg_digit(drawing, x + 148.0, y + 10.0, 0b101_1011);
    drawing.add_fill(
        &rect(x + 8.0, y + 8.0, 244.0, 28.0),
        Paint::linear(
            (x, y + 8.0),
            (x, y + 36.0),
            WHITE.with_alpha(0.08),
            WHITE.with_alpha(0.0),
        ),
        FillRule::NonZero,
    );
}

/// The fader: a recessed track with a dark slit, and the metal thumb
/// with its grip line, riding partway down.
pub(super) fn fader(mut drawing: Weak<DrawingView>, x: f32, top: f32, bottom: f32, thumb: f32) {
    drawing.add_stroke(
        &VectorPath::polyline([(x, top), (x, bottom)]),
        Paint::linear(
            (x - 8.0, 0.0),
            (x + 8.0, 0.0),
            Color::hex("#0a0c0e"),
            Color::hex("#25282d"),
        ),
        StrokeStyle::width(16).cap(LineCap::Round),
    );
    line(drawing, (x, top), (x, bottom), Color::hex("#050607"), 4.0);
    line(
        drawing,
        (x + 7.0, top + 2.0),
        (x + 7.0, bottom - 2.0),
        WHITE.with_alpha(0.08),
        1.5,
    );

    circle(
        drawing,
        (x, thumb + 3.0),
        26.0,
        Paint::radial(
            (x, thumb + 3.0),
            26.0,
            BLACK.with_alpha(0.45),
            BLACK.with_alpha(0.0),
        ),
    );
    drawing.add_fill(
        &rect(x - 18.0, thumb - 13.0, 36.0, 26.0),
        Paint::linear(
            (x, thumb - 13.0),
            (x, thumb + 13.0),
            Color::hex("#e9ecef"),
            Color::hex("#82898f"),
        )
        .stop(Color::hex("#b9bfc5"), 0.45)
        .stop(Color::hex("#6f767d"), 0.55)
        .grain(0.06),
        FillRule::NonZero,
    );
    drawing.add_stroke(
        &rect(x - 18.0, thumb - 13.0, 36.0, 26.0),
        SHADE.with_alpha(0.5),
        StrokeStyle::width(1.5),
    );
    line(
        drawing,
        (x - 14.0, thumb),
        (x + 14.0, thumb),
        Color::hex("#14171a"),
        2.5,
    );
    line(
        drawing,
        (x - 14.0, thumb + 2.0),
        (x + 14.0, thumb + 2.0),
        WHITE.with_alpha(0.35),
        1.0,
    );
}

/// The red push button: a machined well and a glossy convex cap with a
/// hot top light, a dark lower reflection band and a crisp gloss edge.
pub(super) fn push_button(mut drawing: Weak<DrawingView>, x: f32, y: f32) {
    circle(
        drawing,
        (x, y + 3.0),
        32.0,
        Paint::radial((x, y + 3.0), 32.0, BLACK.with_alpha(0.5), BLACK.with_alpha(0.0)),
    );
    circle(
        drawing,
        (x, y),
        27.0,
        Paint::linear(
            (x, y - 27.0),
            (x, y + 27.0),
            Color::hex("#eef1f4"),
            Color::hex("#878e96"),
        ),
    );
    circle(
        drawing,
        (x, y),
        23.0,
        Paint::linear(
            (x, y - 23.0),
            (x, y + 23.0),
            Color::hex("#07080a"),
            Color::hex("#24272c"),
        ),
    );
    circle(
        drawing,
        (x, y),
        20.0,
        Paint::radial(
            (x - 5.0, y - 7.0),
            30.0,
            Color::hex("#ff6b5e"),
            Color::hex("#7e120b"),
        )
        .stop(Color::hex("#e0281a"), 0.45)
        .stop(Color::hex("#a51a10"), 0.75),
    );
    circle(
        drawing,
        (x, y),
        20.0,
        Paint::linear(
            (x, y + 6.0),
            (x, y + 19.0),
            SHADE.with_alpha(0.0),
            SHADE.with_alpha(0.35),
        ),
    );
    circle(
        drawing,
        (x, y - 6.0),
        17.0,
        Paint::linear(
            (x, y - 20.0),
            (x, y - 2.0),
            WHITE.with_alpha(0.35),
            WHITE.with_alpha(0.0),
        ),
    );
    drawing.add_stroke(
        &VectorPath::circle((x, y), 20),
        SHADE.with_alpha(0.5),
        StrokeStyle::width(1),
    );
}

/// The speaker grille: rows of drilled holes, each a dark bore with
/// the lower rim catching the light.
pub(super) fn grille(mut drawing: Weak<DrawingView>, x: f32, y: f32) {
    for row in 0u8..5 {
        for column in 0u8..10 {
            let center = (
                f32::from(column).mul_add(25.0, x),
                f32::from(row).mul_add(28.0, y),
            );
            circle(
                drawing,
                center,
                6.0,
                Paint::linear(
                    (center.0, center.1 - 6.0),
                    (center.0, center.1 + 6.0),
                    Color::hex("#040506"),
                    Color::hex("#1e2126"),
                ),
            );
            drawing.add_stroke(
                &VectorPath::arc(center, 5.5, 0.6, 1.9),
                WHITE.with_alpha(0.12),
                StrokeStyle::width(1).cap(LineCap::Round),
            );
        }
    }
}

/// Ventilation slots: horizontal slits with a shaded bore and a lit
/// lower edge.
pub(super) fn vents(drawing: Weak<DrawingView>, x: f32, y: f32, width: f32) {
    for i in 0u8..4 {
        let slit_y = f32::from(i).mul_add(25.0, y);
        line(
            drawing,
            (x, slit_y),
            (x + width, slit_y),
            Paint::linear(
                (0.0, slit_y - 4.0),
                (0.0, slit_y + 4.0),
                Color::hex("#050607"),
                Color::hex("#1c1f24"),
            ),
            8.0,
        );
        line(
            drawing,
            (x + 2.0, slit_y + 6.0),
            (x + width - 2.0, slit_y + 6.0),
            WHITE.with_alpha(0.1),
            1.5,
        );
    }
}

/// A hex bolt head: the soft shadow, the six sided head on a vertical
/// metal ramp, its outline, and the round boss in the middle.
pub(super) fn hex_bolt(mut drawing: Weak<DrawingView>, x: f32, y: f32) {
    let corners: Vec<(f32, f32)> = (0u8..6)
        .map(|i| {
            let angle = f32::from(i).mul_add(60.0, -90.0).to_radians();
            (13.0f32.mul_add(angle.cos(), x), 13.0f32.mul_add(angle.sin(), y))
        })
        .collect();
    circle(
        drawing,
        (x, y + 2.0),
        18.0,
        Paint::radial((x, y + 2.0), 18.0, BLACK.with_alpha(0.5), BLACK.with_alpha(0.0)),
    );
    drawing.add_fill(
        &VectorPath::polygon(corners.clone()),
        Paint::linear(
            (x, y - 13.0),
            (x, y + 13.0),
            Color::hex("#d8dde2"),
            Color::hex("#6f767d"),
        )
        .stop(Color::hex("#a9b0b7"), 0.5)
        .grain(0.08),
        FillRule::NonZero,
    );
    drawing.add_stroke(
        &VectorPath::polygon(corners),
        SHADE.with_alpha(0.5),
        StrokeStyle::width(1.5),
    );
    circle(
        drawing,
        (x, y),
        6.0,
        Paint::radial(
            (x - 2.0, y - 2.0),
            9.0,
            Color::hex("#c3c9cf"),
            Color::hex("#5f666e"),
        ),
    );
}

/// The brushed nameplate: a metal plate with grain, a lit top edge, a
/// shaded bottom edge and two rivets.
pub(super) fn nameplate(mut drawing: Weak<DrawingView>, x: f32, y: f32, width: f32, height: f32) {
    drawing.add_fill(
        &rect(x, y + 2.0, width, height),
        BLACK.with_alpha(0.35),
        FillRule::NonZero,
    );
    drawing.add_fill(
        &rect(x, y, width, height),
        Paint::linear(
            (x, y),
            (x, y + height),
            Color::hex("#b9bfc6"),
            Color::hex("#8f959c"),
        )
        .grain(0.1),
        FillRule::NonZero,
    );
    line(
        drawing,
        (x + 1.0, y + 1.0),
        (x + width - 1.0, y + 1.0),
        WHITE.with_alpha(0.5),
        1.5,
    );
    line(
        drawing,
        (x + 1.0, y + height - 1.0),
        (x + width - 1.0, y + height - 1.0),
        SHADE.with_alpha(0.5),
        1.5,
    );
    for rivet_x in [x + 12.0, x + width - 12.0] {
        circle(
            drawing,
            (rivet_x, y + height / 2.0),
            4.0,
            Paint::radial(
                (rivet_x - 1.0, y + height / 2.0 - 1.5),
                6.0,
                Color::hex("#f2f4f6"),
                Color::hex("#6f767d"),
            ),
        );
    }
}
