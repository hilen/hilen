// The fade of a node, see `Material::fade`. A node with a fade below 1
// draws through the cutout entries, which leave out every fragment whose
// 4 by 4 ordered dither threshold, in pixels of the target, is over the
// fade. A flipped node keeps the other share, so a pair at the same place
// whose fades add up to 1, one of them flipped, covers every pixel once.
fn faded_out(pixel: vec2<f32>, fade: f32, flip: f32) -> bool {
    var bayer = array<u32, 16>(0u, 8u, 2u, 10u, 12u, 4u, 14u, 6u, 3u, 11u, 1u, 9u, 15u, 7u, 13u, 5u);
    let at = vec2<u32>(floor(pixel)) % 4u;
    let threshold = (f32(bayer[at.y * 4u + at.x]) + 0.5) / 16.0;
    // Each comparison in its own parentheses, a bare `a < b, c > d` parses
    // as a template list.
    return select((threshold > fade), (threshold < 1.0 - fade), (flip > 0.5));
}
