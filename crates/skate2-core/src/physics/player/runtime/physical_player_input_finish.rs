//! Final `Update1` environmental-grab publication, output lock and debug return.
use crate::{
    debug_overlay::DebugOverlayError,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_grab_spline::publication::GrabPublicationError,
    physical_player_phys_out::PhysOutAccessError,
};
use std::{fmt, sync::Arc};
#[derive(Debug)]
pub(crate) enum InputFinishError {
    Missing(&'static str),
    Poisoned,
    ForeignWorld,
    Pair(SimulationSkeletonJumpPairError),
    Grab(GrabPublicationError),
    Output(PhysOutAccessError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl fmt::Display for InputFinishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "input finish missing {v}"),
            Self::Poisoned => f.write_str("input finish owner lock poisoned"),
            Self::ForeignWorld => f.write_str("grab owner belongs to another world"),
            Self::Pair(e) => write!(f, "input finish pair: {e:?}"),
            Self::Grab(e) => e.fmt(f),
            Self::Output(e) => write!(f, "input finish output: {e:?}"),
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_finish_pair_process_input(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
    ) -> Result<(), InputFinishError> {
        use InputFinishError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        {
            let mut grab = owner.grab_spline().lock().map_err(|_| E::Poisoned)?;
            let grab = grab
                .as_mut()
                .ok_or(E::Missing("constructed grab spline owner"))?;
            if !grab.returned {
                return Err(E::Missing("grab constructor completion"));
            }
            if grab.world != owner.manager() {
                return Err(E::ForeignWorld);
            }
            let mut processed_guard = processed.lock().map_err(|_| E::Poisoned)?;
            grab.try_publish_processed(&mut processed_guard)
                .map_err(E::Grab)?;
        }
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::Poisoned)?
            .lock_after_update1()
            .map_err(E::Output)?;
        // Source3aec/f0 reads debug only AFTER3ae8 has locked actual PhysOut.
        if cvars.try_spew_jump_stuff().map_err(E::Cvar)? {
            // Source debug's XYZ load precedes overlay Get. The final callee825187c0
            // is literally empty; string/color arguments have no observable consumer.
            owner
                .tail()
                .lock()
                .map_err(|_| E::Poisoned)?
                .intended_jump_velocity
                .ok_or(E::Missing("debug intended jump velocity"))?;
            self.debug_overlay.require().map_err(E::Overlay)?;
        }
        Ok(())
    }
}
