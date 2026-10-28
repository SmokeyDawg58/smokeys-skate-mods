//! ChaseMan.Update8270f678 over the retained Actor, statistics and LW owners.
use crate::{selected_living_world_recipes::SelectedRecipeFrame,
    reckoning_numeric::ReckoningVector,
    input_cvar_runtime::InputCvarReferences,physical_player_assembly::PhysicalAssemblyWorld,
    static_ground_pool::StaticGroundPool,actor_construction::ActorConstruction,
    ui::trick_backend::BackendCommand};
use std::sync::Arc;
fn position(actor:&ActorConstruction)->Result<[f32;3],String>{
    let physical=actor.cached_physical_allocation().map_err(|e|e.to_string())?;
    let output=physical.owner().phys_out().lock().map_err(|_|"chase PhysOut lock")?;
    let p=output.try_skeleton_output().map_err(|e|format!("chase position: {e:?}"))?
        .skeleton_root_matrix.columns()[3].operand();
    Ok([p[0],p[1],p[2]])
}
/// ChaseMan.Update8270f880/8270fa20 reads PhysOut+0x40, Filtered+0x00.
/// These constants belong to eFilteredPhysicsState, not raw ePhysicsState.
fn state(actor:&ActorConstruction)->Result<i32,String>{
    let physical=actor.cached_physical_allocation().map_err(|e|e.to_string())?;
    let output=physical.owner().phys_out().lock().map_err(|_|"chase PhysOut lock")?;
    Ok(output.try_filtered_output().map_err(|e|format!("chase filtered state: {e:?}"))?.filtered_physics_state as i32)
}
/// SetJackedRespawnPosition826f3c28. None preserves the prior override for a
/// missing/empty list; a nonempty list with no valid locations clears its flag.
fn jacked_respawn(world:&crate::challenge::system::World,activity:usize,point:[f32;3])
    ->Option<Option<[f32;16]>> {
    let key=world.activities.activities[activity].key;
    // Raw lis/ori/insrdi826f3cb8..cc: challenges' authored jack spawn list.
    let locations=world.db.attribute(crate::challenge::keys::class::CHALLENGES,key,0x015d_0811_2438_56b2)?;
    if locations.len()==0{return None;}
    let mut nearest:Option<([f32;16],f32)>=None;
    for i in 0..locations.len(){
        let Some(name)=locations.text(i) else{continue;};
        let hash=crate::challenge::keys::hash_value32(name);
        let Some(transform)=world.host.location_orientation(hash)
            .or_else(||world.locations.get(&hash).copied()) else{continue;};
        let delta=[transform[12]-point[0],transform[13]-point[1],transform[14]-point[2],0.];
        let distance=crate::reckoning_numeric::host_dot3(delta,delta);
        if nearest.is_none_or(|(_,best)|distance<best){nearest=Some((transform,distance));}
    }
    Some(nearest.map(|(transform,_)|transform))
}
impl SelectedRecipeFrame<'_>{
    pub(crate) fn update_chase_manager(&mut self,world:&mut PhysicalAssemblyWorld,
        cvars:&InputCvarReferences,dt:f32,ground:&StaticGroundPool,
        ai_respawn:&mut crate::actor_construction::breadcrumbs::ActorAiRespawnQuery<'_>)->Result<(),String>{
        let local=Arc::clone(world.local_skater_identity.as_ref().ok_or("chase SkaterManager absent")?);
        let index=local.try_local_index().map_err(|e|e.to_string())?;
        let Some(actor)=local.try_find_by_index(index).map_err(|e|e.to_string())? else{return Ok(());};
        let owner=Arc::clone(world.grind_services.living_world());
        let mut living=owner.lock().map_err(|_|"chase LW lock")?;
        let mut chase=living.chase.take().ok_or("ChaseMan absent")?;
        // Restore the SAME manager on every checked failure. Respawn executes
        // after restoring it: breadcrumb constraints query this LW owner.
        let result=(||->Result<Option<(Arc<ActorConstruction>,Option<[f32;16]>)>,String>{
            if chase.local_enabled&&!chase.are_local_chases_forced_to_terminate(){
                let p=position(&actor)?;
                if self.painter.lock().map_err(|_|"chase painter lock")?
                    .try_region_name(crate::session_marker::SECURITY_GUARD_AREAS,[p[0],p[2]])
                    .map_err(|e|e.to_string())?.is_some(){
                    local.backend().update_additive_statistic(5,dt,false,cvars)?;
                }
            }
            // The stats call precedes both chase duration writes.
            chase.prevent_local_chase_timer=if chase.prevent_local_chase_timer<=0.{0.}
                else{chase.prevent_local_chase_timer-dt};
            if chase.flags&0x20!=0&&chase.prevent_local_chase_timer<=0.{chase.flags&=0xdb;}
            if chase.chase_is_on()&&!chase.is_skater_being_jacked()&&chase.local_enabled
                &&!chase.are_local_chases_forced_to_terminate(){
                let offboard=state(&actor)?==6;
                local.backend().update_additive_statistic(4,dt,false,cvars)?;
                chase.current_chase_timer+=dt;
                chase.current_offboard_chase_timer+=if offboard{dt}else{0.};
            }
            // Native caches the original end pointer. Erase shifts live slots
            // left while the iterator advances; the old tail remains readable.
            let mut slots=chase.warners.clone();
            let mut live_len=slots.len();
            for index in 0..slots.len(){
                let id=slots[index];
                if id.kind!=1{continue;}
                let Some(guard)=living.pedestrian_entities.as_ref().and_then(|m|m.get(id.id as u32))
                    .and_then(|e|e.security_guard.as_ref()) else{continue;};
                let p=position(&actor)?;
                let painter=self.painter.lock().map_err(|_|"chase painter lock")?;
                if !guard.zone.contains_boundary_or_buffer(&painter,p).map_err(|e|e.to_string())?{
                    chase.unregister_warner(id);
                    if let Some(at)=slots[..live_len].iter().position(|v|*v==id){
                        slots.copy_within(at+1..live_len,at);live_len-=1;
                    }
                }
            }
            if !chase.is_skater_being_jacked(){
                let stored=actor.prefix().map_err(|e|e.to_string())?.chasee.as_ref()
                    .ok_or("Actor Chasee absent")?.group_end_reason;
                let captured=chase.group_end_reason(stored,true)==1;
                if !captured{return Ok(None);}
                let guard=chase.chasers.iter().filter(|id|id.kind==1).find_map(|id|
                    living.pedestrian_entities.as_ref().and_then(|m|m.get(id.id as u32))
                        .filter(|e|e.security_guard.is_some()));
                let Some(guard)=guard else{return Ok(None);};
                let activity=guard.challenge_owner;
                chase.flags|=0x80;
                chase.jacked_timer=cvars.try_jacked_timer().map_err(|e|format!("Jacked timer: {e:?}"))?;
                chase.jacked_actor=Some(actor.prefix().map_err(|e|e.to_string())?.fields()
                    .ok_or("Actor fields absent")?.player_index as i32);
                actor.try_prevent_manual_respawn().map_err(|e|e.to_string())?;
                // JackSkater8270fd08: the overlay and service capping are career
                // free-roam only. Authored respawn selection applies in every mode.
                let overlay=local.backend().try_game_mode().map_err(|e|e.to_string())?==1
                    && self.challenge.as_deref().is_none_or(|w|w.activities.main.is_none());
                if overlay {chase.flags|=0x40;} else {chase.flags&=!0x40;}
                local.backend().send_tricks_command(BackendCommand::SetIsJacked(overlay))?;
                if let Some(challenge)=self.challenge.as_deref_mut(){
                    if let Some(activity)=activity {
                        if overlay {
                            crate::challenge::dynamic_hull::set_capping_on_jacked(challenge,activity,position(&actor)?);
                        }
                    }
                }
                chase.warners.clear();
                if let Some(challenge)=self.challenge.as_deref(){
                    if let Some(activity)=activity {
                        if let Some(spawn)=jacked_respawn(challenge,activity,position(&actor)?) {
                            if let Some(transform)=spawn {
                                chase.closest_jacked_spawn=Some(transform);chase.flags|=2;
                            }else{chase.flags&=!2;}
                        }
                    }
                }
                if std::env::var_os("SKATE2_TRACE_SECURITY_CAPTURE").is_some(){
                    eprintln!("security capture: begin player={index} owner={activity:?} overlay={overlay} timer={} spawn={:?}",
                        chase.jacked_timer,if chase.flags&2!=0{chase.closest_jacked_spawn}else{None});
                }
                return Ok(None);
            }
            chase.jacked_timer-=dt;
            let Some(captured)=local.try_find_by_index(chase.jacked_actor.unwrap_or(-1))
                .map_err(|e|e.to_string())? else{return Ok(None);};
            let filtered=state(&captured)?;
            if filtered!=4{
                if std::env::var_os("SKATE2_TRACE_SECURITY_CAPTURE").is_some(){
                    eprintln!("security capture: cancel player={:?} filtered={filtered} timer={}",chase.jacked_actor,chase.jacked_timer);
                }
                chase.flags&=!0x80;return Ok(None);
            }
            if !(chase.jacked_timer<=0.){return Ok(None);}
            if Arc::ptr_eq(&actor,&captured){
                local.backend().send_tricks_command(if chase.flags&0x40!=0{
                    BackendCommand::SetCanShowJackedOverlay(true)
                }else{BackendCommand::SetIsJacked(false)})?;
                if let Some(challenge)=self.challenge.as_deref_mut(){
                    if let Some(main)=challenge.activities.main {
                        // Photo=6, Pro=28 (8270fad4/e8), including their empty
                        // StartLocation default; TeleportManager owns resolution.
                        if matches!(crate::challenge::signup::challenge_type(challenge,main),6|28){
                            let key=challenge.activities.activities[main].key;
                            let location=challenge.db.attribute(crate::challenge::keys::class::CHALLENGES,
                                key,0xe9b6_99e3_1f7c_45ca).and_then(|a|a.text(0)).unwrap_or("").to_owned();
                            if std::env::var_os("SKATE2_TRACE_SECURITY_CAPTURE").is_some(){
                                eprintln!("security capture: challenge respawn player={index} location={location}");
                            }
                            crate::challenge::teleport::teleport_player(challenge,index,&location,index,false,0.,false);
                            chase.jacked_actor=None;chase.flags&=0x7d;
                            return Ok(None);
                        }
                    }
                }
            }
            Ok(Some((captured,if chase.flags&2!=0{chase.closest_jacked_spawn}else{None})))
        })();
        living.chase=Some(chase);
        drop(living);
        let Some((captured,override_transform))=result? else{return Ok(());};
        if std::env::var_os("SKATE2_TRACE_SECURITY_CAPTURE").is_some(){
            eprintln!("security capture: breadcrumb respawn override={override_transform:?}");
        }
        let breadcrumbs=captured.retained_breadcrumbs().map_err(|e|format!("chase breadcrumbs: {e:?}"))?
            .ok_or("chase BreadcrumbManager absent")?;
        let mut numeric=crate::collision_line_numeric::HostCollisionLineNumeric;
        let mut source={
            let mut crumbs=breadcrumbs.lock().map_err(|_|"chase breadcrumbs lock")?;
            if let Some(transform)=override_transform{
                // SetRespawnPosition82ce17f8.
                crumbs.override_active=true;
                crumbs.override_respawn=crate::actor_construction::breadcrumbs::SkaterBreadCrumb{
                    transform:crate::skeleton_root_transforms::SkeletonRootMatrix::known(transform),
                    foot_forward_right:true,offboard:true};
            }
            crate::actor_construction::breadcrumbs::LiveBreadCrumbSelectionSource::new(&crumbs,&captured,
                self.pedestrian_animation.world,world,ground,cvars,&mut numeric)
        };
        captured.try_teleport_somewhere_appropriate(&breadcrumbs,&mut source,ai_respawn)
            .map_err(|e|format!("chase respawn: {e:?}"))?;
        let mut living=owner.lock().map_err(|_|"chase LW lock")?;
        let chase=living.chase.as_mut().ok_or("ChaseMan absent after respawn")?;
        chase.jacked_actor=None;chase.flags&=0x7d;
        Ok(())
    }
}
