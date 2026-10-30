// The air of a `SpaceSky` and the balls that hide the light, shared by
// the sky and the mesh shader. The air is the math of
// `scene/atmosphere.rs`, which builds the 2 tables, see `Atmosphere` and
// Hillaire 2020. `view.air` holds, from 0 to 5: the center from the
// camera and the ground radius, Rayleigh scattering and the top radius,
// Mie scattering and the Rayleigh height, Mie extinction and the Mie
// height, ozone absorption and the ozone center, then the ozone width,
// the Mie g, the ground albedo and 1 when there is air.

@group(0) @binding(4)
var air_transmittance: texture_2d<f32>;

@group(0) @binding(5)
var air_multi: texture_2d<f32>;

// The share of a light's disc that the occluders hide from a point, as
// the light still let through. `offset` is the point from the camera,
// `to_light` the unit way to the light, `size` half its disc's width in
// radians. A ball much wider in the sky than the light, a planet under
// the point, hides the share of the disc past a straight edge. 2 of
// about 1 size, a moon in front of the sun, hide the overlap of 2 discs.
fn light_through(offset: vec3<f32>, to_light: vec3<f32>, size: f32) -> vec3<f32> {
    var through = vec3<f32>(1.0);
    let count = u32(view.occlusion.x);
    let cos_size = cos(size);
    let sin_size = sin(size);
    for (var index = 0u; index < count; index++) {
        let ball = view.occluders[index];
        let toward = ball.xyz - offset;
        let distance = max(length(toward), 1e-6);
        let along = dot(to_light, toward) / distance;
        // A light whose disc lies wholly past the ball's edge, the cosine
        // of the ball's angular radius plus the disc's, is not hidden, the
        // full test below gives exactly 0 there. The margin covers the 32
        // bit error of the ball's size near its surface, about 4e-4 there.
        let sin_ball = min(ball.w / distance, 1.0);
        let cos_ball = sqrt(max(1.0 - sin_ball * sin_ball, 0.0));
        if along < cos_ball * cos_size - sin_ball * sin_size - 2e-3 {
            continue;
        }
        // The mirror: a ball wider than the light whose disc lies wholly
        // inside the ball's edge hides all of it, the full test gives 1.
        if sin_ball > sin_size * 1.01 && along > cos_ball * cos_size + sin_ball * sin_size + 2e-3 {
            through *= view.transmit[index].rgb;
            continue;
        }
        let across = length(cross(to_light, toward)) / distance;
        // The ball's edge from the point, as the angle between the way to
        // its center and its rim, 0 on or inside the ball. Worked out from
        // the height over the ball and not from radius over distance,
        // which loses the last meters of a 20 km planet in 32 bits.
        let rim = atan2(sqrt(max(distance - ball.w, 0.0) * (distance + ball.w)), ball.w);
        let ball_size = PI * 0.5 - rim;
        var hidden: f32;
        if ball_size > 8.0 * size {
            // How far the light stands inside the edge, in light radii.
            let inside = clamp((atan2(along, across) - rim) / max(size, 1e-6), -1.0, 1.0);
            hidden = (acos(-inside) + inside * sqrt(1.0 - inside * inside)) / PI;
        } else {
            hidden = disc_overlap(size, ball_size, atan2(across, along));
        }
        through *= mix(vec3<f32>(1.0), view.transmit[index].rgb, hidden);
    }
    return through;
}

// The share of a disc of radius `light` that a disc of radius `ball`
// `apart` from it covers, all angles small enough to treat as flat.
fn disc_overlap(light: f32, ball: f32, apart: f32) -> f32 {
    if apart >= light + ball {
        return 0.0;
    }
    if apart <= ball - light {
        return 1.0;
    }
    if apart <= light - ball {
        return (ball * ball) / (light * light);
    }
    let a = acos(clamp((apart * apart + light * light - ball * ball) / (2.0 * apart * light), -1.0, 1.0));
    let b = acos(clamp((apart * apart + ball * ball - light * light) / (2.0 * apart * ball), -1.0, 1.0));
    let k = sqrt(max((light + ball - apart) * (apart + light - ball) * (apart - light + ball) * (apart + light + ball), 0.0));
    return (light * light * a + ball * ball * b - 0.5 * k) / (PI * light * light);
}

// Rayleigh and Mie heights of air over this height leave about 1.6e-4
// of the air at the ground, under a color step of the darkest pixels.
const AIR_EMPTY_HEIGHTS: f32 = 8.75;

