// Shared by the mesh and the sky shader, prepended to both by
// `MeshPipeline`. See `SceneView` for what the fields carry.

struct SceneView {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    ambient: vec4<f32>,
    viewport: vec4<f32>,
    irradiance: array<vec4<f32>, 9>,
}

@group(0) @binding(0)
var<uniform> view: SceneView;

@group(0) @binding(1)
var sky_cube: texture_cube<f32>;

@group(0) @binding(2)
var sky_sampler: sampler;

const PI: f32 = 3.14159265;

// Colors arrive encoded, the whole frame is encoded sRGB, see
// docs/colors.md. Lighting is linear, so decode, light, encode.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, c <= vec3<f32>(0.0031308));
}

// The highlight compression of the Khronos PBR Neutral tone mapper,
// without its black offset: a color whose brightest channel stays under
// the knee lands on screen as the hex it was written as, and only what
// would clip rolls off, desaturating towards white the way film does.
fn tonemap(color: vec3<f32>) -> vec3<f32> {
    let knee = 0.8 - 0.04;
    let desaturation = 0.15;
    let peak = max(color.r, max(color.g, color.b));
    if peak < knee {
        return color;
    }
    let d = 1.0 - knee;
    let new_peak = 1.0 - d * d / (peak + d - knee);
    let scaled = color * new_peak / peak;
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(scaled, vec3<f32>(new_peak), g);
}

// The clip space point of a fragment, from its window coordinates and
// the depth band the scene draws in.
fn fragment_ndc(frag: vec4<f32>) -> vec3<f32> {
    return vec3<f32>(
        frag.x / view.viewport.x * 2.0 - 1.0,
        1.0 - frag.y / view.viewport.y * 2.0,
        (frag.z - view.viewport.z) / view.viewport.w,
    );
}

// Where a fragment is in the world. Rebuilt here instead of carried
// from the vertex stage, an A7 draws nothing above eight components.
fn world_position(frag: vec4<f32>) -> vec3<f32> {
    let world = view.inv_view_proj * vec4<f32>(fragment_ndc(frag), 1.0);
    return world.xyz / world.w;
}

fn sky_radiance(dir: vec3<f32>, level: f32) -> vec3<f32> {
    return srgb_to_linear(textureSampleLevel(sky_cube, sky_sampler, dir, level).rgb);
}

fn encode(color: vec3<f32>) -> vec3<f32> {
    return linear_to_srgb(saturate(tonemap(color)));
}
