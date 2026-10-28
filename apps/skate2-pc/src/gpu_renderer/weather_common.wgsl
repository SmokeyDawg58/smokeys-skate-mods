// Adapted from Skate 3 Remaster build10 / revision038ff331.
// Shipped custom weather shader; host integration is local to Skate2.
// MIT License
// 
// Copyright (c) 2026 Skate 3 Custom Engine Layer contributors
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of the original Custom Engine Layer software and associated documentation
// files (the "Software"), to deal in the Software without restriction,
// including without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to permit
// persons to whom the Software is furnished to do so, subject to the following
// conditions:
// 
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// 
// This license applies only to original project-owned additions. It does not
// relicense Skate3Recomp, rexglue, third-party dependencies, Electronic Arts
// game data, or assets embedded by map authors.


fn tint() -> vec3<f32> { return vec3<f32>(0.62, 0.66, 0.72); }

fn cover(rain: f32) -> f32 { return saturate(rain * 2.0); }

fn gloom(rain: f32) -> f32 { return 1.0 - 0.18 * cover(rain); }

fn hash(cell: vec2<f32>) -> f32 {

    let q = bitcast<vec2<u32>>(vec2<i32>(cell));

    var h = (q.x * 1597334677u) ^ (q.y * 3812015801u);

    h = h * 747796405u + 2891336453u;

    h = ((h >> ((h >> 28u) + 4u)) ^ h) * 277803737u;

    return f32((h >> 22u) ^ h) / 4294967295.0;

}

fn noise(p: vec2<f32>) -> f32 {

    let i = floor(p);

    let f = fract(p);

    let u = f * f * (3.0 - 2.0 * f);

    return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),

        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);

}

fn overcast(light: vec3<f32>, rain: f32) -> vec3<f32> {

    let lifted = pow(max(light, vec3<f32>(0.0)), vec3<f32>(0.7));

    let grey = mix(lifted, vec3<f32>(dot(lifted, vec3<f32>(0.2126, 0.7152, 0.0722))), 0.5);

    return mix(light, grey * tint(), cover(rain));

}

fn clouds(direction: vec3<f32>, t: f32, sky: vec3<f32>) -> vec3<f32> {

    let up = saturate(direction.y);

    let p = direction.xz / (up + 0.12) * 1.8 + vec2<f32>(0.012, 0.006) * t;

    let n = 0.5 * noise(p) + 0.3 * noise(p * 2.3 + 5.0) + 0.2 * noise(p * 5.1 + 11.0);

    let shade = mix(0.6, 1.05, smoothstep(0.25, 0.8, n)) * (1.0 - 0.15 * up);

    return sky * mix(1.0, shade, smoothstep(0.0, 0.25, up));

}

fn snow_light(baked: vec3<f32>) -> vec3<f32> { return pow(max(baked, vec3<f32>(0.0)), vec3<f32>(0.75)) * 0.9; }

fn snow_albedo(xz: vec2<f32>) -> vec3<f32> {

    return vec3<f32>(0.8, 0.83, 0.88) * (0.88 + 0.07 * noise(xz * 0.35) + 0.05 * noise(xz * 3.0));

}

fn snow_cover(up: f32, xz: vec2<f32>, light: f32, snow: f32) -> f32 {

    let ragged = 0.5 * (noise(xz * 1.3) + noise(xz * 4.1 + 9.0)) - 0.5;

    return smoothstep(0.3, 0.6, up + 0.3 * ragged) * smoothstep(0.03, 0.12, light) * cover(snow);

}
