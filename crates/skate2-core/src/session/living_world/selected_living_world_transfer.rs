//! Manager.EvaluateTransferableBehaviours82710cc0 / transfer82710a78.
use crate::{physical_player_assembly::PhysicalAssemblyWorld,
    input_cvar_runtime::InputCvarReferences,static_ground_pool::StaticGroundPool,
    native_skater_random::SimulationCommandRandom,
    reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric},
    behaviour_graph_runtime::TransferCondition};
use std::sync::Arc;

impl super::SelectedRecipeFrame<'_> {
    pub(crate) fn evaluate_transferable_behaviours(&mut self,world:&mut PhysicalAssemblyWorld,
        cvars:&InputCvarReferences,ground:&StaticGroundPool,mut random:Option<&mut SimulationCommandRandom>,
    )->Result<(),String>{
        let owner=Arc::clone(world.grind_services.living_world());
        let mut living=owner.lock().map_err(|_|"transfer Manager lock")?;
        let timer=&mut living.behaviour_evaluate_frame_timer;
        *timer=if *timer>10{0}else{*timer+1};
        if *timer!=0{return Ok(());}
        // Sole registration list, in insertion order. The transfer operation
        // only registers sinks; it cannot mutate this source list.
        let sources=self.graphs.plugin_sources.lock().map_err(|_|"transfer source list lock")?.clone();
        let actors=world.entity_roster.as_ref().ok_or("transfer Actor roster absent")?
            .snapshot().map_err(|e|format!("transfer Actor roster: {e:?}"))?;
        let triggers=self.triggers.lock().map_err(|_|"transfer trigger lock")?;
        let avoid=self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
        drop(triggers);
        let painter=self.painter.lock().map_err(|_|"transfer painter lock")?;
        for source in sources {
            let prop=living.props.as_deref().ok_or("transfer PropMan absent")?.entities.get(source)
                .ok_or("registered transfer source absent")?;
            // Native GetPosition precedes both admission gates.
            let _=prop.core.as_ref().ok_or("transfer source core absent")?.transform[3];
            if !prop.can_offer_behaviour(){continue;}
            let ids=living.pedestrian_entities.as_deref().ok_or("transfer pedestrian roster absent")?.ids();
            for id in ids {
                crate::pedestrian_graph::backend::frame::with_entity(&mut living,world,id,cvars,
                    &actors,&painter,self.vault,&avoid,|entity,frame|{
                    if entity.plugin.is_some(){return Ok(());}
                    let prop=frame.props.entities.get(source).ok_or("transfer source disappeared")?;
                    let template=Arc::clone(prop.transferable.as_ref().ok_or("transfer template absent")?);
                    if !entity.can_use_behaviour_source(&template)?{return Ok(());}
                    let tolerance=cvars.try_plugin_transfer_y_tolerance().map_err(|e|format!("transfer Y tuning: {e:?}"))?;
                    let position=entity.core.as_ref().ok_or("transfer sink core absent")?.transform[3];
                    let source_core=prop.core.as_ref().ok_or("transfer source core absent")?;
                    let at=source_core.transform[3];
                    if !(tolerance>(at[1]-position[1]).abs()){return Ok(());}
                    let radius=prop.transfer_radius.ok_or("transfer radius absent")?;
                    let delta=[at[0]-position[0],0.,at[2]-position[2],0.];
                    let distance=HostReckoningNumeric.dot3(delta,delta).map_err(|e|format!("transfer distance: {e:?}"))?;
                    // Preserve subtraction/comparison, including unordered NaN.
                    if distance-radius*radius>=0.{return Ok(());}
                    let category=source_core.entity_key;
                    let accepted=check_transfer_conditions(frame,entity,source,ground,random.as_deref_mut())?;
                    if accepted&&entity.decide_to_use_behaviour_source(category,&template,cvars)?{
                        frame.props.transfer_behaviour_to_pedestrian(source,entity)?;
                    }
                    Ok(())
                })?;
            }
        }
        Ok(())
    }
}

