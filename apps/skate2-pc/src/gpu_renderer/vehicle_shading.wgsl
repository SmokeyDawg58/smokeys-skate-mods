// Shipped vehicle.fx / vehicle_glass.fx / vehiclelight.fx. Shared vertex skin,
// world/CSM shadows and fog/postfx are supplied by dynamic_object/environment.
fn vehicle_lighting(input: PsIn, glass: bool) -> vec4<f32> {
    let sampled = textureSample(i_diffuse,wrap_sampler,input.uv);
    var diffuse = sampled*sampled;
    var specular = diffuse.a;
    diffuse.a = 1.0;
    if glass {
        diffuse = vec4<f32>(material.m_params[4].xyz,1.0);
        specular = 1.0;
    } else {
        diffuse = vec4<f32>(vehicle_paint(diffuse.rgb),1.0);
    }
    let normal_xy = 2.0*textureSample(i_normal,wrap_sampler,input.uv).ag-1.0;
    let tangent_normal = vec3<f32>(normal_xy,sqrt(abs(1.0-dot(normal_xy,normal_xy))));
    let normal = normalize(tangent_normal.x*input.tangent + tangent_normal.y*input.binormal + tangent_normal.z*input.normal);
    let view_dir = normalize(-input.relative);
    let reflected = reflect(view_dir,normal);
    let cube = textureSample(i_environment,cube_sampler,vec3<f32>(-reflected.xy,reflected.z));
    let reflection_colour = cube.rgb*cube.rgb;
    let world = vec4<f32>(input.relative+globals.view_position.xyz,1.0);
    // Compiled vehicle_defaultPS pc47 and glass pc40 use g_charattributes[0].z
    // for the PCF radius; the UPDB's hardcoded self-shadow arguments are stale.
    let shadow = csm_self_shadow(world,vec3<f32>(material.detail_normal_uv_scale.x,0.0,material.detail_normal_uv_scale.z))*world_shadow_value(world);
    let light = globals.light_direction.xyz;
    let fresnel = pow(1.0-clamp(dot(normal,view_dir),0.0,1.0),material.m_params[1].w);
    let spec = phong_specular(normal,view_dir,light,material.m_params[2].w)*shadow;
    let final_spec = (spec*material.m_params[2].xyz + reflection_colour)*fresnel*specular;
    let ambient = vehicle_ambient(normal,material.m_params[5].w);
    let key = max(0.0,dot(normal,light))*shadow;
    if vehicle_diagnostic == 1u { return vec4<f32>(sqrt(max(diffuse.rgb,vec3<f32>(0.0))),1.0); }
    if vehicle_diagnostic == 2u { return vec4<f32>(csm_self_shadow(world,vec3<f32>(material.detail_normal_uv_scale.x,0.0,material.detail_normal_uv_scale.z)),world_shadow_value(world),max(0.0,dot(normal,light)),1.0); }
    if vehicle_diagnostic == 3u { return vec4<f32>(sqrt(ambient),1.0); }
    let colour = diffuse * (key*vec4<f32>(material.m_params[1].xyz,1.0))
        + vec4<f32>(ambient,1.0)*diffuse + vec4<f32>(final_spec,0.0);
    let tone = fog_bloom_tone(colour*material.m_params[0].y,input.fog,globals.envattributes2.x);
    return vec4<f32>(tone.rgb,object.i_params.x*select(1.0,material.m_params[4].w,glass));
}
@fragment fn vehiclePS(input: PsIn) -> @location(0) vec4<f32> { return vehicle_lighting(input,false); }
@fragment fn vehicleGhostPS(input: PsIn) -> @location(0) vec4<f32> {
    if object.i_params.x < 0.0039215689 { discard; }
    return vehicle_lighting(input,false);
}
@fragment fn vehicleGlassPS(input: PsIn) -> @location(0) vec4<f32> {
    // vehicle_glass.xml has blending but no alpha test.
    return vehicle_lighting(input,true);
}
@fragment fn vehicleLightPS(input: PsIn) -> @location(0) vec4<f32> {
    let sampled = textureSample(i_diffuse,wrap_sampler,input.uv);
    let diffuse = sampled*sampled;
    let colour = vec4<f32>(diffuse.rgb*material.m_params[1].xyz,diffuse.a)*material.m_params[0].x;
    return fog_bloom_tone(colour,input.fog,globals.envattributes2.x);
}
