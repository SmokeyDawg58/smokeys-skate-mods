//! `Update1` after `Skeleton.ProcessData`,82e0371c..38e8. `GrindManager` follows.
use super::physical_prepare::PhysicalPlayerPrepareSessionError;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::prior_input::PriorInputReadError,
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::{fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum PostSkeletonInputError<N> {
    Missing(&'static str),
    Pair(SimulationSkeletonJumpPairError),
    Poisoned,
    Ground(PriorInputReadError),
    Prepare(PhysicalPlayerPrepareSessionError<N>),
}
impl<N: fmt::Debug> fmt::Display for PostSkeletonInputError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "post-Skeleton input missing {field}"),
            Self::Pair(error) => write!(f, "post-Skeleton input pair: {error:?}"),
            Self::Poisoned => f.write_str("post-Skeleton input owner lock poisoned"),
            Self::Ground(error) => write!(f, "post-Skeleton Ground read: {error:?}"),
            Self::Prepare(error) => error.fmt(f),
        }
    }
}
fn need<T, N>(value: Option<T>, field: &'static str) -> Result<T, PostSkeletonInputError<N>> {
    value.ok_or(PostSkeletonInputError::Missing(field))
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_process_input_after_skeleton<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<(), PostSkeletonInputError<N::Error>> {
        use PostSkeletonInputError as E;
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
            let mut processed = processed.lock().map_err(|_| E::Poisoned)?;
            try_post_skeleton_prefix::<N::Error>(&owner, &mut processed)?;
        }
        // This existing adapter samples the SAME prepared history and processed
        // owner. Release all prefix locks before its own actual owner captures.
        self.try_update_pair_prepare_velocity(board, skeleton, numeric)
            .map_err(E::Prepare)?;
        let mut processed = processed.lock().map_err(|_| E::Poisoned)?;
        let mut prefix = owner.prefix().lock().map_err(|_| E::Poisoned)?;
        let prefix = prefix.as_mut().ok_or(E::Missing("HiLOD prefix"))?;
        let previous = prefix.last_crouch_input; //38cc
        let current = need(processed.input_controls_mut().crouch_input, "crouch+9d4")?; //38d0
        processed.input_controls_mut().crouch_input_delta = Some(current - previous); //38d8
        // The source recaptures processed and reloads current after delta stores.
        let current = need(processed.input_controls_mut().crouch_input, "crouch+9d4")?; //38e4
        prefix.last_crouch_input = current; //38e8
        Ok(())
    }
}

fn try_post_skeleton_prefix<N>(
    owner: &PhysicalPlayerAssemblyOwner,
    processed: &mut SkeletonProcessedContact,
) -> Result<(), PostSkeletonInputError<N>> {
    use PostSkeletonInputError as E;
    let timer = owner
        .prefix()
        .lock()
        .map_err(|_| E::Poisoned)?
        .as_ref()
        .ok_or(E::Missing("HiLOD prefix"))?
        .skitch_vehicle_collision_avoid_timer;
    let avoid_collision = if timer > 0.0 {
        //3724 ordered GT
        true
    } else if need(processed.physics_state_fields().flags_8cc, "flags8cc")? & 0x0100_0000 != 0 {
        let output = owner.phys_out().lock().map_err(|_| E::Poisoned)?;
        let time = output
            .try_guarded_ground_output()
            .map_err(E::Ground)?
            .time_to_skitch;
        //374c=4098000c: NOT_LT, including unordered. No >= replacement.
        time.partial_cmp(&0.0) != Some(std::cmp::Ordering::Less)
    } else {
        false
    };
    let old = need(processed.physics_state_fields().flags_8cc, "flags8cc")?;
    processed.physics_state_fields_mut().flags_8cc =
        Some((old & !0x10) | if avoid_collision { 0x10 } else { 0 }); //3768
    let no_input = need(processed.known_flags_8c8(), "flags8c8")? & 0x0100_0000 != 0;
    let time = if no_input {
        let dt = need(
            processed.physics_state_fields().time_delta,
            "processed dt+93c",
        )?;
        let prefix = owner.prefix().lock().map_err(|_| E::Poisoned)?;
        let previous = prefix
            .as_ref()
            .ok_or(E::Missing("HiLOD prefix"))?
            .time_since_last_input;
        dt + previous
    } else {
        0.0
    };
    owner
        .prefix()
        .lock()
        .map_err(|_| E::Poisoned)?
        .as_mut()
        .ok_or(E::Missing("HiLOD prefix"))?
        .time_since_last_input = time; //3794
    processed.input_controls_mut().time_since_last_input = Some(time); //3798
    Ok(())
}
