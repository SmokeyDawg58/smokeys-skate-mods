@group(0) @binding(0) var<uniform> data: vec4<u32>;
// Screen pixels per glyph pixel; must match fps.rs SCALE.
const SCALE: u32 = 5u;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>,3>(vec2<f32>(-1.0,-1.0),vec2<f32>(3.0,-1.0),vec2<f32>(-1.0,3.0));
    return vec4<f32>(positions[i],0.0,1.0);
}
fn glyph(i: u32) -> u32 { switch i {
case 0u: { return 31599u; }
case 1u: { return 29850u; }
case 2u: { return 29671u; }
case 3u: { return 31207u; }
case 4u: { return 18925u; }
case 5u: { return 31183u; }
case 6u: { return 31695u; }
case 7u: { return 18727u; }
case 8u: { return 31727u; }
case 9u: { return 31215u; }
case 10u: { return 4815u; }
case 11u: { return 4843u; }
case 12u: { return 31183u; }
case 13u: { return 1040u; }
default: { return 0u; }
} }
@fragment fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    if p.x < 10.0 || p.y < 10.0 { discard; }
    let x = u32(p.x - 10.0) / SCALE;
    let y = u32(p.y - 10.0) / SCALE;
    let cell = x / 4u;
    if y >= 5u || x % 4u >= 3u || cell >= 9u { discard; }
    var code = 99u;
    if cell < 4u { code = 10u + cell; }
    if cell >= 5u {
        var divisor = 1u;
        switch cell { case 5u: { divisor = 1000u; } case 6u: { divisor = 100u; } case 7u: { divisor = 10u; } default: {} }
        if data.x >= divisor || cell == 8u { code = (data.x / divisor) % 10u; }
    }
    if (glyph(code) & (1u << (y * 3u + x % 4u))) == 0u { discard; }
    return vec4<f32>(1.0);
}
