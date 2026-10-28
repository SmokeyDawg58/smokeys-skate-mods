//! Actual animation, target and Hermes owners for PedestrianColliding.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction as Entity,living_world_chase::TargetableId,
 attribute_runtime::AttributeRuntimeInstance,pedestrian_graph::colliding::{CollidingServices,CollisionMoodEvent},
 hermes_game_sim_queue::{LivingWorldMessage,MoodEvent}};
impl CollidingServices for FrameServices<'_>{
 fn animation_data(&mut self,e:&Entity)->Result<AttributeRuntimeInstance,String>{Ok(e.animation.as_ref().ok_or("collision animation absent")?.selected_animation_data()?.clone())}
 fn transitioning(&mut self,e:&Entity)->Result<bool,String>{e.animation.as_ref().ok_or("collision animation absent")?.core.try_is_transitioning().map_err(|e|format!("transition: {e:?}"))}
 fn animation_time(&mut self,e:&Entity)->Result<f32,String>{
  let a=e.animation.as_ref().ok_or("collision animation absent")?;
  let base=a.core.base().ok_or("collision animation base absent")?;
  a.core.body_reference_clock(base.body_root.as_ref().ok_or("collision body root absent")?,true).map_err(|e|format!("clock: {e:?}"))
 }
 fn target_entity_exists(&mut self,t:TargetableId)->Result<bool,String>{
  if !t.is_valid(){return Ok(false);} match t.kind{
   0=>Ok(self.actor(t)?.is_some()),1=>Ok(t.id as u32==self.entity_id||self.pedestrian(t).is_some()),
   2=>Ok(self.dmos.object(t.id as u32).is_some()),3=>Ok(self.vehicles.get(t.id as u32).is_some()),
   _=>Err(format!("unknown targetable kind {}",t.kind)),}
 }
 fn hand_prop_drop_probability(&mut self,e:&Entity)->Result<f32,String>{Ok(e.hand_prop_usage.drop_on_collision)}
 fn throw_hand_prop_away(&mut self,e:&mut Entity)->Result<(),String>{e.throw_hand_prop_away(self.cvars)}
 fn update_player_collision_stats(&mut self,e:&Entity)->Result<(),String>{
  // LWBasicEntity.GetType82e4bfd0 returns the full entity key, NOT its
  // census category. Raw insrdi/cmpld recovers the cases Hex-Rays omits.
  let key=e.core.as_ref().ok_or("collision statistics core absent")?.entity_key;
  let item=match key {
   0x5ac9_5e9c_35f6_5bed=>52,
   0x8151_8f0e_dafe_70a6=>53,
   0x3457_c13f_e6d1_28f1=>54,
   0xe6a5_ffe6_8d7d_12b9=>55,
   0x9a4d_5e87_ade3_af28=>56,
   0xe2a8_1660_722e_d157=>57,
   0xa7e8_1ade_65ca_24e7=>58,
   0xf977_b957_a0e8_68cb=>59,
   0x5ef2_eddd_c75f_25de=>60,
   0xfe1f_4594_b6f9_18e2=>61,
   0x58b6_0bae_7322_56bb=>62,
   0x74ed_302d_8480_8459|0x8894_6253_76ee_1158|0xef73_f403_30bd_5d77=>63,
   _=>51,
  };
  self.world.local_skater_identity.as_ref().ok_or("collision statistics backend absent")?
   .backend().update_additive_statistic(item,1.,false,self.cvars)
 }
 fn broadcast_mood(&mut self,e:CollisionMoodEvent)->Result<(),String>{self.game_sim_events.send(LivingWorldMessage::Mood(MoodEvent{
  event_type:e.event_type,position:e.position.map(f32::to_bits),perpetrator:e.perpetrator,victim:e.victim,
  loudness_distance_factor:e.magnitude.to_bits(),visibility_distance_factor:e.exposure.to_bits(),}))}
 fn guard_knocked_down(&mut self,id:u32)->Result<(),String>{
  self.world.local_skater_identity.as_ref().ok_or("guard knockdown SkaterManager absent")?
   .try_challenge_broadcaster().map_err(|e|format!("guard knockdown broadcaster: {e:?}"))?
   .send_guard_knocked_down(id).map_err(|e|format!("guard knockdown enqueue: {e:?}"))
 }
 fn career_mode(&mut self)->Result<bool,String>{
  //IsCareerModeGame826f55c8 compares CommonBackEnd gameMode to1.
  Ok(self.world.local_skater_identity.as_ref().ok_or("collision SkaterManager absent")?.backend().try_game_mode().map_err(|e|e.to_string())?==1)
 }
 fn chasee_actor_is_local(&mut self,e:&Entity)->Result<Option<bool>,String>{
  let target=e.state.as_ref().ok_or("collision AI state absent")?.chaser.chasee;
  let Some(actor)=self.actor(target)? else{return Ok(None);};
  let local=self.world.local_skater_identity.as_ref().ok_or("collision SkaterManager absent")?
   .try_is_local_player(||actor.prefix().map_err(|e|e.to_string())?.fields().map(|f|f.player_index as i32).ok_or("collision Actor fields absent".to_string())).map_err(|e|e.to_string())?;
  Ok(Some(local))
 }
 fn grant_good_samaritan(&mut self)->Result<(),String>{
  // PedestrianColliding.Begin82716c58..60: AT_GoodSamaritan=36, delay0.
  // The caller retains the career/player-collision/guard/nonlocal-chasee gates.
  self.world.local_skater_identity.as_ref().ok_or("GoodSamaritan backend absent")?
   .backend().grant_achievement(36,0)
 }
}
