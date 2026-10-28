//! FillWorld82e90e30/FillSkater82e91530, shared original collision Parts.
use super::{PedestrianBodyKind as Kind, PedestrianCollision};
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    physical_player_assembly::PhysicalAssemblyWorld, input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{entity_core::CommonEntityCore,waypoints::PropMan},
    reckoning_numeric::{HostReckoningNumeric,try_look_at_up_eye},
    reckoning_sin_cos::{HostReckoningSinCosNumeric,try_sin_cos},
    world::{orthonormalize_collision_affine,compose_collision_affine}};
type Matrix=[[f32;4];4];
fn flat(m:Matrix)->[f32;16]{std::array::from_fn(|i|m[i/4][i%4])}
fn matrix(m:[f32;16])->Matrix{std::array::from_fn(|i|std::array::from_fn(|j|m[i*4+j]))}
fn rotation(x_axis:bool,angle:f32)->[f32;16]{
    let(s,c)=try_sin_cos(angle,&mut HostReckoningSinCosNumeric).expect("infallible host SinCos");
    flat(if x_axis {[[1.,0.,0.,0.],[0.,c,s,0.],[0.,-s,c,0.],[0.;4]]}
        else {[[c,0.,-s,0.],[0.,1.,0.,0.],[s,0.,c,0.],[0.;4]]})
}
fn distance(a:[f32;4],b:[f32;4],r:f32)->f32{
    let delta=[a[0]-b[0],0.,a[2]-b[2],0.];
    crate::reckoning_numeric::host_dot3(delta,delta)-r*r
}
struct Frame<'a>{
    renderable:bool,interacting:bool,
    waypoint:Option<crate::living_world_grind_services::waypoints::WaypointRef>,
    state:&'a mut crate::living_world_pedestrian_state::PedestrianStateStorage,
    animation:&'a crate::living_world_pedestrian_entity::EntityAnimation,
    spatial:&'a crate::pedestrian_adjust_owners::SpatialAdjust,
    props:Option<&'a PropMan>,cvars:&'a InputCvarReferences,
}
impl Frame<'_>{
    fn fill(&mut self,kind:Kind,core:&CommonEntityCore,collision:&PedestrianCollision,world:&PhysicalAssemblyWorld)
        ->Result<Vec<[f32;16]>,String>{
        let model=collision.model.as_ref().ok_or("pedestrian fill collision model absent")?;
        let half_height=model.dimensions[1]*0.5;
        let from_pipe=match kind {
            Kind::World=>model.world_collision.as_ref().ok_or("world collision model absent")?,
            Kind::Skater=>&model.skater_collision,
        }.lock().map_err(|e|format!("collision definition lock: {e:?}"))?.from_pipe;
        let mut rotation=rotation(true,f32::from_bits(0x3fc9_0fdb));
        let mut base=core.target_transform;
        if kind==Kind::World {
            //Native starts from ACTUAL Entity.GetTransform before plugin cases.
            let mut result=flat(core.transform);
            if self.state.flags[2]&0x40!=0 {
                result=collision.part_transform(Kind::World,0,world)?;
                if !self.interacting && (self.state.timer(42)<=0. || !(distance(matrix(result)[3],core.transform[3],
                    self.cvars.try_pedestrian_plugin_unlock_tolerance().map_err(|e|format!("{e:?}"))?)>0.)) {
                    self.state.flags[2]&=!0x40;
                }
            }
            if self.state.flags[2]&0x40==0 {
                if self.state.flags[2]&0x80!=0 {
                    if let Some(reference)=self.waypoint {
                        let point=self.props.ok_or("plugin collision requires PropMan")?.point(reference)?;
                        let tolerance=self.cvars.try_pedestrian_plugin_lock_tolerance().map_err(|e|format!("{e:?}"))?;
                        if distance(point.location,core.transform[3],tolerance)!=0. {
                            result=collision.part_transform(Kind::World,0,world)?;
                            let shift=self.cvars.try_pedestrian_plugin_collision_body_shift_amount().map_err(|e|format!("{e:?}"))?;
                            for i in 0..4 { result[12+i]=(point.location[i]+point.orientation[i]*shift)+if i==1 {half_height}else{0.}; }
                            self.state.flags[2]=(self.state.flags[2]&!0x80)|0x40;
                        }
                    }
                }else{
                    if self.renderable {
                        let bone=self.animation.sim_lod_bone_global.as_ref().ok_or("renderable world fill needs actual SimLOD bones")?.matrices[8];
                        if collision.colliding {base=bone;rotation=rotation_y_negative_half_pi();}
                        else {base[3]=bone[3];}
                    }else if !from_pipe {
                        for i in 0..4 {base[3][i]=(base[3][i]+[0.,1.,0.,0.][i]*half_height)+[0.,1.,0.,0.][i]*0.2;}
                    }
                    result=compose_collision_affine(orthonormalize_collision_affine(flat(base)),rotation);
                }
            }
            if self.cvars.try_pedestrian_jump_shift_collision().map_err(|e|format!("{e:?}"))? && self.spatial.jump.is_jumping {
                result[13]+=self.cvars.try_pedestrian_jump_shift_amount().map_err(|e|format!("{e:?}"))?;
            }
            return Ok(vec![result]);
        }
        let mut arms=[base;2];
        let mut retained_main=None;
        if self.renderable {
            let bones=&self.animation.sim_lod_bone_global.as_ref().ok_or("renderable skater fill needs actual SimLOD bones")?.matrices;
            //SpatialAdjust+220 bit08 is DidSlideThisFrame, not position lock.
            if self.spatial.did_slide {retained_main=Some(collision.skater_part_transform(0,world)?);}
            else {
                let hips=bones[7][3];
                let look=std::array::from_fn(|i|hips[i]+bones[7][2][i]);
                let up=std::array::from_fn(|i|bones[0][3][i]-hips[i]);
                base=try_look_at_up_eye(look,up,hips,&mut HostReckoningNumeric).map_err(|e|format!("pedestrian body look: {e:?}"))?;
            }
            for (arm,(a,b)) in arms.iter_mut().zip([(4,5),(1,2)]) {
                *arm=bones[a];arm[3]=std::array::from_fn(|i|(bones[a][3][i]+bones[b][3][i])*0.5);
            }
        }else if !from_pipe {
            for body in std::iter::once(&mut base).chain(arms.iter_mut()) {
                for i in 0..4 {body[3][i]=(body[3][i]+[0.,1.,0.,0.][i]*half_height)+[0.,1.,0.,0.][i]*0.2;}
            }
        }
        let mut matrices=[base,arms[0],arms[1]].map(|m|orthonormalize_collision_affine(flat(m)));
        for m in &mut matrices {for i in 0..4 {m[12+i]+=core.velocity[i]*f32::from_bits(0x3c88_8889);}}
        matrices[0]=if let Some(part)=retained_main{part}else{compose_collision_affine(matrices[0],rotation)};
        Ok(matrices.to_vec())
    }
}
fn rotation_y_negative_half_pi()->[f32;16]{rotation(false,f32::from_bits(0xbfc9_0fdb))}
impl PedestrianConstruction{
    /// Reuses cLivingWorldCollision.AddModels82e5cd40's existing ordered
    /// producer/admission owner. All mutable state survives a partial failure.
    pub(crate) fn add_collision_models_to_world(&mut self,world:&mut PhysicalAssemblyWorld,
        allow_world:bool,world_state:u32,skater_state:u32,cvars:&InputCvarReferences,props:Option<&PropMan>)
        ->Result<(),String>{
        // BigBlack.AddModelsToWorld82e704a0 ignores the caller's collision
        // choices and always admits both bodies as ACTIVE_BODY (4).
        let (allow_world,world_state,skater_state)=if self.big_black.is_some(){(true,4,4)}
            else{(allow_world,world_state,skater_state)};
        let mut frame=Frame {renderable:self.renderable,interacting:self.interacting_object.is_some(),waypoint:self.current_waypoint,
            state:self.state.as_mut().ok_or("collision fill AI state absent")?,
            animation:self.animation.as_ref().ok_or("collision fill animation absent")?,
            spatial:self.spatial_adjust.as_ref().ok_or("collision fill spatial owner absent")?,props,cvars};
        self.collision.as_mut().ok_or("collision fill object absent")?.add_models_to_world(
            self.core.as_mut().ok_or("collision fill core absent")?,world,allow_world,world_state,skater_state,
            |kind,core,collision,world|frame.fill(kind,core,collision,world))
    }
    pub(crate) fn update_collision_models(&mut self,world:&mut PhysicalAssemblyWorld,dt:f32,
        cvars:&InputCvarReferences,props:Option<&PropMan>)->Result<(),String>{
        let mut frame=Frame {renderable:self.renderable,interacting:self.interacting_object.is_some(),waypoint:self.current_waypoint,
            state:self.state.as_mut().ok_or("collision fill AI state absent")?,
            animation:self.animation.as_ref().ok_or("collision fill animation absent")?,
            spatial:self.spatial_adjust.as_ref().ok_or("collision fill spatial owner absent")?,props,cvars};
        self.collision.as_ref().ok_or("collision fill object absent")?.update_collision_transforms(
            self.core.as_ref().ok_or("collision fill core absent")?,world,dt,cvars,
            |kind,core,collision,world|frame.fill(kind,core,collision,world))
    }
}
