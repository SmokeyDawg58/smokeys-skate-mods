// Snow-only weather adapter; rain uses the supplied friend renderer.
struct WeatherParameters {
    clip: mat4x4<f32>, inverse_clip: mat4x4<f32>, shelter_clip: mat4x4<f32>,
    camera: vec4<f32>, right: vec4<f32>, up: vec4<f32>, settings: vec4<f32>, fog: vec4<f32>, view: vec4<f32>,
}
@group(0) @binding(0) var<uniform> p: WeatherParameters;
@group(0) @binding(1) var scene: texture_2d<f32>;
@group(0) @binding(2) var depth: texture_depth_2d;
@group(0) @binding(3) var stencil: texture_2d<u32>;
@group(0) @binding(4) var linear_clamp: sampler;
@group(0) @binding(5) var shelter: texture_depth_2d;
// The original weather tint provides a daylight floor in this post-grade adapter.
fn atmosphere()->vec3<f32> {return max(p.fog.rgb,tint()*0.6);}
struct Screen { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, }
@vertex fn vs_surface(@builtin(vertex_index) id:u32)->Screen {
    let uv=vec2<f32>(f32((id<<1u)&2u),f32(id&2u));return Screen(vec4<f32>(uv.x*2.-1.,1.-uv.y*2.,0.,1.),uv);
}
fn relative_position(uv:vec2<f32>,z:f32)->vec3<f32> {
    let h=p.inverse_clip*vec4<f32>(uv.x*2.-1.,1.-uv.y*2.,z,1.);return h.xyz/h.w;
}
// Fractional coverage, not a binary shadow texel: snow and wetness blend at roofs.
@group(0) @binding(6) var shelter_compare: sampler_comparison;
fn sky_exposure(world:vec3<f32>)->f32 {
    let clip=p.shelter_clip*vec4<f32>(world,1.);let uv=clip.xy*vec2<f32>(0.5,-0.5)+0.5;
    if any(uv<vec2<f32>(0.)) || any(uv>vec2<f32>(1.)) {return 1.;}
    let texel=1./vec2<f32>(textureDimensions(shelter));var result=0.;
    for (var y=-1;y<=1;y+=1) {for (var x=-1;x<=1;x+=1) {
        let weight=select(1.,2.,x==0)*select(1.,2.,y==0);
        result+=weight*textureSampleCompareLevel(shelter,shelter_compare,uv+vec2<f32>(f32(x),f32(y))*texel,clip.z-0.15/160.);
    }}
    return result/16.;
}
fn sheltered(world:vec3<f32>)->bool {return sky_exposure(world)<0.5;}
fn character_mask(pixel:vec2<i32>)->f32 {return f32(textureLoad(stencil,pixel,0).x)/255.;}
// Select neighbour edges on the same surface instead of crossing a stair riser
// or an MSAA silhouette with a 2x2 derivative quad.
fn depth_position(pixel:vec2<i32>)->vec3<f32> {
    let size=vec2<i32>(textureDimensions(depth));
    let q=clamp(pixel,vec2<i32>(0),size-1);
    return relative_position((vec2<f32>(q)+0.5)/vec2<f32>(size),textureLoad(depth,q,0));
}
fn surface_normal(pixel:vec2<i32>,centre:vec3<f32>)->vec3<f32> {
    let l=centre-depth_position(pixel-vec2<i32>(1,0));
    let r=depth_position(pixel+vec2<i32>(1,0))-centre;
    let u=centre-depth_position(pixel-vec2<i32>(0,1));
    let d=depth_position(pixel+vec2<i32>(0,1))-centre;
    let dx=select(l,r,dot(r,r)<dot(l,l));
    let dy=select(u,d,dot(d,d)<dot(u,u));
    let n=cross(dy,dx);let squared=dot(n,n);
    if squared<1e-16 {return vec3<f32>(0.);}
    return n*inverseSqrt(squared);
}
@fragment fn fs_surface(v:Screen)->@location(0) vec4<f32> {
    let pixel=clamp(vec2<i32>(v.position.xy),vec2<i32>(0),vec2<i32>(textureDimensions(depth))-1);
    let z=textureLoad(depth,pixel,0);let relative=relative_position(v.uv,z);
    var normal=surface_normal(pixel,relative);
    if dot(normal,-relative)<0. {normal=-normal;}
    let original=textureSampleLevel(scene,linear_clamp,v.uv,0.);
    let cloud_amount=max(p.settings.x,select(0.,1.,p.settings.y>0.));
    if z>=0.999999 {
        return vec4<f32>(clouds(normalize(relative),p.settings.z,atmosphere())*gloom(cloud_amount),original.a);
    }
    // The original stencil protects characters/vehicles from ground snow.
    let coating_mask=1.-character_mask(pixel);
    let world=relative+p.camera.xyz;let exposure=sky_exposure(world);
    var colour=overcast(original.rgb*original.rgb,cloud_amount);
    if p.settings.y>0. && exposure>0. && coating_mask>0. {
        // Native baked irradiance is unavailable in this screen-space pass. Keep the scene's
        // shading contrast, with ambient light independent of dark ground albedo.
        let sky_light=min(sqrt(max(original.rgb,vec3<f32>(0.)))+0.35,vec3<f32>(1.));
        let amount=snow_cover(normal.y,world.xz,1.,1.)*exposure*coating_mask;
        colour=mix(colour,snow_light(sky_light)*snow_albedo(world.xz)*0.93,amount);
    }
    let haze=1.-exp(-length(relative)*0.006*cloud_amount);
    return vec4<f32>(mix(sqrt(max(colour,vec3<f32>(0.))),clouds(normalize(relative),p.settings.z,atmosphere()),haze)*gloom(cloud_amount),original.a);
}
struct Drop {
    @builtin(position) position:vec4<f32>,@location(0) streak:vec2<f32>,@location(1) alpha:f32,
    @location(2) @interpolate(flat) head:vec3<f32>,
}
@vertex fn vs_particle(@builtin(vertex_index) vertex:u32,@builtin(instance_index) instance:u32)->Drop {
    let corners=array<vec2<f32>,6>(vec2<f32>(-1.,0.),vec2<f32>(1.,0.),vec2<f32>(1.,1.),vec2<f32>(-1.,0.),vec2<f32>(1.,1.),vec2<f32>(-1.,1.));
    let corner=corners[vertex];let id=f32(instance);let snowing=p.settings.y>0.;
    var out=Drop(vec4<f32>(2.,2.,2.,1.),corner,0.,vec3<f32>(0.));
    if !snowing {return out;}
    if hash(vec2<f32>(id,1.))>=0.8*p.settings.y {return out;}
    let size=vec3<f32>(36.,24.,36.);
    let start=vec3<f32>(hash(vec2<f32>(id,2.)),hash(vec2<f32>(id,3.)),hash(vec2<f32>(id,4.)));
    let speed=hash(vec2<f32>(id,5.));
    let fall=select(vec3<f32>(1.,-9.,0.5)*(0.85+0.3*speed),vec3<f32>(0.3,-1.1,0.15)*(0.6+0.8*speed),snowing);
    let camera=p.camera.xyz;let clock=p.settings.z;
    let offset=(fract((start*size+fall*clock-camera)/size)-0.5)*size;
    let distance=max(length(offset),0.01);let edge=abs(offset)/(size*0.5);
    let near=select(vec2<f32>(0.4,1.5),vec2<f32>(1.2,3.5),snowing);
    let fade=(1.-smoothstep(0.6,1.,max(edge.x,max(edge.y,edge.z))))*smoothstep(near.x,near.y,distance);
    let pixel=distance*2./(p.view.z*p.view.x);var world:vec3<f32>;
    if snowing {
        let phase=hash(vec2<f32>(id,6.))*6.2832;
        let sway=vec3<f32>(sin(clock*1.3+phase),0.,cos(clock*1.1+phase))*0.35;
        let flake=max(0.03,pixel*1.5);let across=p.right.xyz*corner.x+p.up.xyz*(corner.y*2.-1.);
        out.head=camera+offset+sway;world=out.head+across*flake;out.alpha=0.8*(0.03/flake)*fade;
    }
    out.position=p.clip*vec4<f32>(world-camera,1.);return out;
}
@fragment fn fs_particle(i:Drop)->@location(0) vec4<f32> {
    let pixel=clamp(vec2<i32>(i.position.xy),vec2<i32>(0),vec2<i32>(textureDimensions(depth))-1);
    if i.alpha<1e-3 || i.position.z>textureLoad(depth,pixel,0)+1e-6 || sheltered(i.head) {discard;}
    if p.settings.y>0. {let q=vec2<f32>(i.streak.x,i.streak.y*2.-1.);return vec4<f32>(atmosphere()*2.,i.alpha*saturate(1.-dot(q,q)));}
    discard;
}
