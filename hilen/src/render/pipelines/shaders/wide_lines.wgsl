// The lines of a scene, each a quad `width` pixels wide on the screen
// at any distance, one instance per line. Six vertices make the quad,
// the vertex index picks the end and the side of each. The ends are
// square and stick out half the width, so the lines of a polyline join
// with no gap. Colors are encoded like the frame and nothing lights
// them. The view is `scene_common.wgsl`, prepended.

struct WideLine {
    @location(0) start: vec3<f32>,
    @location(1) width: f32,
    @location(2) end: vec3<f32>,
    @location(3) pad: f32,
    @location(4) color: vec4<f32>,
}

struct LineOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

// A line sits nearer to the eye along the line of sight, so it stays on
// its pixel and a face it lies on does not cut it: a thousandth of its
// distance, plus SLANT pixels of the world at that distance for each
// pixel from its middle to its edge. A floor seen at a low angle comes
// nearer by a few of its pixels with every pixel down the screen, and
// without the second part it hid the lower half of a line lying on it.
// SLANT holds a floor down to about 22 degrees. The cost is an edge just
// behind a face, it shows for a few pixels where it leaves a corner.
const NEARER: f32 = 0.001;
const SLANT: f32 = 2.5;

fn pulled(p: vec3<f32>, width: f32) -> vec3<f32> {
    var distance = length(view.camera_pos.xyz - p);
    if view.eye.w > 0.0 {
        distance = dot(p - view.camera_pos.xyz, view.eye.xyz);
    }
    let pull = distance * NEARER + pixel_size_at(p) * (width * 0.5 + 0.5) * SLANT;
    return p + to_eye_of(p) * pull;
}

@vertex
fn v_main(@builtin(vertex_index) index: u32, line: WideLine) -> LineOutput {
    var out: LineOutput;
    out.color = line.color;

    var a = view.view_proj * vec4<f32>(pulled(line.start, line.width) - view.camera_pos.xyz, 1.0);
    var b = view.view_proj * vec4<f32>(pulled(line.end, line.width) - view.camera_pos.xyz, 1.0);

    // A point behind the near plane has no place on the screen, the line
    // is cut there. Wholly behind it, the line is dropped.
    if a.z < 0.0 && b.z < 0.0 {
        out.pos = vec4<f32>(0.0, 0.0, -1.0, 1.0);
        return out;
    }
    if a.z < 0.0 {
        a = mix(a, b, a.z / (a.z - b.z));
    }
    if b.z < 0.0 {
        b = mix(b, a, b.z / (b.z - a.z));
    }

    // In pixels from the middle of the view, y up.
    let half = view.viewport.xy * 0.5;
    let screen_a = a.xy / a.w * half;
    let screen_b = b.xy / b.w * half;
    var along = screen_b - screen_a;
    if length(along) < 1e-6 {
        along = vec2<f32>(1.0, 0.0);
    }
    along = normalize(along);
    let across = vec2<f32>(-along.y, along.x);

    let ends = array<f32, 6>(0.0, 1.0, 1.0, 0.0, 1.0, 0.0);
    let sides = array<f32, 6>(-1.0, -1.0, 1.0, -1.0, 1.0, 1.0);
    let at_end = ends[index];
    let point = mix(a, b, at_end);
    let offset = (across * sides[index] + along * (at_end * 2.0 - 1.0)) * line.width * 0.5;
    out.pos = point + vec4<f32>(offset / half * point.w, 0.0, 0.0);
    return out;
}

@fragment
fn f_main(in: LineOutput) -> @location(0) vec4<f32> {
    return in.color;
}
