//! Actual retained `PhysicsGround::Exit`82d84a80, including jump velocity.
//! Outer state selection/publication remains the caller's responsibility.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_pumping::PumpingError,
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::physical_board_forces::{
        PhysicalPlayerBoardForceError, board_wipeout::PhysicalBoardWipeoutError,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum GroundExitError<N> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Poisoned,
    WrongState,
    ForeignBindings,
    Missing(&'static str),
    Pumping(PumpingError),
    Board(PhysicalPlayerBoardForceError),
    Material(PhysicalBoardWipeoutError),
    Numeric(N),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundExitError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Ground Exit requires skater pool"),
            Self::Pair(e) => write!(f, "Ground Exit pair: {e:?}"),
            Self::Poisoned => f.write_str("Ground Exit owner lock poisoned"),
            Self::WrongState => f.write_str("Ground Exit requires selected Ground"),
            Self::ForeignBindings => f.write_str("Ground Exit has different pair bindings"),
            Self::Missing(field) => write!(f, "Ground Exit missing {field}"),
            Self::Pumping(e) => write!(f, "Ground Exit Pumping: {e:?}"),
            Self::Board(e) => write!(f, "Ground Exit board: {e}"),
            Self::Material(e) => write!(f, "Ground Exit material: {e}"),
            Self::Numeric(e) => write!(f, "Ground Exit numeric: {e:?}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    /// Source82d84a80. Earlier cleanup survives a later missing input or
    /// numerical failure. This does not publish or enter the destination state.
    pub(crate) fn try_exit_pair_ground<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<(), GroundExitError<N::Error>> {
        use GroundExitError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.current != CurrentPhysicalState::Ground {
            return Err(E::WrongState);
        }
        let ground = state.ground.as_mut().ok_or(E::Missing("Ground owner"))?;
        if !ground
            .bindings
            .owner
            .upgrade()
            .is_some_and(|bound| Arc::ptr_eq(&bound, &owner))
            || !ground.bindings.board.same_as(board)
            || !ground.bindings.skeleton.same_as(skeleton)
        {
            return Err(E::ForeignBindings);
        }
        ground.fields.damped_turn = 0.0;
        ground.fields.push_scalar = 1.0;
        owner
            .pumping()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_reset()
            .map_err(E::Pumping)?;
        self.try_set_board_linear_drag(board, 0.0)
            .map_err(E::Board)?;
        self.try_restore_board_wheel_material(board)
            .map_err(E::Material)?;
        if ground.fields.do_jump {
            let processed = Arc::clone(
                owner
                    .reckoning()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .processed(),
            );
            let processed = processed.lock().map_err(|_| E::Poisoned)?;
            let gravity = processed
                .gravity()
                .ok_or(E::Missing("Processed gravity"))?
                .value;
            let timestep = processed
                .physics_state_fields()
                .time_delta
                .ok_or(E::Missing("Processed timestep"))?;
            let acceleration = [0.0, gravity, 0.0, 0.0];
            let jump = ground.fields.jump_direction;
            let candidate = SkeletonRootVector {
                xyz: std::array::from_fn(|lane| {
                    acceleration[lane].mul_add(timestep, jump.xyz[lane])
                }),
                w: jump.w.map(|w| acceleration[3].mul_add(timestep, w)),
            };
            // Toolkit_ClampJumpVelocity samples this SAME Processed+190, after
            // forming the candidate. Its output direction is the retained one.
            let velocity = processed
                .prior_output()
                .velocity()
                .ok_or(E::Missing("Processed velocity"))?
                .value;
            let result = clamp_jump_velocity(candidate, velocity, numeric).map_err(E::Numeric)?;
            drop(processed);
            self.try_set_board_velocity(board, &result)
                .map_err(E::Board)?;
        }
        Ok(())
    }
}

/// `Toolkit_ClampJumpVelocity82b23990`. The second vector load overwrites v63:
/// output is Processed velocity scaled using the candidate's magnitude.
pub(super) fn clamp_jump_velocity<N: ReckoningNumeric>(
    candidate: SkeletonRootVector,
    velocity: SkeletonRootVector,
    numeric: &mut N,
) -> Result<SkeletonRootVector, N::Error> {
    let candidate_squared = numeric.dot_vector(candidate, candidate)?;
    let stored = [velocity.xyz[0], velocity.xyz[1], velocity.xyz[2], 0.0];
    let stored_squared = numeric.dot3(stored, stored)?;
    let candidate_seed = numeric.reciprocal_sqrt_estimate(candidate_squared)?;
    let stored_seed = numeric.reciprocal_sqrt_estimate(stored_squared)?;
    let candidate_length = refined_length(candidate_squared, candidate_seed);
    let stored_length = refined_length(stored_squared, stored_seed);
    let denominator = if stored_length - 1.0 >= 0.0 {
        stored_length
    } else {
        1.0
    };
    // Raw23a5c=40990008 is BO4/BI25: skip unless ordered greater.
    // An unordered candidate therefore preserves scalar1, without division.
    let scalar = if denominator > candidate_length {
        candidate_length / denominator
    } else {
        1.0
    };
    Ok(SkeletonRootVector {
        xyz: velocity.xyz.map(|lane| lane * scalar),
        w: velocity.w.map(|lane| lane * scalar),
    })
}

fn refined_length(squared: f32, seed: f32) -> f32 {
    let square = seed * seed;
    let half = seed * 0.5;
    let residual = -squared.mul_add(square, -1.0);
    let once = half.mul_add(residual, seed);
    let square = once * once;
    let half = once * 0.5;
    let residual = -squared.mul_add(square, -1.0);
    let twice = half.mul_add(residual, once);
    let root = squared * twice;
    if squared == 0.0 { 0.0 } else { root }
}

/// Controlled final corpus operation after the board-wipeout material probe.
/// The actual retained Ground has `do_jump=false` from Enter; no jump is injected.
#[cfg(test)]
pub(crate) fn check_original(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) {
    let owner = world
        .skaters
        .as_mut()
        .unwrap()
        .try_with_jump_pair(&world.backend, board, skeleton, |_, owner| {
            Arc::clone(owner)
        })
        .unwrap();
    assert!(
        !owner
            .physics_state()
            .lock()
            .unwrap()
            .ground
            .as_ref()
            .unwrap()
            .fields
            .do_jump
    );
    let wheel = world
        .board_construction(board)
        .unwrap()
        .unwrap()
        .wheel_material_owner_for_test();
    let wipeout = world
        .board_construction(board)
        .unwrap()
        .unwrap()
        .wipeout_material_owner_for_test();
    let before = world.backend.rigid_body_snapshot();
    world
        .try_exit_pair_ground(
            board,
            skeleton,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    let state = owner.physics_state().lock().unwrap();
    assert_eq!(state.current, CurrentPhysicalState::Ground);
    let fields = &state.ground.as_ref().unwrap().fields;
    assert_eq!(fields.damped_turn.to_bits(), 0.0_f32.to_bits());
    assert_eq!(fields.push_scalar.to_bits(), 1.0_f32.to_bits());
    assert!(!fields.do_jump);
    drop(state);
    for part in 0..7 {
        let expected = if part < 4 { &wheel } else { &wipeout };
        assert!(std::ptr::eq(
            world
                .board(board)
                .unwrap()
                .current_part_material(part)
                .unwrap(),
            Arc::as_ref(expected)
        ));
    }
    let after = world.backend.rigid_body_snapshot();
    // Drag belongs to retained inertia; body velocities/poses/forces remain.
    assert_eq!(after, before);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    #[test]
    fn clamp_scales_retained_direction_and_w_instead_of_candidate() {
        let result = clamp_jump_velocity(
            SkeletonRootVector::known([3.0, 0.0, 0.0, 99.0]),
            SkeletonRootVector::known([0.0, 4.0, 0.0, 8.0]),
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(result, SkeletonRootVector::known([0.0, 3.0, 0.0, 6.0]));
        let result = clamp_jump_velocity(
            SkeletonRootVector::known([8.0, 0.0, 0.0, 0.0]),
            SkeletonRootVector::known([0.0, 4.0, 0.0, 8.0]),
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(result, SkeletonRootVector::known([0.0, 4.0, 0.0, 8.0]));
    }
    #[test]
    fn zero_and_unordered_follow_source_selection() {
        let result = clamp_jump_velocity(
            SkeletonRootVector::known([0.0; 4]),
            SkeletonRootVector::known([0.0; 4]),
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(result, SkeletonRootVector::known([0.0; 4]));
        let result = clamp_jump_velocity(
            SkeletonRootVector::known([f32::NAN, 0.0, 0.0, 0.0]),
            SkeletonRootVector {
                xyz: [1.0, 0.0, 0.0],
                w: None,
            },
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(
            result.xyz.map(f32::to_bits),
            [1.0_f32, 0.0, 0.0].map(f32::to_bits)
        );
        assert_eq!(result.w, None);
    }
}
