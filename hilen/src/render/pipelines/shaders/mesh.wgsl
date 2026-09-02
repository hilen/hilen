
struct SceneView {
    view_proj: mat4x4<f32>,
    light_dir: vec3<f32>,
    ambient: f32,
}

struct MeshInstance {
    model: mat4x4<f32>,
    normal0: vec4<f32>,
    normal1: vec4<f32>,
    normal2: vec4<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> view: SceneView;

@group(1) @binding(0)
var<storage, read> instances: array<MeshInstance>;

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
}

// Four components cross the stage boundary. An A7 draws nothing above
// eight, see docs/ios.md, so the color stays in the storage buffer and
// only the index of the instance comes along.
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) @interpolate(flat) instance: u32,
}

@vertex
fn v_main(
    vertex: Vertex,
    instance: Instance,
    @builtin(instance_index) index: u32,
) -> VertexOutput {
    let model = mat4x4<f32>(instance.model0, instance.model1, instance.model2, instance.model3);
    let normal_matrix = mat3x3<f32>(instance.normal0.xyz, instance.normal1.xyz, instance.normal2.xyz);

    var out: VertexOutput;
    out.pos = view.view_proj * model * vec4<f32>(vertex.pos, 1.0);
    out.normal = normalize(normal_matrix * vertex.normal);
    out.instance = index;
    return out;
}

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

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = instances[in.instance].color;
    let albedo = srgb_to_linear(color.rgb);
    let normal = normalize(in.normal);
    let diffuse = max(dot(normal, -view.light_dir), 0.0);
    let lit = albedo * (view.ambient + diffuse * (1.0 - view.ambient));
    return vec4<f32>(linear_to_srgb(lit), color.a);
}
