
struct SkyOutput {
    @builtin(position) pos: vec4<f32>,
}

// One triangle over the whole viewport, at the far plane.
@vertex
fn v_main(@builtin(vertex_index) index: u32) -> SkyOutput {
    var out: SkyOutput;
    let x = f32((index << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(index & 2u) * 2.0 - 1.0;
    out.pos = vec4<f32>(x, y, 1.0, 1.0);
    return out;
}

@fragment
fn f_main(in: SkyOutput) -> @location(0) vec4<f32> {
    let ndc = fragment_ndc(in.pos).xy;
    let near = view.inv_view_proj * vec4<f32>(ndc, 0.0, 1.0);
    let far = view.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
    let dir = normalize(far.xyz / far.w - near.xyz / near.w);
    return vec4<f32>(encode(sky_radiance(dir, 0.0)), 1.0);
}
