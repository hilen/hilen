
struct MeshInstance {
    model: mat4x4<f32>,
    normal0: vec4<f32>,
    normal1: vec4<f32>,
    normal2: vec4<f32>,
    color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    light_count: u32,
    index: u32,
    lights: vec4<u32>,
    normal_scale: f32,
    joint_base: u32,
    emissive: f32,
    cutout: f32,
    occlusion_strength: f32,
    unlit: f32,
    fade: f32,
    fade_flip: f32,
}

// See `MeshLight` for what the fourth components carry.
struct Light {
    position: vec4<f32>,
    direction: vec4<f32>,
    color: vec4<f32>,
}

@group(1) @binding(0)
var<storage, read> instances: array<MeshInstance>;

// The joint matrices of every skinned node of the frame, bind space to
// model space, each instance's run starting at its `joint_base`.
@group(1) @binding(1)
var<storage, read> joints: array<mat4x4<f32>>;

@group(2) @binding(0)
var<storage, read> lights: array<Light>;

@group(3) @binding(0)
var base_texture: texture_2d<f32>;
@group(3) @binding(1)
var base_sampler: sampler;
@group(3) @binding(2)
var normal_texture: texture_2d<f32>;
@group(3) @binding(3)
var normal_sampler: sampler;
// Plain data like the normal map, metallic in blue and roughness in
// green, the glTF packing.
@group(3) @binding(4)
var metallic_roughness_texture: texture_2d<f32>;
@group(3) @binding(5)
var metallic_roughness_sampler: sampler;
// The share of the light from all around in red.
@group(3) @binding(6)
var occlusion_texture: texture_2d<f32>;
@group(3) @binding(7)
var occlusion_sampler: sampler;

struct Vertex {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    // Four encoded sRGB bytes, red in the lowest, see `Vertex3D::color`.
    @location(14) color: u32,
}

struct Instance {
    @location(3) model0: vec4<f32>,
    @location(4) model1: vec4<f32>,
    @location(5) model2: vec4<f32>,
    @location(6) model3: vec4<f32>,
    @location(7) normal0: vec4<f32>,
    @location(8) normal1: vec4<f32>,
    @location(9) normal2: vec4<f32>,
    @location(10) index: u32,
    @location(11) joint_base: u32,
}

// The second vertex buffer of a skinned mesh, see `SkinVertex`.
struct SkinVertex {
    @location(12) joints: vec4<u32>,
    @location(13) weights: vec4<f32>,
}

// Seven components cross the stage boundary. An A7 draws nothing above
// eight, see docs/ios.md, so the material and the light list stay in
// the storage buffers, the world position is rebuilt from the depth,
// only the index of the instance comes along, and the vertex color
// comes packed and flat, one per triangle.
// Invariant, a cut out node is drawn twice and the second draw tests its
// depth for equal.
struct VertexOutput {
    @invariant @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) @interpolate(flat) instance: u32,
    @location(3) @interpolate(flat) color: u32,
}

fn place(pos: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>, color: u32, instance: Instance) -> VertexOutput {
    let model = mat4x4<f32>(instance.model0, instance.model1, instance.model2, instance.model3);
    let normal_matrix = mat3x3<f32>(instance.normal0.xyz, instance.normal1.xyz, instance.normal2.xyz);

    var out: VertexOutput;
    out.pos = view.view_proj * vec4<f32>(model_offset(model, pos), 1.0);
    out.uv = uv;
    out.normal = normalize(normal_matrix * normal);
    out.instance = instance.index;
    out.color = color;
    return out;
}

@vertex
fn v_main(vertex: Vertex, instance: Instance) -> VertexOutput {
    return place(vertex.pos, vertex.normal, vertex.uv, vertex.color, instance);
}

// A mesh whose colors fade across a triangle, see
// `Model::from_smooth_mesh`. The color takes 4 components where the flat
// one takes 1, and the uv gives up its 2, so eight components cross the
// stage boundary, all an A7 draws, see docs/ios.md.
struct SmoothOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) @interpolate(flat) instance: u32,
    @location(2) color: vec4<f32>,
}

@vertex
fn v_smooth(vertex: Vertex, instance: Instance) -> SmoothOutput {
    let placed = place(vertex.pos, vertex.normal, vec2<f32>(0.0), vertex.color, instance);
    var out: SmoothOutput;
    out.pos = placed.pos;
    out.normal = placed.normal;
    out.instance = placed.instance;
    out.color = unpack4x8unorm(vertex.color);
    return out;
}

