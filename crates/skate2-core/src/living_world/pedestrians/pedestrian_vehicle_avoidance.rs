//! VehicleAvoidanceNavModifier::RecommendTarget82e8c600.
use super::{VehicleModifier,EntityOwner,RecommendationFlags};
use crate::{living_world_grind_services::vehicle_motion::nav_rig::VehicleNavRigValidator,
 living_world_pedestrian_state::PedestrianStateStorage,living_world_chase::TargetableId,
 pedestrian_intent_packet::{IntentStage,queue::IntentQueue},play_animation::FastString30,
 reckoning_numeric::{ReckoningNumeric,try_normalize_unsafe,try_length,length_from_squared_and_estimate}};
pub(crate) struct VehicleTuning{pub(crate) jump_distance:f32,pub(crate) jump_minimum_speed:f32,pub(crate) flee_distance:f32,pub(crate) flee_speed:f32}
pub(crate) struct VehicleTarget<'a>{pub(crate) id:u32,pub(crate) position:[f32;4],pub(crate) velocity:[f32;4],pub(crate) breaking_rules:bool,pub(crate) rig:Option<&'a VehicleNavRigValidator>}
fn sub(a:[f32;4],b:[f32;4])->[f32;4]{std::array::from_fn(|i|a[i]-b[i])}
fn flat(a:[f32;4])->[f32;4]{[a[0],0.,a[2],0.]}
/// geotest::LineSphereIntersection82e4e460 is an unbounded unit-heading
/// ray query; the rwcollision segment helper has different inside/end gates.
fn line_sphere<N:ReckoningNumeric>(start:[f32;4],heading:[f32;4],center:[f32;4],radius:f32,n:&mut N)->Result<(bool,f32),N::Error>{
 let delta=sub(center,start);let squared=n.dot3(delta,delta)?;let projection=n.dot3(delta,heading)?;
 let discriminant=projection.mul_add(projection,radius*radius-squared);
 let estimate=n.reciprocal_sqrt_estimate(discriminant)?;
 Ok((discriminant>=0.,projection-length_from_squared_and_estimate(discriminant,estimate)))
}
impl VehicleModifier{
 pub(crate) fn recommend<N:ReckoningNumeric>(&mut self,position:[f32;4],velocity:[f32;4],interim_target:[f32;4],on_road:bool,
  target:VehicleTarget<'_>,t:&VehicleTuning,queue:&mut IntentQueue,state:&mut PedestrianStateStorage,n:&mut N)->Result<(),String> where N::Error:std::fmt::Debug{
  self.target_entity=Some(EntityOwner::LivingWorld(target.id));self.flags=RecommendationFlags::default();self.distance=30.;
  if !self.enabled{return Ok(());}
  if on_road&&target.breaking_rules{
   let delta=sub(position,target.position);
   if n.dot3(delta,delta).map_err(err)?-t.flee_distance*t.flee_distance<0.{
    if n.dot3(flat(sub(target.position,position)),target.velocity).map_err(err)?>t.flee_speed{
     for want in &mut state.wants{want.active=false;}
     state.wants[10].target=TargetableId{kind:3,id:target.id as i32};state.wants[10].active=true;state.wants[10].explicit=false;
     return Ok(());
    }
   }
  }
  let delta=sub(interim_target,position);
  let mut heading=try_normalize_unsafe(flat(delta),n).map_err(err)?;
  //Native CR6.EQ tests all-false after replacing W with the X lane.
  if !delta[..3].iter().any(|v|v.abs()>f32::from_bits(0x34000000)){
   heading=try_normalize_unsafe(flat(velocity),n).map_err(err)?;
  }
  let mut start=position;start[1]+=1.5;
  let rig=target.rig.ok_or("vehicle avoidance requires actual vehicle NavRig validator")?;
  for sphere in 0..4{
   //Native computes this bound from retained mDistance splat, not from
   //the sphere center; preserve that source expression despite its name.
   let bound=try_length(sub([self.distance;4],start),n).map_err(err)?*4.;
   let mut center=rig.sphere_positions[sphere];center[1]=start[1];
   let(hit,distance)=line_sphere(start,heading,center,rig.sphere_radii[sphere],n).map_err(err)?;
   if !(hit&&bound>distance&&distance>=0.){continue;}
   if t.jump_distance>distance&&n.dot3(target.velocity,target.velocity).map_err(err)?-t.jump_minimum_speed*t.jump_minimum_speed>0.{
    self.flags.active=true;self.flags.intent=true;
    let mut stage=IntentStage::default();for name in ["Avoid","Jump","Back"]{
     stage.add_intent(FastString30::encode(name).expect("native vehicle avoid literal"))?;
    }
    stage.rotate_to_target=false;queue.packet_mut(self.packet)?.replace_single_stage(stage);
   }
   let mut nearest=0;let mut distance=try_length(sub(start,rig.line_starts[0]),n).map_err(err)?;
   for index in 1..4{
    let delta=sub(start,rig.line_starts[index]);
    if n.dot3(delta,delta).map_err(err)?-distance*distance<=0.{nearest=index;distance=try_length(delta,n).map_err(err)?;}
   }
   self.target=rig.line_starts[nearest];self.flags.target=true;self.flags.active=true;
  }
  Ok(())
 }
}
fn err(e:impl std::fmt::Debug)->String{format!("vehicle avoidance: {e:?}")}
