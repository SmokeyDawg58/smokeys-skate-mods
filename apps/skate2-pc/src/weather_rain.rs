//! Saved Underground rain look: world-anchored, wind-driven ribbons using the native pass.

use crate::{
    weather_runtime::{MAX_INTENSITY, Snapshot, WIND},
    weather_world::{World, cross, dot, normalized, sub},
};
use skate2_core::world_particles::{ParticleCamera, ParticleVertex};

pub(crate) const TEXTURE_ID: u64 = 0x5355_5241_494e_0001;
const RADIUS: f64 = 40.0;
const HEIGHT: f64 = 23.2;
const DENSITY: f64 = 42.0;

/// Cosmetic weather advances on every presented frame, including repeated native
/// presentation samples. This clock never supplies simulation or game audio time.
#[derive(Default)]
pub(crate) struct PresentationClock {
    previous:Option<std::time::Instant>, seconds:f64, report_seconds:f64,
    frames:u32,native_repeats:u32,native_previous:Option<f32>,
}
impl PresentationClock {
    pub(crate) fn sample(&mut self,native:f32)->(f32,f32) {
        let now=std::time::Instant::now();
        let elapsed=self.previous.map_or(1.0/120.0,|last|now.saturating_duration_since(last).as_secs_f64());
        self.previous=Some(now);self.advance(elapsed,native)
    }
    fn advance(&mut self,elapsed:f64,native:f32)->(f32,f32) {
        // A suspended/minimized renderer resumes locally, without catching up a
        // whole paused interval in one visible frame.
        let delta=if !elapsed.is_finite()||elapsed<0.0 {0.0}else if elapsed>0.25{1.0/120.0}else{elapsed};
        self.seconds+=delta;self.report_seconds+=delta;self.frames+=1;
        self.native_repeats+=u32::from(self.native_previous.is_some_and(|last|native<=last));self.native_previous=Some(native);
        if self.report_seconds>=1.0 {
            if std::env::var_os("SKATE2_DIAGNOSTIC_RAIN").is_some() {
                eprintln!("rain presentation clock: frames={} native_repeats={} elapsed={:.4}; weather uses render time",self.frames,self.native_repeats,self.report_seconds);
            }
            self.report_seconds=0.0;self.frames=0;self.native_repeats=0;
        }
        (self.seconds as f32,delta as f32)
    }
}

#[derive(Default)]
pub(crate) struct Rain {offset:[f64;3],drops:Vec<Particle>,world_key:(usize,usize),epoch:Option<f64>,
    world_generation:u64,frame:u64,scratch:Vec<(f32,[ParticleVertex;4])>}
#[derive(Clone,Copy)]struct Cover {origin:[f32;3],clear:bool,until:f32,generation:u64}
#[derive(Clone,Copy)]struct Particle {point:[f64;3],wind:[f32;3],cover:Option<Cover>,visibility:f32}
/// Authored coherent gusts. Each new droplet keeps its birth velocity, so the
/// front moves through the rain naturally and exact trajectory cover stays cached.
fn wind_at(time:f64,seed:u32)->[f32;3] {
    let phase=(time+3.5).rem_euclid(17.0);
    let gust=if (4.0..11.0).contains(&phase){((phase-4.0)/7.0*std::f64::consts::PI).sin().powi(2)}else{0.0};
    let bend=(time*0.19).sin();let variation=(random(seed^0xf192_3a67)-0.5)*0.7;
    [WIND[0]+(gust*(6.4+bend*1.4))as f32+variation as f32,WIND[1],WIND[2]+(gust*(2.7+(time*0.23).cos()))as f32]
}
fn hash(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}
fn random(value: u32) -> f64 {
    f64::from(hash(value) >> 8) / 16_777_216.0
}

