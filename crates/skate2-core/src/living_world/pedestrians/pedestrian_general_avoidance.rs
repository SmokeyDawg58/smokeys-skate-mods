//! GeneralNavModifier82e93d40 and rules of the road82e76a30.
use super::{GeneralModifier,EntityOwner,RecommendationFlags};
use crate::{reckoning_numeric::{ReckoningNumeric,try_length,try_signed_angle},
 pedestrian_intent_packet::queue::IntentQueue};
pub(crate) struct GeneralTuning{
 pub(crate) height_tolerance:f32,pub(crate) avoid_speed:f32,pub(crate) road_speed:f32,
 pub(crate) outer_radius:f32,pub(crate) inner_radius:f32,pub(crate) front_half_angle:f32,
 pub(crate) intersect_tolerance:f32,pub(crate) oncoming_angle:f32,pub(crate) oncoming_width:f32,
 pub(crate) oncoming_avoid_angle:f32,pub(crate) oncoming_avoid_distance:f32,
}
pub(crate) struct GeneralTarget{
 pub(crate) owner:EntityOwner,pub(crate) position:[f32;4],pub(crate) velocity:[f32;4],
}
#[derive(Clone,Copy)]
enum Situation{None,Avoid,Stop,Oncoming}
fn flat(v:[f32;4])->[f32;4]{[v[0],0.,v[2],0.]}
fn sub(a:[f32;4],b:[f32;4])->[f32;4]{std::array::from_fn(|i|a[i]-b[i])}
/// geotest::LineSegmentTestXZ82e54ae8. Raw scalar loads distinguish two
/// numerators hidden by decompiler stack aliasing. Endpoints are excluded.
fn segment_test(a:[f32;4],b:[f32;4],c:[f32;4],d:[f32;4])->Option<[f32;4]>{
 let dx1=b[0]-a[0];let dz1=b[2]-a[2];let dx2=d[0]-c[0];let dz2=d[2]-c[2];
 let denominator=dz2*dx1-dx2*dz1;
 if !(denominator.abs()>f32::from_bits(0x34000000)){return None;}
 let dx=a[0]-c[0];let dz=a[2]-c[2];let reciprocal=1./denominator;
 let u=(dx1*dz-dz1*dx)*reciprocal;let t=(dx2*dz-dz2*dx)*reciprocal;
 if t<=0.||t>=1.||u<=0.||u>=1.{return None;}
 Some([dx1.mul_add(t,a[0]),0.,dz1.mul_add(t,a[2]),0.])
}
impl GeneralModifier{
 pub(crate) fn recommend<N:ReckoningNumeric>(&mut self,my_position:[f32;4],my_velocity:[f32;4],
  my_facing:[f32;4],my_right:[f32;4],target:GeneralTarget,t:&GeneralTuning,queue:&mut IntentQueue,numeric:&mut N)
  ->Result<(),String> where N::Error:std::fmt::Debug{
  self.target_entity=Some(target.owner);self.flags=RecommendationFlags::default();
  if !self.enabled{return Ok(());}
  if !(t.height_tolerance>(target.position[1]-my_position[1]).abs()){return Ok(());}
  let speed=numeric.dot3(flat(target.velocity),flat(target.velocity)).map_err(err)?;
  let situation=if speed<=t.avoid_speed*t.avoid_speed{
   self.resolve_road(my_position,my_velocity,my_right,target.position,target.velocity,t,numeric)?
  }else if self.should_avoid(my_position,target.position,target.velocity,t.outer_radius,numeric).map_err(err)?{Situation::Avoid}
  else{Situation::None};
  match situation{
   Situation::None=>{},Situation::Stop=>self.handle_burdened_stop(),
   Situation::Avoid=>self.handle_burdened_avoid(my_facing,t.inner_radius,t.front_half_angle,queue,numeric)?,
   Situation::Oncoming=>{
    self.flags.active=true;self.flags.target=true;
    let mut trig=crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
    let rotated=crate::physical_player_grind_manager::math::rotate_vector([0.,1.,0.,0.],
     t.oncoming_avoid_angle*(-std::f32::consts::PI/180.),flat(my_velocity),&mut trig).map_err(err)?;
    self.target=std::array::from_fn(|i|rotated[i].mul_add(t.oncoming_avoid_distance,my_position[i]));
   }
  }
  Ok(())
 }
 fn resolve_road<N:ReckoningNumeric>(&self,my_position:[f32;4],my_velocity:[f32;4],my_right:[f32;4],
  target_position:[f32;4],target_velocity:[f32;4],t:&GeneralTuning,numeric:&mut N)->Result<Situation,String>
  where N::Error:std::fmt::Debug{
  let velocity=flat(target_velocity);let mine=flat(my_velocity);
  if numeric.dot3(velocity,velocity).map_err(err)?-t.road_speed*t.road_speed<=0.{return Ok(Situation::None);}
  let a=flat(target_position);let b=flat(my_position);let delta=sub(a,b);
  let ahead=numeric.dot3(mine,delta).map_err(err)?;
  if !(ahead>0.){return Ok(Situation::None);}
  // Source uses separately rounded multiplication then addition here.
  let target_end=flat(std::array::from_fn(|i|target_position[i]+target_velocity[i]*self.predict_time));
  let my_end=flat(std::array::from_fn(|i|my_position[i]+my_velocity[i]*self.predict_time));
  let Some(hit)=segment_test(a,target_end,b,my_end) else{
   let right=flat(my_right);
   let width=(numeric.dot3(right,delta).map_err(err)?+numeric.dot3(right,velocity).map_err(err)?).abs();
   return Ok(if t.oncoming_width>width{Situation::Oncoming}else{Situation::None});
  };
  let distance_delta=(try_length(sub(a,hit),numeric).map_err(err)?-try_length(sub(b,hit),numeric).map_err(err)?).abs();
  if !(t.intersect_tolerance>distance_delta){return Ok(Situation::None);}
  let approaching=numeric.dot3(velocity,delta.map(|v|-v)).map_err(err)?;
  if approaching>0.{
   let mut angle=try_signed_angle(mine,velocity.map(|v|-v),[0.,1.,0.,0.],numeric).map_err(err)?;
   if angle>std::f32::consts::PI{angle-=std::f32::consts::TAU;}
   if t.oncoming_angle*(std::f32::consts::PI/180.)>angle.abs(){return Ok(Situation::Oncoming);}
   if 0.>angle{return Ok(Situation::Stop);}
  }else if ahead> -approaching{return Ok(Situation::Stop);}
  Ok(Situation::None)
 }
}
fn err(e:impl std::fmt::Debug)->String{format!("general avoidance: {e:?}")}