// The height over which a step of the march adds nothing, above both
// kinds of haze and the ozone.
fn air_empty_height() -> f32 {
    let haze = AIR_EMPTY_HEIGHTS * max(view.air[2].w, view.air[3].w);
    return max(haze, view.air[4].w + view.air[5].x);
}

fn has_air() -> bool {
    return view.air[5].w > 0.5;
}

struct AirMedium {
    rayleigh: vec3<f32>,
    mie: vec3<f32>,
    extinction: vec3<f32>,
}

fn air_medium(height: f32) -> AirMedium {
    let h = max(height, 0.0);
    var medium: AirMedium;
    medium.rayleigh = view.air[1].xyz * exp(-h / view.air[2].w);
    let mie_share = exp(-h / view.air[3].w);
    medium.mie = view.air[2].xyz * mie_share;
    let ozone = max(1.0 - abs(h - view.air[4].w) / view.air[5].x, 0.0);
    medium.extinction = medium.rayleigh + view.air[3].xyz * mie_share + view.air[4].xyz * ozone;
    return medium;
}

// A value from 0 to 1 as a place in a table of `size` texels, 0 on the
// middle of the first texel and 1 on the middle of the last, how the
// tables were filled.
fn table_place(x: f32, size: f32) -> f32 {
    return (saturate(x) * (size - 1.0) + 0.5) / size;
}

fn air_hits_ground(r: f32, mu: f32) -> bool {
    let ground = view.air[0].w;
    return mu < 0.0 && r * r * (mu * mu - 1.0) + ground * ground >= 0.0;
}

// What gets through the air from `r` from the center along `mu`, the
// cosine from straight up, to the top.
fn air_transmittance_at(r: f32, mu: f32) -> vec3<f32> {
    let ground = view.air[0].w;
    let top = view.air[1].w;
    let horizon = sqrt(top * top - ground * ground);
    let rho = sqrt(max(r * r - ground * ground, 0.0));
    let d = max(-r * mu + sqrt(max(r * r * (mu * mu - 1.0) + top * top, 0.0)), 0.0);
    let d_min = top - r;
    let d_max = rho + horizon;
    let uv = vec2<f32>(
        table_place((d - d_min) / max(d_max - d_min, 1e-6), 256.0),
        table_place(rho / horizon, 64.0),
    );
    return textureSampleLevel(air_transmittance, sky_sampler, uv, 0.0).rgb;
}

// The sun reaching `p`, relative to the center: through the air, none
// behind the ground.
fn air_sun_at(p: vec3<f32>, to_sun: vec3<f32>) -> vec3<f32> {
    let r = length(p);
    let mu = dot(p, to_sun) / r;
    if air_hits_ground(r, mu) {
        return vec3<f32>(0.0);
    }
    return air_transmittance_at(r, mu);
}

fn air_multi_at(r: f32, mu_sun: f32) -> vec3<f32> {
    let height = (r - view.air[0].w) / (view.air[1].w - view.air[0].w);
    let uv = vec2<f32>(table_place(mu_sun * 0.5 + 0.5, 32.0), table_place(height, 32.0));
    return textureSampleLevel(air_multi, sky_sampler, uv, 0.0).rgb;
}

// What the air lets through of a light toward `to_light` to a point
// `offset` from the camera, 1 with no air. Behind the planet the
// occluders do the rest.
fn air_light_at(offset: vec3<f32>, to_light: vec3<f32>) -> vec3<f32> {
    if !has_air() {
        return vec3<f32>(1.0);
    }
    let p = offset - view.air[0].xyz;
    let r = length(p);
    return air_transmittance_at(r, dot(p, to_light) / r);
}

fn rayleigh_phase(cos: f32) -> f32 {
    return 3.0 / (16.0 * PI) * (1.0 + cos * cos);
}

// The Cornette-Shanks phase of Mie scattering.
fn mie_phase(cos: f32, g: f32) -> f32 {
    let g2 = g * g;
    return 3.0 / (8.0 * PI) * (1.0 - g2) * (1.0 + cos * cos) / ((2.0 + g2) * pow(1.0 + g2 - 2.0 * g * cos, 1.5));
}

struct AirSpan {
    start: f32,
    end: f32,
    ground: bool,
    hit: bool,
}