impl Rain {
    pub(crate) fn vertices(&mut self,camera:&ParticleCamera,dt:f32,state:Snapshot,world:&World)->Vec<ParticleVertex>{
        if !camera.position.iter().all(|v|v.is_finite()){return Vec::new();}
        let dt=if dt.is_finite(){dt.clamp(0.,0.25)}else{0.};
        let previous_offset=self.offset;
        for(offset,velocity)in self.offset.iter_mut().zip(WIND){*offset+=f64::from(dt)*f64::from(velocity);}
        let elapsed=self.offset[1]/f64::from(WIND[1]);
        let epoch=*self.epoch.get_or_insert_with(||if state.time.is_finite(){f64::from(state.time)}else{0.});let now=epoch+elapsed;
        if state.intensity<=0.0001{return Vec::new();}
        let strength=(state.intensity/MAX_INTENSITY).clamp(0.,1.);
        let count=(DENSITY*(2.*RADIUS).powi(2)*HEIGHT/1000.*f64::from(strength)).round()as usize;
        let key=(world as*const World as usize,world.triangles.len());
        // Streamed map updates formerly invalidated every drop at once. Spread
        // cover refresh over several frames and retain the last known cover in
        // the meantime, avoiding thousands of simultaneous BVH queries.
        if self.world_key!=key{self.world_key=key;self.world_generation=self.world_generation.wrapping_add(1);}
        self.frame=self.frame.wrapping_add(1);let generation=self.world_generation;let frame=self.frame;
        let mut ray_budget=384_usize;
        let size=[RADIUS*2.,HEIGHT,RADIUS*2.];
        let low=[f64::from(camera.position[0])-RADIUS,f64::from(camera.position[1])-HEIGHT/3.,f64::from(camera.position[2])-RADIUS];
        while self.drops.len()<count{
            let seed=hash(self.drops.len()as u32+0x5355_5241);let seeds=[random(seed),random(seed^0x68bc_21eb),random(seed^0xa361_2d9b)];
            let point=std::array::from_fn(|i|low[i]+(seeds[i]*size[i]+previous_offset[i]-low[i]).rem_euclid(size[i]));
            self.drops.push(Particle{point,wind:wind_at(now-f64::from(dt),seed),cover:None,visibility:0.});
        }
        self.drops.truncate(count);let mut output=std::mem::take(&mut self.scratch);output.clear();
        if output.capacity()<count {output.reserve(count);}
        for(index,p)in self.drops.iter_mut().enumerate(){
            let seed=hash(index as u32+0x5355_5241);
            for i in 0..3{p.point[i]+=f64::from(p.wind[i])*f64::from(dt);}
            let top=low[1]+HEIGHT;
            if p.point[1]<low[1]||p.point[1]>=top{
                // Preserve phase in both directions. Clamping above-top drops to
                // the top synchronized a camera-cut cohort into a repeating sheet.
                let wrapped=low[1]+(p.point[1]-low[1]).rem_euclid(HEIGHT);
                let residual=(top-wrapped)/-f64::from(WIND[1]);
                let old_wind=p.wind;p.wind=wind_at(now-residual,seed);
                for i in [0,2]{p.point[i]+=f64::from(p.wind[i]-old_wind[i])*residual;}
                p.point[1]=wrapped;
                p.cover=None;
            }
            for i in [0,2]{if p.point[i]<low[i]||p.point[i]>=low[i]+size[i]{p.point[i]=low[i]+(p.point[i]-low[i]).rem_euclid(size[i]);p.cover=None;}}
            let point=p.point.map(|v|v as f32);let relative=sub(point,camera.position);
            if dot(relative,camera.forward)<0.0{continue;}
            let distance=relative[0].hypot(relative[2]);
            let fade=((40.-distance)/6.).clamp(0.,1.)*((distance-0.65)/1.5).clamp(0.,1.)
                *((point[1]-low[1]as f32)/1.5).clamp(0.,1.)*((top as f32-point[1])/1.5).clamp(0.,1.);
            if fade<=0.{continue;}
            let fall=p.wind.map(|v|v/-p.wind[1]);let sky=normalized(p.wind.map(|v|-v));
            let cached=p.cover.filter(|c|{let delta=sub(point,c.origin);let along=dot(delta,fall)/dot(fall,fall);let error=sub(delta,fall.map(|v|v*along));along>=0.&&along<c.until&&dot(error,error)<0.000001});
            let refresh=cached.is_none() || cached.is_some_and(|c|c.generation!=generation && (index as u64+frame)%16==0);
            let cover=if refresh && ray_budget>0 {
                ray_budget-=1;Cover{origin:point,clear:world.ray_distance(point,sky,10_000.).is_none(),until:world.ray_distance(point,fall,10_000.).unwrap_or(10_000.),generation}
            }else if let Some(previous)=cached {previous}else{continue;};
            p.cover=Some(cover);
            if !cover.clear{p.visibility=0.;continue;}
            p.visibility=(p.visibility+dt*6.).min(1.);
            let along=dot(sub(point,cover.origin),fall)/dot(fall,fall);
            let variant=random(seed^0x02e5_be93)as f32;
            let length=(cover.until-along-0.015).min(0.22+0.54*variant);
            if length<=0.{continue;}
            let mut side=normalized(cross(relative.map(|v|-v),fall));if dot(side,side)<0.5{side=camera.right;}
            let physical_width=(0.0045+0.009*variant)*0.5;
            let width=physical_width.max(dot(relative,relative).sqrt()*0.00045);side=side.map(|v|v*width);
            let tail:[f32;3]=std::array::from_fn(|i|point[i]+fall[i]*length);
            let positions=[std::array::from_fn(|i|point[i]-side[i]),std::array::from_fn(|i|point[i]+side[i]),std::array::from_fn(|i|tail[i]+side[i]),std::array::from_fn(|i|tail[i]-side[i])];
            let alpha=(0.16+0.16*random(seed^0x19ab_cdef)as f32)*fade*strength.sqrt()*(physical_width/width)*p.visibility;
            let quad=std::array::from_fn::<_,4,_>(|i|ParticleVertex{position:positions[i],color:[0.79,0.80,0.82,alpha],uv:[[0.,0.],[1.,0.],[1.,1.],[0.,1.]][i]});
            output.push((dot(relative,relative),quad));
        }
        output.sort_unstable_by(|a,b|b.0.total_cmp(&a.0));
        let vertices=output.iter().flat_map(|(_,q)|q.iter().copied()).collect();self.scratch=output;vertices
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "soft streak alpha is clamped by construction to 0..255 UNORM"
)]
pub(crate) fn texture() -> skate2_core::texture::NativeTexture {
    use skate2_formats::material::{
        LinearTextureLevel, PIXEL_FORMAT_A8R8G8B8, TextureEndian, TextureType,
    };
    let mut data = Vec::with_capacity(8 * 32 * 4);
    for y in 0..32_u16 {
        for x in 0..8_u16 {
            let u = f32::from(x) / 7.0;
            let v = f32::from(y) / 31.0;
            let alpha =
                (1.0 - (u * 2.0 - 1.0).abs()).powi(2) * (std::f32::consts::PI * v).sin().powi(2);
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0).round() as u8]);
        }
    }
    skate2_core::texture::NativeTexture {
        dictionary_index: 0,
        guid: TEXTURE_ID,
        name: "Underground.Rain".into(),
        texture_type: TextureType::TwoDimensional,
        endian: TextureEndian::None,
        width: 8,
        height: 32,
        depth: 1,
        mip_levels: 1,
        pixel_format: PIXEL_FORMAT_A8R8G8B8,
        packed_mips: false,
        mip_data_offset: 0,
        encoded_data: Vec::new(),
        linear_levels: vec![LinearTextureLevel {
            level: 0,
            face: 0,
            width: 8,
            height: 32,
            width_in_blocks: 8,
            height_in_blocks: 32,
            bytes_per_block: 4,
            row_pitch: 32,
            data,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn camera(position: [f32; 3]) -> ParticleCamera {
        ParticleCamera {
            position,
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            forward: [0.0, 0.0, 1.0],
        }
    }
    #[test]
    fn presented_rain_keeps_moving_when_native_samples_repeat_or_rewind() {
        let mut clock=PresentationClock::default();let mut rain=Rain::default();let world=World::default();let camera=camera([0.,2.,0.]);let mut previous:Option<f64>=None;
        for frame in 0..240 {
            // Source presentation remains frozen, then rewinds. Its former
            // max(native-last,0) driver would repeatedly supply a zero delta.
            let native=if frame<120 {2.0}else{1.0};let(time,delta)=clock.advance(1.0/120.0,native);
            assert!(delta>0.0);let vertices=rain.vertices(&camera,delta,Snapshot{intensity:1.,time,..Default::default()},&world);assert!(!vertices.is_empty());
            let point=rain.drops[0].point;
            if let Some(last)=previous {let movement=(point[1]-last).abs();assert!(movement>0.10,"rain must move on every presented frame");}
            previous=Some(point[1]);
        }
        assert!((clock.seconds-2.0).abs()<0.00001);
    }
    #[test]
    fn rain_resume_does_not_jump_a_suspended_interval() {
        let mut clock=PresentationClock::default();clock.advance(1.0/120.0,0.0);let(before,_)=clock.advance(1.0/120.0,0.0);let(after,delta)=clock.advance(20.0,0.0);
        assert!(delta>0.0&&delta<0.02&&after-before<0.02);
    }
    #[test]fn coherent_gust_changes_angle_without_unbounded_wind(){
        let mut max=0_f32;let mut min=100_f32;
        for i in 0..170{let w=wind_at(f64::from(i)*0.1,4);assert!(w.iter().all(|v|v.is_finite()));assert_eq!(w[1],WIND[1]);max=max.max(w[0]);min=min.min(w[0]);}
        assert!(max-min>5.0&&max<10.0);
    }
    #[test]
    fn saved_look_is_bounded_and_stable_across_city_teleports() {
        let mut rain = Rain::default();
        let world = World::default();
        let state = Snapshot {
            intensity: MAX_INTENSITY,
            ..Snapshot::default()
        };
        for position in [[0.0, 2.0, 0.0], [10_000.0, 2.0, 0.0], [-10_000.0, 2.0, 0.0]] {
            let vertices = rain.vertices(&camera(position), 1.0 / 60.0, state, &world);
            assert!(!vertices.is_empty() && vertices.len() < 6500 * 4);
            assert!(
                vertices
                    .iter()
                    .all(|v| v.position.iter().all(|f| f.is_finite())
                        && (v.position[0] - position[0]).abs() < 41.0)
            );
        }
        assert!(
            rain.vertices(&camera([0.0, 2.0, 0.0]), 0.0, Snapshot::default(), &world)
                .is_empty()
        );
    }
    #[test]
    fn frame_rate_does_not_change_fall_speed() {
        let mut a = Rain::default();
        let mut b = Rain::default();
        let world = World::default();
        let state = Snapshot {
            intensity: 0.6,
            ..Snapshot::default()
        };
        let camera = camera([0.0, 2.0, 0.0]);
        for _ in 0..30 {
            a.vertices(&camera, 1.0 / 30.0, state, &world);
        }
        for _ in 0..60 {
            b.vertices(&camera, 1.0 / 60.0, state, &world);
        }
        for i in 0..3 {
            assert!((a.offset[i] - b.offset[i]).abs() < 1e-6);
        }
        for (a,b) in a.drops.iter().zip(&b.drops){for i in 0..3{assert!((a.point[i]-b.point[i]).abs()<0.002,"particle motion differs by frame rate");}}
    }
    #[test]
    fn streamed_ground_refresh_spreads_cover_queries_across_frames() {
        let mut rain=Rain::default();let camera=camera([0.,2.,0.]);let a=World::default();let b=World::default();
        let state=Snapshot{intensity:1.,..Snapshot::default()};
        rain.vertices(&camera,1./120.,state,&a);
        assert!(rain.drops.iter().filter(|p|p.cover.is_some()).count()<=384);
        for _ in 0..24 {rain.vertices(&camera,1./120.,state,&a);}
        let cached=rain.drops.iter().filter(|p|p.cover.is_some()).count();assert!(cached>500);
        rain.vertices(&camera,1./120.,state,&b);let generation=rain.world_generation;
        assert!(rain.drops.iter().filter(|p|p.cover.is_some_and(|c|c.generation==generation)).count()<=384);
        assert!(rain.drops.iter().any(|p|p.cover.is_some_and(|c|c.generation!=generation)),"old cover remains while replacement warms");
        for _ in 0..32 {rain.vertices(&camera,1./120.,state,&b);}
        assert!(rain.drops.iter().filter(|p|p.cover.is_some_and(|c|c.generation==generation)).count()>500);
    }
    #[test]
    fn rain_does_not_query_or_emit_ribbons_wholly_behind_the_camera() {
        let mut rain=Rain::default();let camera=camera([0.0,2.0,0.0]);
        let vertices=rain.vertices(&camera,0.0,Snapshot{intensity:1.0,..Snapshot::default()},&World::default());
        assert!(vertices.len()/4<3500);
        assert!(vertices.chunks_exact(4).all(|quad| quad.iter().any(|v| dot(sub(v.position,camera.position),camera.forward)>-0.8)));
        assert!(vertices.chunks_exact(4).any(|quad| quad[0].position[2]>20.0));
    }

}

#[cfg(test)]mod burst_regression {
use super::*;
#[test]fn camera_drop_must_not_synchronize_raindrops_into_a_sheet(){
 let world=World::default();let mut rain=Rain::default();let mut worst=0;let mut count=0;
 for frame in 0..720 {
  let time=frame as f32/120.;let y=if frame<120{12.}else{2.+(time*1.5).sin()*0.3};
  let camera=ParticleCamera{position:[0.,y,0.],right:[1.,0.,0.],up:[0.,1.,0.],forward:[0.,0.,1.]};
  rain.vertices(&camera,1./120.,Snapshot{intensity:1.,time,..Default::default()},&world);
  let low=f64::from(y)-HEIGHT/3.;let mut bins=[0usize;64];
  for drop in &rain.drops{let bin=(((drop.point[1]-low)/HEIGHT*64.).floor()as usize).min(63);bins[bin]+=1;}
  count=rain.drops.len();if frame>=120{worst=worst.max(*bins.iter().max().unwrap());}
 }
 println!("camera-drop vertical concentration: worst_bin={worst} particles={count} fraction={:.4}",worst as f64/count as f64);
 assert!(worst<count/20,"camera motion must preserve each droplet phase, not turn a cap into a synchronized rain sheet");
}
}