/// Source expression only, shared by normal transfer and prop-driven spawn.
/// The spawn path deliberately omits normal radius/retry/acceptance gates.
pub(super) fn check_transfer_conditions(
    frame:&mut crate::pedestrian_graph::backend::frame::FrameServices<'_>,
    entity:&crate::living_world_pedestrian_entity::PedestrianConstruction,source:u32,
    ground:&StaticGroundPool,mut random:Option<&mut SimulationCommandRandom>,
)->Result<bool,String>{
    use crate::living_world_grind_services::waypoints::plugin::transfer;
    let prop=frame.props.entities.get(source).ok_or("transfer source disappeared")?;
    let template=Arc::clone(prop.transferable.as_ref().ok_or("transfer template absent")?);
    let at=prop.core.as_ref().ok_or("transfer source core absent")?.transform[3];
    let position=entity.core.as_ref().ok_or("transfer sink core absent")?.transform[3];
    let cvars=frame.cvars;
    transfer::evaluate(&template.declaration.expression,&mut |condition|{
                        if let Some(value)=entity.transfer_entity_condition(condition)?{return Ok(value);}
                        if matches!(condition,TransferCondition::PluginTransferOdds{..}){
                            let random=random.as_deref_mut().ok_or("transfer odds require actual Simulation RNG")?;
                            return transfer::random_condition(condition,random)?.ok_or("transfer odds binding absent".into());
                        }
                        {
                            let source_prop=frame.props.entities.get(source).ok_or("transfer source disappeared")?;
                            if let Some(value)=transfer::waypoint_condition(condition,Some(source_prop),||Ok(position),
                                &mut HostReckoningNumeric,cvars)?{return Ok(value);}
                        }
                        match condition {
                            TransferCondition::CanRecognizePlayer{maximum_recognizers}=>{
                                let Some(count)=frame.props.recognizer_count()? else{return Ok(false);};
                                //827068dc computes capacity before invoking the
                                //pedestrian query, but still invokes it if full.
                                let capacity=count<*maximum_recognizers as usize;
                                Ok(entity.can_recognize_player()?&&capacity)
                            }
                            TransferCondition::EntityAllowedToJoinConversation|
                            TransferCondition::ConversationIsGatheringMembers=>{
                                //82706bb0/82706cc0 return false if source has no
                                //ILWConversationArea. Actual admitted Prop and
                                //WaypointGroup have no such interface. Native
                                //factory82e694f0 maps conversation+descendants
                                //to null callback82784530 (li r3,0; blr).
                                Ok(false)
                            }
                            TransferCondition::IsNearCrossWalk{distance}=>{
                                //82706acc reads actual LWEntityCoreData+160.
                                let xz=entity.core.as_ref().ok_or("crosswalk core absent")?.position_xz;
                                let Some(item)=frame.rain.intersection_data(xz).map_err(|e|format!("transfer intersection: {e:?}"))?
                                    else{return Ok(false);};
                                let intersection=frame.rain.intersections.get(&item.id).ok_or("transfer intersection disappeared")?;
                                let distance=match distance {
                                    crate::behaviour_graph_runtime::NativeOptionalF32::Written(bits)=>f32::from_bits(*bits),
                                    // Deliberate deterministic compatibility policy for
                                    // the native uninitialized field, NOT a recovered default.
                                    // Keep Unwritten in the parser; authored values stay exact.
                                    // See pedestrian-crosswalk-descriptor-audit-2026-09-30.md.
                                    crate::behaviour_graph_runtime::NativeOptionalF32::Unwritten=>0.0,
                                };
                                // All four authored corners, full XYZ distance.
                                for index in 0..4 {
                                    let corner=intersection.record.vector(64+index*16).map_err(|e|format!("transfer intersection corner: {e:?}"))?;
                                    let delta=std::array::from_fn(|i|position[i]-corner[i]);
                                    let square=HostReckoningNumeric.dot3(delta,delta).map_err(|e|format!("transfer crosswalk distance: {e:?}"))?;
                                    if square-distance*distance<=0.{return Ok(true);}
                                }
                                Ok(false)
                            }
                            TransferCondition::HasLineOfSightToProp=>{
                                //826fd288 -> direct World.LineCollisionTest827ab338,
                                //not perception cone/hearing or entity filtering.
                                let eye=[position[0]+0.,position[1]+1.8,position[2]+0.,position[3]+0.];
                                frame.world.try_pedestrian_visibility_line(ground,eye,at,
                                    "HasLineOfSightToProp",&mut crate::collision_line_numeric::HostCollisionLineNumeric)
                            }
                            TransferCondition::IsChasing=>{
                                let target=entity.state.as_ref().ok_or("transfer chase state absent")?.chaser.chasee;
                                frame.with_chasee(entity,target,|chasee|chasee.is_some())
                            }
                            _=>Err(format!("transfer condition requires concrete binding: {condition:?}")),
                        }
                    })
}
