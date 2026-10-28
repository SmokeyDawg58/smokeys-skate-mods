//! Selected non-AI Actor.Update after `SetUpPhysics`, including audio timers.
use super::{ActorConstruction, ActorConstructionError};

#[derive(Debug)]
pub(crate) enum SelectedActorUpdateError {
    Actor(ActorConstructionError),
    Missing(&'static str),
    NonemptyChaseePending,
    Chasee(String),
}
impl std::fmt::Display for SelectedActorUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => error.fmt(f),
            Self::Missing(field) => write!(f, "selected Actor.Update missing {field}"),
            Self::NonemptyChaseePending => {
                f.write_str("Actor.Update nonempty Chasee callback pending")
            }
            Self::Chasee(error)=>f.write_str(error),
        }
    }
}
impl ActorConstruction {
    /// References: `Actor.Update825f10f0..1140`, `Chasee.UpdateByChasee82de5200`.
    /// The manager calls this after `SetUpPhysics` has copied the one-shot flags,
    /// and after the session's `AIController::Update` for an AI Actor.
    pub(crate) fn try_update_after_physics_setup(&self,dt:f32) -> Result<(), SelectedActorUpdateError> {
        self.try_update_after_physics_setup_with_chasee(dt,|chasee,_,_|{
            if chasee.chasers.is_empty(){Ok(())}else{Err(SelectedActorUpdateError::NonemptyChaseePending)}
        })
    }
    pub(crate) fn try_update_after_physics_setup_with_chasee(&self,dt:f32,
        update:impl FnOnce(&mut crate::living_world_chase::ChaseeState,
            crate::living_world_chase::TargetableId,crate::living_world_chase::TargetableId)->Result<(),SelectedActorUpdateError>
    )->Result<(),SelectedActorUpdateError>{
        use SelectedActorUpdateError as E;
        let mut state = self.prefix().map_err(E::Actor)?;
        state
            .physics_auxiliary
            .as_ref()
            .ok_or(E::Missing("physics auxiliaries"))?;
        // Native +258 at825f1138: clear impulse/reset/AI-present only. The
        // already published physics input is a distinct destination.
        let flags = state
            .input_generation_flags
            .as_mut()
            .ok_or(E::Missing("input flags"))?;
        *flags &= 0x97ff_ffff;
        let fields=state.fields().ok_or(E::Missing("Actor fields"))?;
        let id=crate::living_world_chase::TargetableId{kind:0,id:fields.player_index as i32};
        let protector=fields.protector;
        update(state.chasee.as_mut().ok_or(E::Missing("Chasee constructor"))?,id,protector)?;
        // Actor.Update825f1144..118c. Exact zero after subtraction retains
        // sound level until the next frame; only a strict crossing clears it.
        let fields=state.fields_mut().ok_or(E::Missing("Actor fields"))?;
        if !(fields.sound_timer<=0.) {
            fields.sound_timer-=dt;
            if !(fields.sound_timer>=0.) {
                fields.sound_timer=0.;
                fields.sound_level=0.;
            }
        } else {fields.sound_level=0.;}
        if !(fields.grind_sound_timer<=0.) {
            fields.grind_sound_timer-=dt;
            if !(fields.grind_sound_timer>=0.) {fields.grind_sound_timer=0.;}
        }
        Ok(())
    }
}
