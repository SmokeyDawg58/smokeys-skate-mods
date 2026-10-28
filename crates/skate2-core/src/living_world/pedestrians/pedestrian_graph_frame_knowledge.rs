//! UpdateEntityKnowledge82e9e620 adapter over the existing state kernel.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    living_world_pedestrian_state::KnowledgeObservation,static_ground_pool::StaticGroundPool};

impl FrameServices<'_> {
    pub(crate) fn update_entity_knowledge(&mut self,entity:&mut PedestrianConstruction,
        dt:f32,ground:&StaticGroundPool)->Result<(),String>{
        // Move the sole state out temporarily to borrow the entity's actual
        // perception/core. Restore it on every Result path, including failure.
        let mut state=entity.state.take().ok_or("knowledge AI state absent")?;
        let frame=std::cell::RefCell::new(self);
        let result=state.update_entity_knowledge(dt,
            |target|frame.borrow().target_exists(entity,target),
            |target,just_added|{
                let mut frame=frame.borrow_mut();
                let centre=frame.target_centre_of_mass(entity,target)?.ok_or("knowledge target centre absent")?;
                // PDB eMoodEventType_chasedpersondetectibilitytest=15; both
                // native factors are1. drawDebug has no detection side effects.
                let visible=frame.can_detect_mood_event(entity,15,centre,1.,1.,ground)?;
                if !visible&&!just_added {
                    // These unused lanes never overwrite retained knowledge.
                    return Ok(KnowledgeObservation{visible,position:[0.;3],facing:[0.;3],velocity:[0.;3]});
                }
                let position=frame.target_position(entity,target)?.ok_or("knowledge target position absent")?;
                let facing=frame.target_facing(entity,target)?.ok_or("knowledge target facing absent")?;
                let velocity=frame.target_velocity(entity,target)?.ok_or("knowledge target velocity absent")?;
                Ok(KnowledgeObservation{visible,position:[position[0],position[1],position[2]],
                    facing:[facing[0],facing[1],facing[2]],velocity:[velocity[0],velocity[1],velocity[2]]})
            });
        entity.state=Some(state);
        result
    }
}