// The blend of the four joint matrices that move this vertex. A joint
// turns and moves, it does not squash, so its upper left turns the
// normal as it is.
fn skin_matrix(skin: SkinVertex, base: u32) -> mat4x4<f32> {
    return skin.weights.x * joints[base + skin.joints.x]
        + skin.weights.y * joints[base + skin.joints.y]
        + skin.weights.z * joints[base + skin.joints.z]
        + skin.weights.w * joints[base + skin.joints.w];
}

@vertex
fn v_skinned(vertex: Vertex, instance: Instance, skin: SkinVertex) -> VertexOutput {
    let matrix = skin_matrix(skin, instance.joint_base);
    let pos = (matrix * vec4<f32>(vertex.pos, 1.0)).xyz;
    let normal = mat3x3<f32>(matrix[0].xyz, matrix[1].xyz, matrix[2].xyz) * vertex.normal;
    return place(pos, normal, vertex.uv, vertex.color, instance);
}

// One blade of a grass patch, see `GrassBlade`.
struct Blade {
    @location(3) place: vec4<f32>,
    @location(4) size: f32,
    @location(5) tint: u32,
    @location(6) slot: u32,
    @location(7) normal: u32,
}

// How much of the ground's normal a blade's normal takes. Lit by its own
// face a thin blade flips from bright to dark as it turns, lit nearly
// like the ground under it a field reads as 1 surface.
const GRASS_UP: f32 = 0.7;
// The units around the player's trail that grass leans away in.
const GRASS_PUSH_REACH: f32 = 1.0;
// Grass closer to the camera than this is not drawn, so a blade never
// fills the view.
const GRASS_NEAR: f32 = 2.0;

