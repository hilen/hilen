// The pass that builds the sky view table, see `SKY_VIEW_SIZE`. Appended
// to the common part and the air, with its own bind of the view and the
// air's tables.

struct SkyViewOutput {
    @location(0) light: vec4<f32>,
    @location(1) through: vec4<f32>,
}

@vertex
fn v_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(index & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn f_main(@builtin(position) pos: vec4<f32>) -> SkyViewOutput {
    let air = air_sky(sky_view_dir(pos.xy / SKY_VIEW_SIZE));
    var out: SkyViewOutput;
    out.light = vec4<f32>(air.light, 1.0);
    out.through = vec4<f32>(air.through, 1.0);
    return out;
}
