
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
    params: vec4<f32>,
}

// See `MeshLight` for what the fourth components carry.
struct Light {
    position: vec4<f32>,
    direction: vec4<f32>,
    color: vec4<f32>,
}

@group(1) @binding(0)
var<storage, read> instances: array<MeshInstance>;

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

@vertex
fn v_main(vertex: Vertex, instance: Instance) -> VertexOutput {
    let model = mat4x4<f32>(instance.model0, instance.model1, instance.model2, instance.model3);
    let normal_matrix = mat3x3<f32>(instance.normal0.xyz, instance.normal1.xyz, instance.normal2.xyz);

    var out: VertexOutput;
    out.pos = view.view_proj * model * vec4<f32>(vertex.pos, 1.0);
    out.uv = vertex.uv;
    out.normal = normalize(normal_matrix * vertex.normal);
    out.instance = instance.index;
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
    let invmax = inverseSqrt(max(dot(t, t), dot(b, b)));
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

fn light_index(packed: vec4<u32>, slot: u32) -> u32 {
    return (packed[slot / 2u] >> ((slot % 2u) * 16u)) & 0xffffu;
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let instance = instances[in.instance];
    let texel = textureSample(base_texture, base_sampler, in.uv);
    var map = textureSample(normal_texture, normal_sampler, in.uv).xyz * 2.0 - 1.0;
    map = vec3<f32>(map.xy * instance.params.x, map.z);

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

    color += shade(s, -view.sun_dir.xyz, view.sun_color.rgb);

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

    return vec4<f32>(encode(color), alpha);
}