// A blade of a grass patch with its wind and push. The patch draws
// through its own bind of group 1:
// `instances` holds 1 instance per patch slot, read by the fragment stage
// as for any node, and `joints` 1 matrix per slot, see `Grass`. Its first
// column is the fade: where blades thin out from, the length of the
// thinning, 1 over the band a blade shrinks over and the blade count. The
// second is the 4 phases of the sway, the third the sway distance, the
// push distance and 1 over the height of a full bend, the last the
// patch's origin.
@vertex
fn v_grass(vertex: Vertex, blade: Blade, @builtin(instance_index) rank: u32) -> VertexOutput {
    // 2 matrices a patch: its numbers, and the turn of its field's own
    // axes, which everything below works in until the end.
    let field = joints[blade.slot * 2u];
    let turn4 = joints[blade.slot * 2u + 1u];
    let field_turn = mat3x3<f32>(turn4[0].xyz, turn4[1].xyz, turn4[2].xyz);
    let to_field = transpose(field_turn);
    let fade = field[0];
    let phases = field[1];
    let sway = field[2];
    // The middle of the patch from the camera, and its place in the field.
    let patch_middle = field[3].xyz - view.camera_pos.xyz;
    let patch_in_field = vec2<f32>(field[3].w, sway.w);
    let root = patch_middle + field_turn * blade.place.xyz;

    // The blades of a patch lie in random order, so the first ones stay
    // the farthest out and the field thins evenly. Each shrinks away over
    // a band before its own distance, so none pops.
    let share = 1.0 - f32(rank) / fade.w;
    let gone = fade.x + share * fade.y;
    let grown = saturate((gone - length(root)) * fade.z);

    let local = vertex.pos * blade.size * grown;
    let turn = vec2<f32>(cos(blade.place.w), sin(blade.place.w));
    var pos = vec3<f32>(turn.x * local.x + turn.y * local.z, local.y, turn.x * local.z - turn.y * local.x);

    // The turn that takes straight up onto the ground's normal, so a blade
    // stands out of a slope. Its columns are where x, y and z go.
    let flat = unpack2x16snorm(blade.normal);
    let rise = sqrt(max(1.0 - dot(flat, flat), 0.0));
    let k = 1.0 / (1.0 + rise);
    let tilt = mat3x3<f32>(
        vec3<f32>(1.0 - flat.x * flat.x * k, -flat.x, -flat.x * flat.y * k),
        vec3<f32>(flat.x, rise, flat.y),
        vec3<f32>(-flat.x * flat.y * k, -flat.y, 1.0 - flat.y * flat.y * k),
    );

    var bent = tilt * pos;
    let rest = length(bent);
    // How much a point bends, its height in the mesh over the height of a
    // full bend, a straight line, so the root stays.
    let bend = saturate(vertex.pos.y * sway.z);

    // The wind: a steady lean along it and a gust, a wave running along
    // the wind times 3 sines across and along it.
    // The wind laid onto the field's ground.
    let wind_flat = (to_field * view.grass_wind.xyz).xz;
    let wind = select(vec2<f32>(0.0), normalize(wind_flat), dot(wind_flat, wind_flat) > 1e-8);
    let strength = view.grass_wind.w;
    let world = patch_in_field + blade.place.xz + bent.xz;
    let across = dot(world, vec2<f32>(-wind.y, wind.x));
    let along = dot(world, wind);
    let wave = vec3<f32>(across, across, along) * vec3<f32>(-0.3, -0.432, -0.2);
    let gust = sin(phases.x + wave.z);
    let waves = sin(phases.yzw + wave);
    let rough = abs(2.0 * waves.x * waves.y) * 0.75 + 0.25;
    let wind3 = vec3<f32>(wind.x, 0.0, wind.y);
    let swing = wind3 * (gust * waves.z * sway.x * bend * strength * rough);
    let steady = wind3 * (sway.x * bend * strength * 0.4);
    var shift = swing * 0.5 + steady;
    shift.y -= 0.5 * length(shift.xz);
    bent += shift;

    // The push: within reach of the line from where the feet were to
    // where they are, so a walk leaves a trail, out from the player and
    // down, only above the feet.
    if view.grass_push.w > 0.5 {
        let point = blade.place.xyz + bent;
        let feet = to_field * (view.grass_push.xyz - patch_middle);
        let trail_end = (to_field * (view.grass_trail.xyz - patch_middle)).xz;
        let from_feet = point.xz - feet.xz;
        let trail = trail_end - feet.xz;
        let trail_length = length(trail);
        var apart = length(from_feet);
        if trail_length >= 0.01 {
            let back = trail / trail_length;
            let behind = dot(back, from_feet);
            if behind > trail_length {
                apart = length(trail_end - point.xz);
            } else if behind >= 0.0 {
                apart = abs(dot(vec2<f32>(back.y, -back.x), from_feet));
            }
        }
        let push = (1.0 - min(apart / GRASS_PUSH_REACH, 1.0)) * bend * sway.y;
        let out_of_way = blade.place.xz - feet.xz;
        let away = out_of_way / max(length(out_of_way), 1e-4);
        if feet.y < point.y {
            bent += vec3<f32>(away.x * push, -push, away.y * push);
        }
    }

    // Every point keeps its distance from the root, so a blade swings as
    // if hinged there and never stretches.
    let reach = length(bent);
    if reach > 1e-5 {
        bent *= rest / reach;
    }

    let n = vec3<f32>(turn.x * vertex.normal.x + turn.y * vertex.normal.z, vertex.normal.y, turn.x * vertex.normal.z - turn.y * vertex.normal.x);

    var out: VertexOutput;
    out.pos = view.view_proj * vec4<f32>(root + field_turn * bent, 1.0);
    out.uv = vertex.uv;
    out.normal = normalize(field_turn * mix(tilt * n, tilt[1], GRASS_UP));
    out.instance = blade.slot;
    out.color = pack4x8unorm(unpack4x8unorm(vertex.color) * unpack4x8unorm(blade.tint));
    return out;
}

// The Filament mobile model: Lambert diffuse, GGX distribution, the
// fast height correlated Smith visibility and the Schlick Fresnel.
// `roughness` is the perceptual one squared.
fn d_ggx(noh: f32, roughness: f32) -> f32 {
    let a = noh * roughness;
    let k = roughness / (1.0 - noh * noh + a * a);
    return k * k / PI;
}

fn v_smith_ggx_fast(nov: f32, nol: f32, roughness: f32) -> f32 {
    return 0.5 / mix(2.0 * nol * nov, nol + nov, roughness);
}

fn f_schlick(f0: vec3<f32>, f90: f32, voh: f32) -> vec3<f32> {
    let f = pow(1.0 - voh, 5.0);
    return f0 + (vec3<f32>(f90) - f0) * f;
}

