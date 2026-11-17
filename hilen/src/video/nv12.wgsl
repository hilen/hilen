// Decoder planes to RGB. The luma plane and the half size interleaved chroma
// plane come straight from the decoder, 8 bit NV12 or 10 bit P010, one
// fullscreen triangle writes the RGBA image the `ImageView` draws. Output is
// encoded sRGB, which is what a BT.709 or BT.601 video carries, so an SDR
// frame lands unchanged. A PQ or HLG frame is tone mapped down to SDR first.

struct Params {
    full_range: u32,
    // 0 BT.709, 1 BT.601, 2 BT.2020.
    matrix: u32,
    // The planes hold 16 bit little endian samples, P010.
    ten_bit: u32,
    // 0 SDR, 1 PQ, 2 HLG.
    transfer: u32,
}

@group(0) @binding(0) var t_y: texture_2d<f32>;
@group(0) @binding(1) var t_uv: texture_2d<f32>;
@group(0) @binding(2) var s_planes: sampler;

@group(1) @binding(0) var<uniform> params: Params;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn v_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let x = f32(i32(index & 1u) * 4 - 1);
    let y = f32(i32(index & 2u) * 2 - 1);
    var out: VertexOutput;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

// A 16 bit sample uploaded as 2 bytes, low then high. Linear filtering of the
// 2 bytes apart gives the same as filtering the sample, the sum is linear.
// P010 keeps its 10 bits at the top, so the full scale is 1023 * 64.
fn wide(low: f32, high: f32) -> f32 {
    return (low * 255.0 + high * 65280.0) / 65472.0;
}

// Y in 0..1 and chroma around zero, from either plane layout.
fn sample_yuv(uv: vec2<f32>) -> vec3<f32> {
    let luma = textureSample(t_y, s_planes, uv);
    let chroma = textureSample(t_uv, s_planes, uv);

    var yuv: vec3<f32>;
    var black = 16.0 / 255.0;
    var luma_scale = 255.0 / 219.0;
    var chroma_scale = 255.0 / 224.0;
    if params.ten_bit == 1u {
        yuv = vec3<f32>(wide(luma.r, luma.g), wide(chroma.r, chroma.g), wide(chroma.b, chroma.a));
        black = 64.0 / 1023.0;
        luma_scale = 1023.0 / 876.0;
        chroma_scale = 1023.0 / 896.0;
        // The middle of the chroma range is 512 of 1023, not a half.
        yuv = vec3<f32>(yuv.x, yuv.y - 512.0 / 1023.0, yuv.z - 512.0 / 1023.0);
    } else {
        yuv = vec3<f32>(luma.r, chroma.r - 0.5, chroma.g - 0.5);
    }

    if params.full_range == 0u {
        yuv = vec3<f32>((yuv.x - black) * luma_scale, yuv.y * chroma_scale, yuv.z * chroma_scale);
    }
    return yuv;
}

fn to_rgb(yuv: vec3<f32>) -> vec3<f32> {
    let y = yuv.x;
    let u = yuv.y;
    let v = yuv.z;
    if params.matrix == 1u {
        return vec3<f32>(y + 1.402 * v, y - 0.344136 * u - 0.714136 * v, y + 1.772 * u);
    }
    if params.matrix == 2u {
        return vec3<f32>(y + 1.4746 * v, y - 0.164553 * u - 0.571353 * v, y + 1.8814 * u);
    }
    return vec3<f32>(y + 1.5748 * v, y - 0.187324 * u - 0.468124 * v, y + 1.8556 * u);
}

const PQ_M1: f32 = 0.1593017578125;
const PQ_M2: f32 = 78.84375;
const PQ_C1: f32 = 0.8359375;
const PQ_C2: f32 = 18.8515625;
const PQ_C3: f32 = 18.6875;

