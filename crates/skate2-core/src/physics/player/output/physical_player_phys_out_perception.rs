//! LWPedestrianMan.UpdateAudibles82e838b8 direct reads of actual PhysOut.
use crate::physics_state_ids::PHYSICS_STATE_SKITCHING;
use super::{PhysicalPlayerPhysOut,PhysOutComponent,PhysOutSystemVector};
pub(crate) struct PerceptionAudio {
    pub(crate) wheel_slide:f32,pub(crate) grinding:bool,pub(crate) wiping_out:bool,
    pub(crate) landed:[bool;4],pub(crate) previous_landed:[bool;4],pub(crate) impact:[f32;4],
}
pub(crate) struct MoodableFacts {
    pub(crate) audio: PerceptionAudio,
    pub(crate) just_finished_grind: bool,
    /// Finished initial trick is valid and belongs to FLIPS/AIR/METRIC/HANDPLANT.
    pub(crate) qualifying_air_finished: bool,
    pub(crate) skitching_vehicle: bool,
    pub(crate) hom_bail_in_progress: bool,
    pub(crate) hom_score: u32,
}
impl PhysicalPlayerPhysOut {
    /// UpdateMoodables82e83e40 consumes the same Audio/State/Scoring2
    /// publications as existing gameplay, without a parallel event history.
    pub(crate) fn try_pedestrian_moodable_facts(&self) -> Result<MoodableFacts, String> {
        let audio = self.try_pedestrian_audio()?;
        self.require_complete_component(PhysOutComponent::Scoring2)
            .map_err(|e| format!("moodable Scoring2: {e:?}"))?;
        let score = self.concrete.scoring.as_ref().expect("checked Scoring2");
        let settings = &score.air_finished_data.initial_trick.trick_scoring_settings;
        let qualifying_air_finished = score.air_finished && settings.is_valid
            && super::conditioners::scorable_id_category(settings.scorable_id)
                .is_some_and(|category| matches!(category, 3 | 4 | 5 | 6));
        let state = self.concrete.state.as_ref().expect("audio checked State");
        Ok(MoodableFacts {
            audio,
            just_finished_grind: score.just_finished_grind,
            qualifying_air_finished,
            skitching_vehicle: state.physics_state == PHYSICS_STATE_SKITCHING && state.grab_spline_desc.object_type == 1,
            hom_bail_in_progress: score.ho_m_bail_in_progress,
            hom_score: score.ho_m_bail_data.score,
        })
    }

    pub(crate) fn try_pedestrian_audio(&self)->Result<PerceptionAudio,String>{
        self.require_complete_component(PhysOutComponent::Audio).map_err(|e|format!("perception Audio: {e:?}"))?;
        self.require_complete_component(PhysOutComponent::State).map_err(|e|format!("perception State: {e:?}"))?;
        let audio=self.concrete.audio.as_ref().expect("checked Audio");
        let state=self.concrete.state.as_ref().expect("checked State");
        Ok(PerceptionAudio{wheel_slide:audio.wheel_slide,grinding:audio.is_grinding,
            wiping_out:state.is_wiping_out,landed:audio.wheel_landing.landed,
            previous_landed:audio.prev_wheel_landing.landed,impact:audio.wheel_landing.impact_strength})
    }
    pub(crate) fn try_pedestrian_spectate_event(&self)->Result<Option<crate::living_world_pedestrian_entity::perception::SpectateEvent>,String>{
        use crate::reckoning_numeric::ReckoningVector;
        self.require_complete_component(PhysOutComponent::Scoring2).map_err(|e|format!("perception Scoring2: {e:?}"))?;
        let score=self.concrete.scoring.as_ref().expect("checked Scoring2");
        let wiped_out=score.just_wiped_out||score.wipeout_interrupted_trick;
        if score.banked_point_change_this_frame<=0&&!wiped_out{return Ok(None);}
        let position=self.try_camera_system_vector(PhysOutSystemVector::CentreOfMass)
            .map_err(|e|format!("perception SystemReckoning: {e:?}"))?.operand();
        Ok(Some(crate::living_world_pedestrian_entity::perception::SpectateEvent{
            active:true,position,score:score.banked_point_change_this_frame,wiped_out,
        }))
    }
}