// The second half of the split sum, the environment BRDF, as the fit
// Karis published for mobile, so no lookup table is needed.
fn env_brdf(f0: vec3<f32>, perceptual: f32, nov: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = perceptual * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * nov)) * r.x + r.y;
    let ab = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return f0 * ab.x + ab.y;
}

// The diffuse light the sky sends a surface with this normal, from the
// nine harmonics `hilen_pixels::irradiance` projected.
fn sky_irradiance(n: vec3<f32>) -> vec3<f32> {
    let c = view.irradiance;
    return c[0].rgb * 0.282095
        + c[1].rgb * (0.488603 * n.y)
        + c[2].rgb * (0.488603 * n.z)
        + c[3].rgb * (0.488603 * n.x)
        + c[4].rgb * (1.092548 * n.x * n.y)
        + c[5].rgb * (1.092548 * n.y * n.z)
        + c[6].rgb * (0.315392 * (3.0 * n.z * n.z - 1.0))
        + c[7].rgb * (1.092548 * n.x * n.z)
        + c[8].rgb * (0.546274 * (n.x * n.x - n.y * n.y));
}

// A tangent frame from the screen derivatives of the position and the
// uv, so a normal map needs no tangent attribute.
fn perturb_normal(n: vec3<f32>, p: vec3<f32>, uv: vec2<f32>, map: vec3<f32>) -> vec3<f32> {
    let dp1 = dpdx(p);
    let dp2 = dpdy(p);
    let duv1 = dpdx(uv);
    let duv2 = dpdy(uv);
    let dp2perp = cross(dp2, n);
    let dp1perp = cross(n, dp1);
    let t = dp2perp * duv1.x + dp1perp * duv2.x;
    let b = dp2perp * duv1.y + dp1perp * duv2.y;
    let longest = max(dot(t, t), dot(b, b));
    // A mesh without uvs has no tangent frame, and dividing by its zero
    // length turns the normal into NaN and the surface black.
    if longest <= 0.0 {
        return n;
    }
    let invmax = inverseSqrt(longest);
    return normalize(t * invmax * map.x + b * invmax * map.y + n * map.z);
}

struct Surface {
    normal: vec3<f32>,
    to_eye: vec3<f32>,
    nov: f32,
    diffuse: vec3<f32>,
    f0: vec3<f32>,
    f90: f32,
    roughness: f32,
    // 1 for `Material::dust`.
    dust: f32,
}

// Light arriving from `to_light` with `radiance`, both already scaled
// by the light's falloff. A light's intensity is the brightness of a
// white matte surface facing it, so the Lambert term carries no 1 / PI
// and the specular one carries a PI to stay in the same units.
fn shade(s: Surface, to_light: vec3<f32>, radiance: vec3<f32>) -> vec3<f32> {
    let nol = dot(s.normal, to_light);
    if nol <= 0.0 {
        return vec3<f32>(0.0);
    }
    let h = normalize(to_light + s.to_eye);
    let noh = saturate(dot(s.normal, h));
    let loh = saturate(dot(to_light, h));

    let d = d_ggx(noh, s.roughness);
    let v = v_smith_ggx_fast(s.nov, nol, s.roughness);
    let f = f_schlick(s.f0, s.f90, loh);

    // Lommel-Seeliger for dust, 1 where Lambert is 1, see `Material::dust`.
    let diffuse_light = select(nol, 2.0 * nol / (nol + s.nov), s.dust > 0.5);
    return (s.diffuse * diffuse_light + PI * d * v * f * nol) * radiance;
}

// How much the sun reaches a point of a cascade's map, 0 in shadow and
// 1 in the light, from the four texels around where it lands, each
// compared on its own and blended by the distance to them.
fn cascade_shadow(layer: i32, uv: vec2<f32>, depth: f32) -> f32 {
    let size = vec2<f32>(textureDimensions(shadow_map));
    let texel = uv * size - 0.5;
    let base = floor(texel);
    let f = texel - base;
    let last = vec2<i32>(size) - 1;
    let i = vec2<i32>(base);
    let lit00 = f32(depth <= textureLoad(shadow_map, clamp(i, vec2<i32>(0), last), layer, 0));
    let lit10 = f32(depth <= textureLoad(shadow_map, clamp(i + vec2<i32>(1, 0), vec2<i32>(0), last), layer, 0));
    let lit01 = f32(depth <= textureLoad(shadow_map, clamp(i + vec2<i32>(0, 1), vec2<i32>(0), last), layer, 0));
    let lit11 = f32(depth <= textureLoad(shadow_map, clamp(i + vec2<i32>(1, 1), vec2<i32>(0), last), layer, 0));
    return mix(mix(lit00, lit10, f.x), mix(lit01, lit11, f.x), f.y);
}

