
struct SkyOutput {
    @builtin(position) pos: vec4<f32>,
}

// One triangle over the whole viewport, at the far end of the reversed
// depth.
@vertex
fn v_main(@builtin(vertex_index) index: u32) -> SkyOutput {
    var out: SkyOutput;
    let x = f32((index << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(index & 2u) * 2.0 - 1.0;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

fn star_hash(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.5453123);
}

// The `DayNightSky` in this direction, in linear light: a gradient that
// darkens with the night, the sunset glow on the horizon towards a low
// sun, the sun disc and its glow, then stars, the moon disc and its
// glow as the night comes.
fn day_night_sky(dir: vec3<f32>) -> vec3<f32> {
    let up = dir.y;
    let sun_dir = view.sky_sun.xyz;
    let moon_dir = view.sky_moon.xyz;
    let night = view.sky_moon.w;
    let day = 1.0 - night;
    let seed = view.sky_params.z;

    let height = saturate(up);
    let day_sky = mix(vec3<f32>(0.6, 0.75, 0.9), vec3<f32>(0.25, 0.45, 0.85), height);
    let night_sky = mix(vec3<f32>(0.004, 0.004, 0.012), vec3<f32>(0.002, 0.002, 0.008), height);
    var color = mix(day_sky, night_sky, night);

    let sunset = view.sky_params.x * day;
    if sunset > 0.0 {
        let sun_flat = normalize(vec3<f32>(sun_dir.x, 0.0, sun_dir.z) + vec3<f32>(0.0, 0.0, 1e-6));
        let view_flat = normalize(vec3<f32>(dir.x, 0.0, dir.z) + vec3<f32>(0.0, 0.0, 1e-6));
        let toward = pow(saturate(dot(view_flat, sun_flat)), 2.0);
        let low = saturate(1.0 - abs(up) * 2.5);
        color = mix(color, vec3<f32>(0.95, 0.45, 0.15), sunset * toward * low * 0.8);
    }

    if up < 0.0 {
        color = mix(color, vec3<f32>(0.01, 0.01, 0.02), saturate(-up * 5.0));
    }

    let sun_dot = dot(dir, sun_dir);
    if sun_dot > 0.9995 && sun_dir.y > 0.0 {
        color = mix(color, vec3<f32>(1.0, 0.95, 0.8), saturate((sun_dot - 0.9995) / 0.0005));
    }
    if sun_dir.y > -0.05 {
        color += vec3<f32>(1.0, 0.9, 0.6) * pow(saturate(sun_dot), 128.0) * 0.5 * day;
    }

    // A star sits in one cell of a grid of 200 cells per unit over the
    // direction, one cell in about 270 holds one.
    if night > 0.01 && up > 0.0 {
        let scale = 200.0;
        let cell = floor(dir * scale);
        let pick = star_hash(cell + vec3<f32>(seed, 0.0, 0.0));
        if pick > 0.99625 {
            let center = normalize((cell + vec3<f32>(0.5)) / scale);
            if length(dir - center) < 0.002 {
                let brightness = star_hash(cell + vec3<f32>(0.0, seed, 0.0));
                let twinkle = 0.7 + 0.3 * sin(view.sky_params.y + pick * 100.0);
                color += vec3<f32>(0.9, 0.92, 1.0) * night * brightness * twinkle;
            }
        }
    }

    let moon_dot = dot(dir, moon_dir);
    if moon_dir.y > 0.0 {
        if moon_dot > 0.999 {
            color = mix(color, vec3<f32>(0.85, 0.88, 0.95), saturate((moon_dot - 0.999) / 0.001) * night);
        }
        color += vec3<f32>(0.5, 0.55, 0.7) * pow(saturate(moon_dot), 64.0) * 0.3 * night;
    }

    return color;
}

// A star in this direction, in a grid of 200 cells per unit over the
// direction, one cell in about 270 holds one, or black.
fn star(dir: vec3<f32>, seed: f32) -> vec3<f32> {
    let scale = 200.0;
    let cell = floor(dir * scale);
    let pick = star_hash(cell + vec3<f32>(seed, 0.0, 0.0));
    if pick > 0.99625 {
        let center = normalize((cell + vec3<f32>(0.5)) / scale);
        if length(dir - center) < 0.002 {
            return vec3<f32>(0.9, 0.92, 1.0) * star_hash(cell + vec3<f32>(0.0, seed, 0.0));
        }
    }
    return vec3<f32>(0.0);
}

// The `SpaceSky` in this direction: the air marched from the camera, or
// black with none, the stars turned by their frame and hidden by a bright
// sky, and the sun's disc darker at its rim. A ray into the planet ends
// on its lit ground.
fn space_sky(dir: vec3<f32>) -> vec3<f32> {
    let to_sun = -view.sun_dir.xyz;
    var light = vec3<f32>(0.0);
    var through = vec3<f32>(1.0);
    if has_air() {
        let origin = -view.air[0].xyz;
        let air = air_march(origin, dir, to_sun, 1e30, 24u, vec3<f32>(0.0));
        light = air.light * view.sun_color.rgb;
        through = air.through;
        let span = air_span(origin, dir, 1e30);
        if span.hit && span.ground {
            return light;
        }
    }
    if view.space.y > 0.0 {
        let turn = mat3x3<f32>(view.stars[0].xyz, view.stars[1].xyz, view.stars[2].xyz);
        // The eye adapts to a bright sky and loses the stars in it.
        let hide = saturate(1.0 - dot(light, vec3<f32>(0.2126, 0.7152, 0.0722)) * 20.0);
        light += star(turn * dir, view.space.z) * view.space.y * through * hide;
    }
    if view.space.w > 0.5 {
        let size = max(view.occlusion.y, 1e-4);
        let apart = atan2(length(cross(dir, to_sun)), dot(dir, to_sun));
        if apart < size {
            let edge = apart / size;
            let limb = 1.0 - 0.6 * (1.0 - sqrt(1.0 - edge * edge));
            // Far brighter than the sky but not so bright that the tone
            // mapping turns a red sun white.
            light += view.sun_color.rgb * through * limb * 8.0;
        }
    }
    return light;
}

// The sky in this direction, and with fog the fog color at the horizon
// clearing with height. Without a sky the fog color alone, the pass
// then runs only for the fog.
@fragment
fn f_main(in: SkyOutput) -> @location(0) vec4<f32> {
    let ndc = fragment_ndc(in.pos).xy;
    // The depth is reversed, 1 on the near plane. 0 lies at infinity
    // when the camera has no far plane, so the second point is halfway.
    let near = view.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
    let far = view.inv_view_proj * vec4<f32>(ndc, 0.5, 1.0);
    let dir = normalize(far.xyz / far.w - near.xyz / near.w);
    let day_night = view.sky_sun.w > 0.5;
    let space = view.space.x > 0.5;
    var color = select(sky_radiance(dir, 0.0), day_night_sky(dir), day_night);
    if space {
        color = space_sky(dir);
    }
    if view.fog_color.w > 0.5 && !has_air() {
        let amount = saturate(1.0 - dir.y / view.fog_range.z);
        let has_sky = view.ambient.w > 0.5 || day_night || space;
        color = select(view.fog_color.rgb, mix(color, view.fog_color.rgb, amount), has_sky);
    }
    return vec4<f32>(encode(color), 1.0);
}
