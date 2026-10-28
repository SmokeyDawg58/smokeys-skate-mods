// Final vehicle_defaultPS.fpo pc37..41: the green channel selects authored
// surface colour versus the instance's red/blue paint masks. Input is squared.
fn vehicle_paint(diffuse: vec3<f32>) -> vec3<f32> {
    if diffuse.g < 0.001225 {
        return diffuse.r*object.colours[0].rgb + diffuse.b*object.colours[1].rgb;
    }
    return diffuse;
}

fn vehicle_irradiance(n: vec3<f32>) -> vec3<f32> {
    var i = object.irradiance[0].xyz;
    i += object.irradiance[1].xyz*n.x;
    i += object.irradiance[2].xyz*n.y;
    i += object.irradiance[3].xyz*n.z;
    i += object.irradiance[4].xyz*(n.x*n.z);
    i += object.irradiance[5].xyz*(n.z*n.y);
    i += object.irradiance[6].xyz*(n.y*n.x);
    i += object.irradiance[7].xyz*(3.0*n.z*n.z-1.0);
    i += object.irradiance[8].xyz*(n.x*n.x-n.y*n.y);
    return i;
}

fn vehicle_ambient(n: vec3<f32>, multiplier: f32) -> vec3<f32> {
    // Body pc112 MAX c253.xyz followed by pc113's xzy swizzle; glass pc97
    // MAX c254.xyz into yzw, then pc98/104. Both yield RGB (0.036,0.040,0.042), BEFORE
    // m_params[5].w. The older UPDB HLSL omits this shipped ambient floor.
    return max(vehicle_irradiance(n),vec3<f32>(0.036,0.040,0.042))*multiplier;
}