// Texels a receiver is pushed out along its normal before the lookup,
// on a surface edge on to the sun, so a lit face does not shadow itself
// where the map's texels straddle it. A face square to the sun needs
// none, so the push fades with the facing and a floor under a high sun
// keeps its shadows against its posts.
const NORMAL_OFFSET: f32 = 1.5;

// The share of a map's width at its edge over which a receiver blends
// into the next cascade, so the seam between a fine map and a coarse
// one is not a line where the shadows change.
const CASCADE_BLEND: f32 = 0.1;

// The most texels of a cascade one pixel may cover on the surface it
// shades. A map finer than that aliases a thin shadow into dots, the
// way a texture without mips shimmers, since the pixels straddle the
// caster's footprint in the map.
const PIXEL_TEXELS: f32 = 2.0;

// Where `p` lands in one cascade's map: the texel coordinates, the
// depth to compare, and how far inside the map it is, 1 past the blend
// band, falling to 0 at the edge, below 0 outside the map.
struct CascadePoint {
    uv: vec2<f32>,
    depth: f32,
    inside: f32,
}

// `offset` is the point from the camera, what the cascade maps take.
fn cascade_place(layer: i32, offset: vec3<f32>, n: vec3<f32>, nol: f32) -> CascadePoint {
    let texel = view.sun_texel[layer];
    let pushed = offset + n * texel * NORMAL_OFFSET * (1.0 - nol);
    let clip = view.sun_view_proj[layer] * vec4<f32>(pushed, 1.0);
    let ndc = clip.xyz / clip.w;
    var point: CascadePoint;
    point.uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    // One texel of depth bias on top of the push, against acne on a
    // face at a slant to the sun.
    point.depth = ndc.z - texel * view.sun_depth[layer];
    // The filter reads a texel past where the point lands.
    let edge = 2.0 / f32(textureDimensions(shadow_map).x);
    let inside = min(min(point.uv.x, 1.0 - point.uv.x), min(point.uv.y, 1.0 - point.uv.y)) - edge;
    point.inside = select(saturate(inside / CASCADE_BLEND), -1.0, inside < 0.0 || ndc.z > 1.0);
    return point;
}

// The shadow of `p` in a cascade's map, blended into the next map that
// holds the point when `p` sits in the edge band.
fn cascade_blend(layer: i32, point: CascadePoint, offset: vec3<f32>, n: vec3<f32>, nol: f32) -> f32 {
    let shadow = cascade_shadow(layer, point.uv, point.depth);
    if point.inside >= 1.0 {
        return shadow;
    }
    for (var next = layer + 1; next < SHADOW_CASCADES; next++) {
        let other = cascade_place(next, offset, n, nol);
        if other.inside >= 0.0 {
            return mix(cascade_shadow(next, other.uv, other.depth), shadow, point.inside);
        }
    }
    return shadow;
}

// Whether the sun reaches `p`, 0 in shadow and 1 in the light. The
// nearest cascade whose map holds the point answers, unless its texels
// are finer than the pixel, then the answer moves to the coarsest map
// that still holds the point. A slice's map holds only its own slice
// and a little around it, so the last map does not hold what the near
// ones do, and a pixel too coarse for every map that holds it keeps
// the coarsest of those. Outside every map the sun reaches.
fn sun_shadow(p: vec3<f32>, offset: vec3<f32>, n: vec3<f32>) -> f32 {
    if view.sun_color.w < 0.5 {
        return 1.0;
    }
    let facing = max(abs(dot(n, to_eye_of(p))), 0.1);
    let footprint = pixel_size_at(p) / facing;
    let nol = saturate(dot(n, -view.sun_dir.xyz));
    var held = -1;
    var held_point: CascadePoint;
    for (var layer = 0; layer < SHADOW_CASCADES; layer++) {
        let point = cascade_place(layer, offset, n, nol);
        if point.inside < 0.0 {
            continue;
        }
        if view.sun_texel[layer] * PIXEL_TEXELS < footprint && layer + 1 < SHADOW_CASCADES {
            held = layer;
            held_point = point;
            continue;
        }
        return cascade_blend(layer, point, offset, n, nol);
    }
    if held >= 0 {
        return cascade_blend(held, held_point, offset, n, nol);
    }
    return 1.0;
}

