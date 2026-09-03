// Debug lines over the scene, the collider wireframes. Vertices come in
// world space with their own color, encoded like the frame, and nothing
// lights them. The view is `scene_common.wgsl`, prepended.

struct LineVertex {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct LineOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn v_main(vertex: LineVertex) -> LineOutput {
    var out: LineOutput;
    out.pos = view.view_proj * vec4<f32>(vertex.pos, 1.0);
    out.color = vertex.color;
    return out;
}

@fragment
fn f_main(in: LineOutput) -> @location(0) vec4<f32> {
    return in.color;
}