// The part of the ray from `origin`, relative to the center, along `dir`
// in the air up to `limit`, and whether it ends on the ground.
fn air_span(origin: vec3<f32>, dir: vec3<f32>, limit: f32) -> AirSpan {
    var span: AirSpan;
    span.hit = false;
    span.ground = false;
    let b = dot(origin, dir);
    let top = view.air[1].w;
    let ground = view.air[0].w;
    let length2 = dot(origin, origin);
    let discriminant = b * b - (length2 - top * top);
    if discriminant < 0.0 {
        return span;
    }
    let root = sqrt(discriminant);
    let far = -b + root;
    if far <= 0.0 {
        return span;
    }
    span.start = max(-b - root, 0.0);
    var end = far;
    var on_ground = false;
    let ground_discriminant = b * b - (length2 - ground * ground);
    if ground_discriminant >= 0.0 {
        let near = -b - sqrt(ground_discriminant);
        if near > 0.0 {
            end = min(near, far);
            on_ground = true;
        }
    }
    span.ground = on_ground && end <= limit;
    span.end = min(end, limit);
    span.hit = span.end > span.start;
    return span;
}

struct AirLight {
    light: vec3<f32>,
    through: vec3<f32>,
}

// The light the air sends back along `dir` to `origin`, relative to the
// center, for a sun of 1 toward `to_sun`, and how much of what lies
// behind gets through, up to `limit`, in `steps` steps. In the units of
// a scene, pi times radiance, the air alone with no ground, see
// `air_ground`. `from_camera` is `origin` from the camera, where the
// occluders are.
fn air_march(origin: vec3<f32>, dir: vec3<f32>, to_sun: vec3<f32>, limit: f32, steps: u32, from_camera: vec3<f32>) -> AirLight {
    var out: AirLight;
    out.light = vec3<f32>(0.0);
    out.through = vec3<f32>(1.0);
    let span = air_span(origin, dir, limit);
    if !span.hit {
        return out;
    }
    let cos = dot(dir, to_sun);
    let rayleigh = rayleigh_phase(cos);
    let mie = mie_phase(cos, view.air[5].y);
    let step = (span.end - span.start) / f32(steps);
    let sun_size = view.occlusion.y;
    let empty = air_empty_height();
    for (var index = 0u; index < steps; index++) {
        let t = span.start + (f32(index) + 0.5) * step;
        let p = origin + dir * t;
        let r = length(p);
        let height = r - view.air[0].w;
        // So little air is left up there that a step changes no pixel by
        // a color step.
        if height > empty {
            continue;
        }
        let medium = air_medium(height);
        let step_through = exp(-medium.extinction * step);
        // An eclipse darkens the air in the moon's shadow, the planet's
        // own shadow shows over the night side at dusk.
        let shade = light_through(from_camera + dir * t, to_sun, sun_size);
        // In full shadow the step only dims what lies behind it.
        if any(shade > vec3<f32>(0.0)) {
            let sun = air_sun_at(p, to_sun);
            let multi = air_multi_at(r, dot(p, to_sun) / r);
            let source = shade * (sun * (medium.rayleigh * rayleigh + medium.mie * mie) + multi * (medium.rayleigh + medium.mie));
            out.light += out.through * (source - source * step_through) / max(medium.extinction, vec3<f32>(1e-12));
        }
        out.through *= step_through;
    }
    // `view.occlusion.w` is `SpaceSky::air_brightness`.
    out.light *= PI * view.occlusion.w;
    return out;
}

// The sunlit ground of the air at `p`, relative to the center, for a
// sun of 1 toward `to_sun`, with `from_camera` the same point from the
// camera. Only the sky adds it, where its ray ends on the ball. A
// surface over the ground stands for the ground itself, and a sea drawn
// as flat triangles sags under the ball, so its haze would end on the
// ball too and turn pale with this light.
fn air_ground(p: vec3<f32>, to_sun: vec3<f32>, from_camera: vec3<f32>) -> vec3<f32> {
    let shade = light_through(from_camera, to_sun, view.occlusion.y);
    let lit = max(dot(normalize(p), to_sun), 0.0) * view.air[5].z;
    return air_sun_at(p, to_sun) * shade * lit;
}

// The haze of the air between the camera and a point `offset` from it
// over `color`, in 2 steps. Every lit pixel runs this, 8 steps took 57
// percent of a frame at 500 m on an M1 Pro.
fn air_haze(color: vec3<f32>, offset: vec3<f32>) -> vec3<f32> {
    let distance = length(offset);
    let dir = offset / max(distance, 1e-6);
    let air = air_march(-view.air[0].xyz, dir, -view.sun_dir.xyz, distance, 2u, vec3<f32>(0.0));
    return color * air.through + air.light * view.sun_color.rgb;
}
