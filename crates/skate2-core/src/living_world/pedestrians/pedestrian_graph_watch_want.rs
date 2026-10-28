//! WatchWantTarget826ff678/8270b768/826ff738 and StopAndFace826ff7f0/8270ba10/826ff8b0.
use super::{LocalBehaviourBindings,LocalBehaviour,Phase};
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    pedestrian_graph::backend::frame::FrameServices,
    pedestrian_adjust_owners::HeadTrackingMode,reckoning_numeric::{HostReckoningNumeric,try_signed_angle}};
impl LocalBehaviourBindings {
    pub(crate) fn dispatch_watch_want(&mut self,occurrence:u16,phase:Phase,
        entity:&mut PedestrianConstruction,frame:&mut FrameServices<'_>)->Result<(),String>{
        let entry=self.entries.get_mut(occurrence as usize).ok_or("watch occurrence absent")?;
        let Some(LocalBehaviour::WatchWant{want,saved_speed,stopped,always_stop})=&mut entry.local else{
            return Err("watch target instance absent".into());
        };
        let target=entity.state.as_ref().ok_or("watch target state absent")?.wants[*want].target;
        match phase {
            Phase::Begin=>{
                *saved_speed=entity.navigation_mut()?.speed();
                let exists=frame.target_exists(entity,target)?;
                entity.head_tracking.as_mut().ok_or("watch head tracking absent")?
                    .push_targetable(HeadTrackingMode::TargetableVisibilityPoint,target,||Ok(exists))?;
            }
            Phase::End=>{
                entity.navigation_mut()?.seek_speed=*saved_speed;
                entity.head_tracking.as_mut().ok_or("watch head tracking absent")?.pop();
                entity.navigation_intent.explicit_direction=None;
            }
            Phase::Update=>{
                let Some(centre)=frame.target_centre_of_mass(entity,target)? else{return Ok(());};
                let core=entity.core.as_ref().ok_or("watch core absent")?;
                let direction=[centre[0]-core.transform[3][0],0.,centre[2]-core.transform[3][2],0.];
                if *always_stop {
                    entity.navigation_intent.explicit_direction=Some(direction);
                    entity.navigation_mut()?.seek_speed=0.;
                    return Ok(());
                }
                // Both GetTargetableVelocity and AngleBetweenVectors run even
                // after the stop latch has become true (native ordering).
                let predict=frame.cvars.try_navigating_head_tracking_predict().map_err(|e|format!("watch tuning: {e:?}"))?;
                let velocity=frame.target_velocity(entity,target)?.ok_or("watch target velocity absent")?;
                let predicted=std::array::from_fn::<_,4,_>(|i|velocity[i].mul_add(predict,direction[i]));
                let facing=[core.transform[2][0],0.,core.transform[2][2],0.];
                let angle=try_signed_angle(facing,predicted,[0.,1.,0.,0.],&mut HostReckoningNumeric)
                    .map_err(|e|format!("watch target angle: {e:?}"))?;
                let threshold=frame.cvars.try_navigating_head_tracking_angle().map_err(|e|format!("watch tuning: {e:?}"))?;
                if *stopped || (angle>=threshold && angle<=f32::from_bits(0x40c9_0fdb)-threshold){
                    *stopped=true;
                    entity.navigation_mut()?.seek_speed=0.;
                    entity.navigation_intent.explicit_direction=Some(direction);
                }else{
                    entity.navigation_mut()?.seek_speed=*saved_speed;
                    entity.navigation_intent.explicit_direction=None;
                }
            }
        }
        Ok(())
    }
}