// The color seen through the fog between the camera and `p`, the fog
// taking over from `fog_range.x` units away over the length of its
// fade. Linear light, the fog color is tonemapped with the rest.
fn fogged(color: vec3<f32>, p: vec3<f32>) -> vec3<f32> {
    // The air's haze stands in for the fog.
    if has_air() {
        return air_haze(color, p - view.camera_pos.xyz);
    }
    if view.fog_color.w < 0.5 {
        return color;
    }
    let distance = length(p - view.camera_pos.xyz);
    let amount = saturate((distance - view.fog_range.x) * view.fog_range.y);
    return mix(color, view.fog_color.rgb, amount);
}

fn light_index(packed: vec4<u32>, slot: u32) -> u32 {
    return (packed[slot / 2u] >> ((slot % 2u) * 16u)) & 0xffffu;
}

// The lit color of a fragment and its alpha.
// What the textures say about a fragment. Everything with a derivative is
// in here, so a node can drop a fragment after this and pay for no light,
// see `f_grass`. The discard comes after every sample, the derivatives of a
// dropped fragment are not to be trusted.
struct Sampled {
    base:       vec3<f32>,
    alpha:      f32,
    metallic:   f32,
    perceptual: f32,
    occlusion:  f32,
    offset:     vec3<f32>,
    world_pos:  vec3<f32>,
    flat:       vec3<f32>,
    normal:     vec3<f32>,
}

fn sampled(in: VertexOutput) -> Sampled {
    return sample_maps(in.pos, in.uv, in.normal, in.instance, unpack4x8unorm(in.color));
}

// A smooth mesh has no uv, so it reads the first texel of each map.
fn sampled_smooth(in: SmoothOutput) -> Sampled {
    return sample_maps(in.pos, vec2<f32>(0.0), in.normal, in.instance, in.color);
}

fn lit(in: VertexOutput) -> vec4<f32> {
    return shade_sampled(sampled(in), in.instance);
}

fn lit_smooth(in: SmoothOutput) -> vec4<f32> {
    return shade_sampled(sampled_smooth(in), in.instance);
}

// `tint` is the vertex color, encoded sRGB like every color.
fn sample_maps(pos: vec4<f32>, uv: vec2<f32>, normal: vec3<f32>, index: u32, tint: vec4<f32>) -> Sampled {
    let instance = instances[index];
    let texel = textureSample(base_texture, base_sampler, uv);
    var map = textureSample(normal_texture, normal_sampler, uv).xyz * 2.0 - 1.0;
    map = vec3<f32>(map.xy * instance.normal_scale, map.z);

    // The decode of a full channel lands a hair under one in f32, which
    // moved pixels of every mesh when vertex colors came in, and white
    // is what every mesh without vertex colors carries.
    let tint_linear = select(srgb_to_linear(tint.rgb), vec3<f32>(1.0), tint.rgb == vec3<f32>(1.0));
    let base = srgb_to_linear(instance.color.rgb) * tint_linear * srgb_to_linear(texel.rgb);
    let alpha = instance.color.a * tint.a * texel.a;
    let metal_rough = textureSample(metallic_roughness_texture, metallic_roughness_sampler, uv);
    let metallic = instance.metallic * metal_rough.b;
    // Below this a highlight collapses to a few pixels of noise.
    let perceptual = clamp(instance.roughness * metal_rough.g, 0.045, 1.0);
    let occlusion_texel = textureSample(occlusion_texture, occlusion_sampler, uv).r;
    let occlusion = 1.0 + instance.occlusion_strength * (occlusion_texel - 1.0);

    let offset = camera_offset(pos);
    let world_pos = view.camera_pos.xyz + offset;
    let flat = normalize(normal);
    let bent = perturb_normal(flat, world_pos, uv, map);
    return Sampled(base, alpha, metallic, perceptual, occlusion, offset, world_pos, flat, bent);
}

