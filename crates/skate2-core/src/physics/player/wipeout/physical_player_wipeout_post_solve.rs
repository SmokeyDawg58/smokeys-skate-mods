//! Selected WipeoutGround.UpdatePostPhysics82d8f700, after Skeleton.Post.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        wipeout::WipeoutSkeletonError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_wipeout_state::WipeoutGroundFields,
    reckoning_numeric::ReckoningNumeric,
    skeleton_physics_history::lane_knowledge::RecordVectorField,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{convert::Infallible, sync::Arc};
pub(crate) const REST_SPEED: u64 = 0x76ac_6578_b686_80ff;
pub(crate) const MINIMUM_TIME: u64 = 0x4ce6_a5ab_dbf8_9e16;
#[derive(Debug)]
pub(crate) enum WipeoutPostError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Global(WipeoutSkeletonError<Infallible>),
    Cvar(InputCvarError),
    Numeric(N),
}
impl<N: std::fmt::Debug> std::fmt::Display for WipeoutPostError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "Wipeout post-physics missing {field}"),
            Self::Poisoned => f.write_str("Wipeout post-physics poisoned owner"),
            Self::ForeignState => f.write_str("Wipeout post-physics foreign state"),
            Self::Pair(error) => write!(f, "Wipeout post-physics pair: {error}"),
            Self::Global(error) => write!(f, "Wipeout post-physics attributes: {error}"),
            Self::Cvar(error) => write!(f, "Wipeout post-physics cvar: {error:?}"),
            Self::Numeric(error) => write!(f, "Wipeout post-physics numeric: {error:?}"),
        }
    }
}
pub(crate) trait WipeoutPostSource {
    type Error;
    fn slow_speed(&mut self) -> Result<f32, Self::Error>;
    fn global(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
    fn squared_speed(&mut self, value: SkeletonRootVector) -> Result<f32, Self::Error>;
}
pub(crate) fn try_update_wipeout_post<S: WipeoutPostSource>(
    fields: &mut WipeoutGroundFields,
    source: &mut S,
) -> Result<(), S::Error> {
    let slow = source.slow_speed()?;
    let rest = source.global(REST_SPEED)?;
    let minimum = source.global(MINIMUM_TIME)?;
    let hips = source.velocity(23)?;
    let hips_squared = source.squared_speed(hips)?;
    let rest_squared = rest * rest;
    let hips_at_rest = hips_squared < rest_squared;
    let neck = source.velocity(1)?;
    let neck_squared = source.squared_speed(neck)?;
    let neck_at_rest = rest_squared > neck_squared;
    let slow_squared = slow * slow;
    let hips_slow = hips_squared < slow_squared;
    let enough_time = fields.time_in_state > minimum;
    fields.over = hips_at_rest && neck_at_rest && enough_time;
    fields.slow = hips_slow && enough_time;
    Ok(())
}
struct Source<'a, N> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    refs: &'a InputCvarReferences,
    numeric: &'a mut N,
}
impl<N: ReckoningNumeric> WipeoutPostSource for Source<'_, N> {
    type Error = WipeoutPostError<N::Error>;
    fn slow_speed(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_wipeout_slow_speed()
            .map_err(WipeoutPostError::Cvar)
    }
    fn global(&mut self, key: u64) -> Result<f32, Self::Error> {
        use WipeoutPostError as E;
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_pair_global_wipeout_float(&self.world.backend, self.board, self.skeleton, key)
            .map_err(E::Global)
    }
    fn velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        use WipeoutPostError as E;
        // GetPartVelocity82b46078 reads retained Skeleton+26d0+part*16.
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(
                &self.world.backend,
                self.board,
                self.skeleton,
                |normal, _| {
                    normal
                        .physics_history()
                        .record()
                        .vector_typed(RecordVectorField::Velocity(part))
                },
            )
            .map_err(E::Pair)
    }
    fn squared_speed(&mut self, value: SkeletonRootVector) -> Result<f32, Self::Error> {
        self.numeric
            .dot_vector(value, value)
            .map_err(WipeoutPostError::Numeric)
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_wipeout_postphysics<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
    ) -> Result<(), WipeoutPostError<N::Error>> {
        use WipeoutPostError as E;
        let actual = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, p| Arc::clone(p))
            .map_err(E::Pair)?;
        if !Arc::ptr_eq(&actual, owner) {
            return Err(E::ForeignState);
        }
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.current != CurrentPhysicalState::WipeoutGround {
            return Err(E::ForeignState);
        }
        let wipeout = state
            .wipeout_ground
            .as_mut()
            .ok_or(E::Missing("WipeoutGround"))?;
        if !wipeout.bindings.board.same_as(board)
            || !wipeout.bindings.skeleton.same_as(skeleton)
            || !wipeout
                .bindings
                .owner
                .upgrade()
                .is_some_and(|p| Arc::ptr_eq(&p, owner))
        {
            return Err(E::ForeignState);
        }
        try_update_wipeout_post(
            &mut wipeout.fields,
            &mut Source {
                world: self,
                board,
                skeleton,
                refs: &owner.inputs().cvars,
                numeric,
            },
        )
    }
}