// A PQ signal to light, 1 is 10000 nits. SMPTE ST 2084.
fn pq_to_light(signal: vec3<f32>) -> vec3<f32> {
    let p = pow(max(signal, vec3<f32>(0.0)), vec3<f32>(1.0 / PQ_M2));
    let top = max(p - vec3<f32>(PQ_C1), vec3<f32>(0.0));
    let bottom = vec3<f32>(PQ_C2) - PQ_C3 * p;
    return pow(top / bottom, vec3<f32>(1.0 / PQ_M1));
}

fn light_to_pq(light: f32) -> f32 {
    let p = pow(max(light, 0.0), PQ_M1);
    return pow((PQ_C1 + PQ_C2 * p) / (1.0 + PQ_C3 * p), PQ_M2);
}

fn pq_to_light_1(signal: f32) -> f32 {
    return pq_to_light(vec3<f32>(signal)).x;
}

// One HLG channel to scene light in 0..1. ARIB STD-B67.
fn hlg_to_scene(e: f32) -> f32 {
    if e <= 0.5 {
        return e * e / 3.0;
    }
    return (exp((e - 0.55991073) / 0.17883277) + 0.28466892) / 12.0;
}

// The light a mastering display of 1000 nits shows, with 1 as 10000 nits.
const SOURCE_PEAK: f32 = 0.1;
// The white of an SDR screen in HDR terms, 203 nits. BT.2408.
const SDR_WHITE: f32 = 0.0203;

// HDR light to SDR light in 0..1, both BT.2020. The BT.2390 curve in PQ
// space: everything under the knee stays as it is, the range above it up to
// the source peak rolls off into what is left under SDR white. It runs on
// the brightest channel and scales the other 2 with it, so the hue stays.
fn tone_map(light: vec3<f32>) -> vec3<f32> {
    let brightest = max(light.r, max(light.g, light.b));
    if brightest <= 0.0 {
        return vec3<f32>(0.0);
    }

    let source = light_to_pq(SOURCE_PEAK);
    let target_peak = light_to_pq(SDR_WHITE) / source;
    let knee = 1.5 * target_peak - 0.5;

    let e = clamp(light_to_pq(brightest) / source, 0.0, 1.0);
    var mapped = e;
    if e > knee {
        let t = (e - knee) / (1.0 - knee);
        let t2 = t * t;
        let t3 = t2 * t;
        mapped = (2.0 * t3 - 3.0 * t2 + 1.0) * knee + (t3 - 2.0 * t2 + t) * (1.0 - knee) + (-2.0 * t3 + 3.0 * t2) * target_peak;
    }

    let shown = pq_to_light_1(mapped * source) / SDR_WHITE;
    return light * (shown / brightest);
}

// Linear BT.2020 to linear BT.709.
fn narrow_gamut(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        1.6605 * c.r - 0.5876 * c.g - 0.0728 * c.b,
        -0.1246 * c.r + 1.1329 * c.g - 0.0083 * c.b,
        -0.0182 * c.r - 0.1006 * c.g + 1.1187 * c.b,
    );
}

// Display light to the signal an SDR video carries, the inverse of the
// BT.1886 display curve, so a tone mapped frame sits where an SDR master of
// it would.
fn encode(light: vec3<f32>) -> vec3<f32> {
    return pow(clamp(light, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.4));
}

@fragment
fn f_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let signal = to_rgb(sample_yuv(in.uv));

    var rgb = signal;
    if params.transfer == 1u {
        rgb = encode(narrow_gamut(tone_map(pq_to_light(signal))));
    } else if params.transfer == 2u {
        let scene = vec3<f32>(hlg_to_scene(signal.r), hlg_to_scene(signal.g), hlg_to_scene(signal.b));
        // The HLG system gamma for a 1000 nit display, on the luminance.
        let luminance = 0.2627 * scene.r + 0.6780 * scene.g + 0.0593 * scene.b;
        let shown = scene * (SOURCE_PEAK * pow(max(luminance, 0.000001), 0.2));
        rgb = encode(narrow_gamut(tone_map(shown)));
    } else if params.matrix == 2u {
        // SDR in the wide gamut, only the primaries move.
        let light = pow(clamp(signal, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(2.4));
        rgb = encode(narrow_gamut(light));
    }

    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
