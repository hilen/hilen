
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
    padding: vec2<u32>,
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

struct Vertex {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
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

// Six components cross the stage boundary. An A7 draws nothing above
// eight, see docs/ios.md, so the material and the light list stay in
// the storage buffers, the world position is rebuilt from the depth,
// and only the index of the instance comes along.
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) @interpolate(flat) instance: u32,
}

fn place(pos: vec3<f32>, normal: vec3<f32>, uv: vec2<f32>, instance: Instance) -> VertexOutput {
    let model = mat4x4<f32>(instance.model0, instance.model1, instance.model2, instance.model3);
    let normal_matrix = mat3x3<f32>(instance.normal0.xyz, instance.normal1.xyz, instance.normal2.xyz);

    var out: VertexOutput;
    out.pos = view.view_proj * model * vec4<f32>(pos, 1.0);
    out.uv = uv;
    out.normal = normalize(normal_matrix * normal);
    out.instance = instance.index;
    return out;
}

@vertex
fn v_main(vertex: Vertex, instance: Instance) -> VertexOutput {
    return place(vertex.pos, vertex.normal, vertex.uv, instance);
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
    return place(pos, normal, vertex.uv, instance);
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

    return (s.diffuse + PI * d * v * f) * radiance * nol;
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

fn cascade_place(layer: i32, p: vec3<f32>, n: vec3<f32>, nol: f32) -> CascadePoint {
    let texel = view.sun_texel[layer];
    let pushed = p + n * texel * NORMAL_OFFSET * (1.0 - nol);
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
fn cascade_blend(layer: i32, point: CascadePoint, p: vec3<f32>, n: vec3<f32>, nol: f32) -> f32 {
    let shadow = cascade_shadow(layer, point.uv, point.depth);
    if point.inside >= 1.0 {
        return shadow;
    }
    for (var next = layer + 1; next < SHADOW_CASCADES; next++) {
        let other = cascade_place(next, p, n, nol);
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
fn sun_shadow(p: vec3<f32>, n: vec3<f32>) -> f32 {
    if view.sun_color.w < 0.5 {
        return 1.0;
    }
    let to_eye = view.camera_pos.xyz - p;
    let facing = max(abs(dot(n, normalize(to_eye))), 0.1);
    let footprint = length(to_eye) * view.camera_pos.w / view.viewport.y / facing;
    let nol = saturate(dot(n, -view.sun_dir.xyz));
    var held = -1;
    var held_point: CascadePoint;
    for (var layer = 0; layer < SHADOW_CASCADES; layer++) {
        let point = cascade_place(layer, p, n, nol);
        if point.inside < 0.0 {
            continue;
        }
        if view.sun_texel[layer] * PIXEL_TEXELS < footprint && layer + 1 < SHADOW_CASCADES {
            held = layer;
            held_point = point;
            continue;
        }
        return cascade_blend(layer, point, p, n, nol);
    }
    if held >= 0 {
        return cascade_blend(held, held_point, p, n, nol);
    }
    return 1.0;
}

// The color seen through the fog between the camera and `p`, the fog
// taking over from `fog_range.x` units away over the length of its
// fade. Linear light, the fog color is tonemapped with the rest.
fn fogged(color: vec3<f32>, p: vec3<f32>) -> vec3<f32> {
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

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let instance = instances[in.instance];
    let texel = textureSample(base_texture, base_sampler, in.uv);
    var map = textureSample(normal_texture, normal_sampler, in.uv).xyz * 2.0 - 1.0;
    map = vec3<f32>(map.xy * instance.normal_scale, map.z);

    let base = srgb_to_linear(instance.color.rgb) * srgb_to_linear(texel.rgb);
    let alpha = instance.color.a * texel.a;
    let metallic = instance.metallic;
    // Below this a highlight collapses to a few pixels of noise.
    let perceptual = clamp(instance.roughness, 0.045, 1.0);

    let world_pos = world_position(in.pos);

    var s: Surface;
    s.normal = perturb_normal(normalize(in.normal), world_pos, in.uv, map);
    s.to_eye = normalize(view.camera_pos.xyz - world_pos);
    s.nov = max(dot(s.normal, s.to_eye), 1e-4);
    s.diffuse = base * (1.0 - metallic);
    s.f0 = mix(vec3<f32>(0.04), base, metallic);
    s.f90 = saturate(dot(s.f0, vec3<f32>(50.0 * 0.33)));
    s.roughness = perceptual * perceptual;

    var color: vec3<f32>;
    if view.ambient.w > 0.5 {
        // The sky lights the diffuse through its harmonics and the
        // reflection through the mip that matches the roughness.
        let reflected = reflect(-s.to_eye, s.normal);
        let specular = sky_radiance(reflected, perceptual * 5.0);
        color = sky_irradiance(s.normal) * s.diffuse + specular * env_brdf(s.f0, perceptual, s.nov);
    } else {
        // A flat ambient stands in for a sky. Its diffuse is the Lambert
        // term, its reflection the base reflectance.
        color = view.ambient.rgb * (s.diffuse + s.f0);
    }

    color += shade(s, -view.sun_dir.xyz, view.sun_color.rgb) * sun_shadow(world_pos, normalize(in.normal));

    for (var slot = 0u; slot < instance.light_count; slot++) {
        let light = lights[light_index(instance.lights, slot)];

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

    return vec4<f32>(encode(fogged(color, world_pos)), alpha);
}
