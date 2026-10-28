// render.big `data\render\shaders\final\spline_defaultVS.vpo`, `spline_defaultPS.fpo` and
// `spline_darkenPS.fpo`, as their shipped microcode computes: a uniform cubic B-spline of four
// control points through `i_matWorld` (c4..c6), the clip-space depth fades of `i_clipvalues`
// (c200) and `i_intensity` (c199). The pixel programs do not square the colour the `.updb`
// source squares.

struct Draw {
    world: array<vec4<f32>, 3>,
    intensity: vec4<f32>,
    clip_values: vec4<f32>,
    camera: vec4<f32>,
    clip_from_camera: mat4x4<f32>,
    cp: array<vec4<f32>, 142>,
};

@group(2) @binding(0) var<uniform> eye_clip:mat4x4<f32>;
@group(0) @binding(0) var<uniform> draw: Draw;
@group(1) @binding(0) var diffuse: texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler: sampler;

struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) intensity: vec4<f32>,
    @location(1) uv: vec3<f32>,
};

@vertex
fn vs(@location(0) source: vec3<f32>) -> Varyings {
    let t = fract(source.x);
    let first = i32(floor(source.x));
    let blend = vec4<f32>(1.0, t, t * t, t * t * t);
    let weight = vec4<f32>(
        dot(blend, vec4<f32>(1.0, -3.0, 3.0, -1.0)),
        dot(blend, vec4<f32>(4.0, 0.0, -6.0, 3.0)),
        dot(blend, vec4<f32>(1.0, 3.0, 3.0, -3.0)),
        dot(blend, vec4<f32>(0.0, 0.0, 0.0, 1.0)),
    );
    var point = vec3<f32>(0.0);
    for (var i = 0; i < 4; i++) {
        let control = draw.cp[first + i];
        let world = vec3<f32>(dot(control, draw.world[0]), dot(control, draw.world[1]), dot(control, draw.world[2]));
        point += world * weight[i];
    }
    point *= 1.0 / 6.0;
    let clip = eye_clip * draw.clip_from_camera * vec4<f32>(point - draw.camera.xyz, 1.0);
    let near = saturate((clip.z - draw.clip_values.x) / draw.clip_values.y);
    let far = 1.0 - saturate((clip.z - draw.clip_values.z) / draw.clip_values.w);
    var o: Varyings;
    o.position = clip;
    o.intensity = draw.intensity;
    o.uv = vec3<f32>(source.y, source.z, min(near, far));
    return o;
}

// spline_defaultPS: colour times intensity.x over alpha, faded; alpha the reciprocal.
@fragment
fn fs_default(v: Varyings) -> @location(0) vec4<f32> {
    let colour = textureSample(diffuse, diffuse_sampler, v.uv.xy);
    let inverse = 1.0 / colour.a;
    return vec4<f32>(colour.rgb * v.intensity.x * inverse * v.uv.z, inverse);
}

// spline_darkenPS: colour times intensity.y; alpha times intensity.y, faded.
@fragment
fn fs_darken(v: Varyings) -> @location(0) vec4<f32> {
    let colour = textureSample(diffuse, diffuse_sampler, v.uv.xy);
    return vec4<f32>(colour.rgb * v.intensity.y, colour.a * v.intensity.y * v.uv.z);
}
