struct PathView {
    position: vec2<f32>,
    resolution: vec2<f32>,
    color: vec4<f32>,
    z_position: f32,
    scale: f32,
}

@group(0) @binding(0) var<uniform> path_view: PathView;

@vertex
fn v_main(
    @location(0) vertex: vec2<f32>,
) -> @builtin(position) vec4<f32>  {
    let p = (vertex + path_view.position) * path_view.scale;

    let x = p.x * 2.0 / path_view.resolution.x - 1.0;
    let y = 1.0 - p.y * 2.0 / path_view.resolution.y;

    return vec4<f32>(x, y, path_view.z_position, 1.0);
}

@fragment
fn f_main() -> @location(0) vec4<f32> {
    return path_view.color;
}
