// One triangle over the viewport that only writes depth: 0, the far end
// of the scene's reversed depth, before the scene draws, and 1 after it,
// so the UI over it tests depth as before.
@vertex
fn v_far(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return over_viewport(index, 0.0);
}

@vertex
fn v_near(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return over_viewport(index, 1.0);
}

fn over_viewport(index: u32, depth: f32) -> vec4<f32> {
    let x = f32((index << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(index & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, depth, 1.0);
}

@fragment
fn f_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}