fn shade_sampled(in: Sampled, index: u32) -> vec4<f32> {
    let instance = instances[index];
    let base = in.base;
    let metallic = in.metallic;
    let perceptual = in.perceptual;
    let offset = in.offset;
    let world_pos = in.world_pos;

    var s: Surface;
    s.normal = in.normal;
    s.to_eye = to_eye_of(world_pos);
    s.nov = max(dot(s.normal, s.to_eye), 1e-4);
    s.diffuse = base * (1.0 - metallic);
    s.f0 = mix(vec3<f32>(0.04), base, metallic);
    s.f90 = saturate(dot(s.f0, vec3<f32>(50.0 * 0.33)));
    s.roughness = perceptual * perceptual;
    s.dust = instance.normal0.w;

    var color: vec3<f32>;
    if view.ambient.w > 0.5 {
        // The sky lights the diffuse through its harmonics and the
        // reflection through the mip that matches the roughness.
        let reflected = reflect(-s.to_eye, s.normal);
        // The air has no cube to reflect, its 9 numbers stand in, a
        // blurred sky.
        let specular = select(sky_radiance(reflected, perceptual * 5.0), sky_irradiance(reflected), has_air());
        color = sky_irradiance(s.normal) * s.diffuse + specular * env_brdf(s.f0, perceptual, s.nov);
    } else {
        // A flat ambient stands in for a sky. Its diffuse is the Lambert
        // term, its reflection the base reflectance.
        color = view.ambient.rgb * (s.diffuse + s.f0);
    }
    // Occlusion darkens only the light from all around, never the sun
    // or a lamp, as glTF asks.
    color *= in.occlusion;

    let to_sun = -view.sun_dir.xyz;
    color += shade(s, to_sun, view.sun_color.rgb) * sun_shadow(world_pos, offset, in.flat)
        * light_through(offset, to_sun, view.occlusion.y) * air_light_at(offset, to_sun);

    for (var slot = 0u; slot < instance.light_count; slot++) {
        let light = lights[light_index(instance.lights, slot)];
        if light.position.w < 0.0 {
            // Directional, full strength from everywhere a ball does not
            // hide it.
            let to_light = light.position.xyz;
            let through = light_through(offset, to_light, light.direction.w) * air_light_at(offset, to_light);
            color += shade(s, to_light, light.color.rgb * through);
            continue;
        }

        let to_light = light.position.xyz - world_pos;
        let distance2 = max(dot(to_light, to_light), 1e-4);
        let l = to_light * inverseSqrt(distance2);

        // The inverse square, pulled to zero at the range.
        let ranged = distance2 * light.position.w;
        let window = saturate(1.0 - ranged * ranged);
        var attenuation = window * window / distance2;

        let cone = saturate(dot(light.direction.xyz, -l) * light.direction.w + light.color.w);
        attenuation *= cone * cone;

        color += shade(s, l, light.color.rgb * attenuation);
    }

    // Its own glow, the base color the lights would have shown.
    color += base * instance.emissive;

    // An unlit node shows its base color alone. A select and not an early
    // return: the flag differs per instance, and the derivatives above
    // need every fragment to take the same path.
    color = select(color, base, instance.unlit > 0.5);

    return vec4<f32>(encode(fogged(color, world_pos)), in.alpha);
}

// A translucent node, blended by its alpha.
@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return lit(in);
}

// An opaque node replaces the target, so it writes a full alpha. The
// alpha of its texture would leave a hole in a scene picture.
@fragment
fn f_opaque(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(lit(in).rgb, 1.0);
}

// A cut out node, see `Material::cutout`, or a fading one, see
// `Material::fade`. Its own entry, so a plain opaque draw carries no
// discard. The color it returns goes nowhere, `f_opaque` lights what
// stays, see `MeshDraw::CutoutDepth`.
@fragment
fn f_cutout_depth(in: VertexOutput) -> @location(0) vec4<f32> {
    let instance = instances[in.instance];
    if coverage(in) < instance.cutout || faded_out(in.pos.xy, instance.fade, instance.fade_flip) {
        discard;
    }
    return vec4<f32>(0.0);
}

// The 3 entries above for a mesh with fading colors.
@fragment
fn f_smooth_main(in: SmoothOutput) -> @location(0) vec4<f32> {
    return lit_smooth(in);
}

@fragment
fn f_smooth_opaque(in: SmoothOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(lit_smooth(in).rgb, 1.0);
}

@fragment
fn f_smooth_cutout(in: SmoothOutput) -> @location(0) vec4<f32> {
    let surface = sampled_smooth(in);
    let instance = instances[in.instance];
    if surface.alpha < instance.cutout || faded_out(in.pos.xy, instance.fade, instance.fade_flip) {
        discard;
    }
    return vec4<f32>(shade_sampled(surface, in.instance).rgb, 1.0);
}

// A blade of grass, cut out and dropped near the camera.
@fragment
fn f_grass(in: VertexOutput) -> @location(0) vec4<f32> {
    let surface = sampled(in);
    let instance = instances[in.instance];
    if surface.alpha < instance.cutout || length(camera_offset(in.pos)) < GRASS_NEAR {
        discard;
    }
    return vec4<f32>(shade_sampled(surface, in.instance).rgb, 1.0);
}

// The 24 bits a depth photo has for a distance, spread on a log scale
// from 1 mm to 10 km, see `scene/photo.rs`, which reads them back. A
// log scale keeps the same share of precision at every distance, and a
// pixel where the multisample resolve mixed 2 surfaces still lands
// between them.
const DEPTH_LOG_NEAR: f32 = -9.965784;
const DEPTH_LOG_FAR: f32 = 13.287712;
const DEPTH_STEPS: f32 = 16777214.0;

// The distance from the camera to the fragment along the line of sight
// as 3 bytes in red, green and blue. The full 24 bits, white, is left
// for a pixel with nothing on it.
fn depth_color(pos: vec4<f32>) -> vec4<f32> {
    let along = max(dot(camera_offset(pos), view.eye.xyz), 1e-6);
    let part = saturate((log2(along) - DEPTH_LOG_NEAR) / (DEPTH_LOG_FAR - DEPTH_LOG_NEAR));
    let steps = u32(round(part * DEPTH_STEPS));
    let bytes = vec3<u32>(steps >> 16u, (steps >> 8u) & 255u, steps & 255u);
    return vec4<f32>(vec3<f32>(bytes) / 255.0, 1.0);
}

// The alpha `lit` gives a fragment, without the light.
fn coverage(in: VertexOutput) -> f32 {
    let texel = textureSample(base_texture, base_sampler, in.uv);
    return instances[in.instance].color.a * unpack4x8unorm(in.color).a * texel.a;
}

@fragment
fn f_depth(in: VertexOutput) -> @location(0) vec4<f32> {
    return depth_color(in.pos);
}

@fragment
fn f_depth_cutout(in: VertexOutput) -> @location(0) vec4<f32> {
    let instance = instances[in.instance];
    if coverage(in) < instance.cutout || faded_out(in.pos.xy, instance.fade, instance.fade_flip) {
        discard;
    }
    return depth_color(in.pos);
}

// `f_depth_cutout` for a mesh with fading colors. A smooth node that is
// not cut out needs no entry of its own, the plain `f_depth` reads only
// the position.
@fragment
fn f_smooth_depth_cutout(in: SmoothOutput) -> @location(0) vec4<f32> {
    let instance = instances[in.instance];
    let texel = textureSample(base_texture, base_sampler, vec2<f32>(0.0));
    if instance.color.a * in.color.a * texel.a < instance.cutout || faded_out(in.pos.xy, instance.fade, instance.fade_flip) {
        discard;
    }
    return depth_color(in.pos);
}

// An edge lies on the faces it borders, at their depth, so a line drawn
// there loses to them on every other pixel. Each fragment moves this
// share of its distance toward the camera. The reversed depth shrinks
// like 1 over the distance, so a share of it is the same share of the
// distance. Depth bias would do this, WebGPU allows it only for
// triangles.
const WIREFRAME_PULL: f32 = 0.002;
// White, encoded like the frame, unlit and unfogged.
const WIREFRAME_COLOR: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 1.0);

struct WireframeOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn f_wireframe(in: VertexOutput) -> WireframeOutput {
    var out: WireframeOutput;
    out.color = WIREFRAME_COLOR;
    out.depth = min(in.pos.z * (1.0 + WIREFRAME_PULL), 1.0);
    return out;
}
