//! Actual PhysicsAir.Enter82d98dd0 on the retained pair after state publication.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GRIND, PHYSICS_STATE_GROUND_ANIMATION, PHYSICS_STATE_KNOWN_AIR,
    PHYSICS_STATE_NONSPECIFIC, PHYSICS_STATE_PHYSICS_GROUND,
};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError, SimulationSkeletonJumpPairError,
        board_entry::air_drive::AirDriveError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_air_state::PhysicsAirState,
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_footplant::{FootPlantError, RetainedTrajectory},
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::{
        air_jump_velocity::{self, AirJumpVelocityError},
        physical_skeleton_controller::{
            PhysicalSkeletonControllerError, SkeletonCollisionTransition,
        },
    },
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum AirEntryError<N> {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    Poisoned,
    Missing(&'static str),
    WrongState,
    ForeignBindings,
    Collision(PhysicalSkeletonControllerError),
    MissingIk,
    Drive(AirDriveError<N>),
    FootPlant(FootPlantError),
    Transform(crate::dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError),
    Cvar(InputCvarError),
    JumpVelocity(AirJumpVelocityError<N>),
    Attribute(AttributeRuntimeError),
    Numeric(N),
    Overlay(DebugOverlayError),
    MissingDebugOverlay,
}
impl<N: std::fmt::Debug> std::fmt::Display for AirEntryError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Air entry requires skater pool"),
            Self::Pair(e) => write!(f, "Air entry pair: {e}"),
            Self::Pool(e) => write!(f, "Air entry pool: {e}"),
            Self::Poisoned => f.write_str("Air entry owner lock poisoned"),
            Self::Missing(field) => write!(f, "Air entry missing {field}"),
            Self::WrongState => f.write_str("Air entry requires selected Air"),
            Self::ForeignBindings => f.write_str("Air entry has different pair bindings"),
            Self::Collision(e) => write!(f, "Air entry collision: {e}"),
            Self::MissingIk => f.write_str("Air entry IK owner unavailable"),
            Self::Drive(e) => write!(f, "Air entry drive: {e}"),
            Self::FootPlant(e) => write!(f, "Air entry FootPlant: {e:?}"),
            Self::Transform(e) => write!(f, "Air entry transform: {e}"),
            Self::Cvar(e) => write!(f, "Air entry cvar: {e:?}"),
            Self::JumpVelocity(e) => write!(f, "Air entry CalcVelFromJump: {e}"),
            Self::Attribute(e) => write!(f, "Air entry mount attribute: {e}"),
            Self::Numeric(e) => write!(f, "Air entry mount numeric: {e:?}"),
            Self::Overlay(e) => write!(f, "Air entry debug overlay: {e:?}"),
            Self::MissingDebugOverlay => f.write_str("Air entry debug overlay unavailable"),
        }
    }
}
impl PhysicalAssemblyWorld {
    /// The ordinary Ground100 predecessor takes the complete no-COM arm.
    /// Other predecessors preserve source writes until their missing producer;
    /// no generic trajectory or fabricated mounted input is substituted.
    #[expect(
        clippy::too_many_lines,
        reason = "single source-ordered Air entry with retained failure prefixes"
    )]
    pub(crate) fn try_enter_pair_air<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), AirEntryError<N::Error>> {
        use AirEntryError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.current != CurrentPhysicalState::Air {
            return Err(E::WrongState);
        }
        let air = state.air.as_mut().ok_or(E::Missing("Air owner"))?;
        if !air
            .bindings
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, &owner))
            || !air.bindings.board.same_as(board)
            || !air.bindings.skeleton.same_as(skeleton)
        {
            return Err(E::ForeignBindings);
        }
        self.try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::InitAir,
        )
        .map_err(E::Collision)?;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.try_enable_air_entry_ik()
            })
            .map_err(E::Pair)?
            .map_err(|_| E::MissingIk)?;
        self.try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::Set(6),
        )
        .map_err(E::Collision)?;
        self.try_enable_board_angular_drive_only(board, 1.0, refs, numeric)
            .map_err(E::Drive)?;
        owner
            .footplant()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_full_reset(&owner, skeleton)
            .map_err(E::FootPlant)?;
        air.fields.reached_apex = false;
        air.fields.trajectory_timer = 0;
        air.fields.time_in_state = 0.0;
        air.fields.landing_normal =
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
        let transform = self
            .board(board)
            .map_err(E::Pool)?
            .try_live_part_transform_host(&self.backend, 6)
            .map_err(E::Transform)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let processed = processed.lock().map_err(|_| E::Poisoned)?;
        air.fields.max_y = transform[13];
        air.fields.start_y = processed
            .prior_output()
            .ground_position_all_states()
            .ok_or(E::Missing("Processed GroundPosAllStates"))?
            .xyz[1];
        let fields = processed.physics_state_fields();
        let last_category = fields
            .last_category
            .ok_or(E::Missing("Processed last category"))?;
        let last = fields.last.ok_or(E::Missing("Processed last state"))?;
        if fields.flags_8d4.1 & 0x0400_0000 == 0 {
            return Err(E::Missing("Processed run mount"));
        }
        air.fields.run_mount = Some(fields.flags_8d4.0 & 0x0400_0000 != 0);
        if fields.flags_8d4.1 & 0x0200_0000 == 0 {
            return Err(E::Missing("Processed sprint mount"));
        }
        air.fields.sprint_mount = Some(fields.flags_8d4.0 & 0x0200_0000 != 0);
        let update_heading;
        if last_category == PHYSICS_CATEGORY_GRIND || last == PHYSICS_STATE_NONSPECIFIC {
            update_heading = false;
            let flags = processed.known_flags_8c4();
            if flags.1 & 0x4000 == 0 {
                return Err(E::Missing("Processed COM velocity selection"));
            }
            air.fields.use_com_velocity = flags.0 & 0x4000 == 0;
        } else {
            update_heading = ![PHYSICS_STATE_PHYSICS_GROUND, PHYSICS_STATE_GROUND_ANIMATION, PHYSICS_STATE_KNOWN_AIR].contains(&last);
            air.fields.use_com_velocity = update_heading;
        }
        // 82d98ef8..f04: the fix-frames count and the full Processed+260 COM
        // velocity load both precede the live limit compare.
        let com = if air.fields.use_com_velocity {
            let frames = fields
                .frames_since_jump_correct
                .ok_or(E::Missing("Processed frames since jump correction"))?;
            let velocity = processed
                .prior_output()
                .centre_of_mass_velocity()
                .ok_or(E::Missing("Processed COM velocity"))?
                .value;
            Some((frames, velocity))
        } else {
            None
        };
        // CalcVelFromJump and InitMountTrajectory reacquire this Processed owner.
        drop(processed);
        if let Some((frames, com_velocity)) = com {
            let limit = owner
                .inputs()
                .cvars
                .try_physics_air_jump_fix_frames()
                .map_err(E::Cvar)?;
            // 82d98f10 cmpw/bge: signed; only fewer frames call CalcVelFromJump.
            let velocity = if frames < limit {
                air_jump_velocity::try_calculate(&owner, frames, &self.debug_overlay, numeric)
                    .map_err(E::JumpVelocity)?
            } else {
                com_velocity
            };
            let processed = Arc::clone(
                owner
                    .reckoning()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .processed(),
            );
            let processed = processed.lock().map_err(|_| E::Poisoned)?;
            if air.fields.run_mount == Some(true) || air.fields.sprint_mount == Some(true) {
                // 82d98f94: the mount branch discards the velocity above.
                init_mount_trajectory(
                    air,
                    &processed,
                    &owner.inputs().cvars,
                    &self.debug_overlay,
                    numeric,
                )?;
            } else {
                // 82d98f40..f8c: Processed+250 position, the velocity above,
                // VECTOR3_GRAVITY and splatted -1 time, in store order.
                let position = processed
                    .prior_output()
                    .centre_of_mass()
                    .ok_or(E::Missing("Processed COM position"))?
                    .value;
                air.fields.com_trajectory = RetainedTrajectory {
                    position,
                    velocity,
                    gravity: VECTOR3_GRAVITY,
                    time: [-1.0; 4],
                };
            }
        }
        if update_heading {
            // 82d98fa8..fb0: Skeleton byte+3ded = 1.
            self.skaters
                .as_mut()
                .ok_or(E::MissingPool)?
                .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                    normal.root_transforms_mut().request_air_entry_heading();
                })
                .map_err(E::Pair)?;
        }
        if refs.try_draw_physics_air_debug_info().map_err(E::Cvar)? {
            return Err(E::MissingDebugOverlay);
        }
        Ok(())
    }
}

/// `Math::VECTOR3_GRAVITY` (initializer 82f76360): (0, -9.8, 0, 0).
const VECTOR3_GRAVITY: SkeletonRootVector =
    SkeletonRootVector::known([0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0]);
/// `physics_airstates` keys read by InitMountTrajectory (82d8abec/ac38 run,
/// 82d8accc/ad18 sprint): the first scales Processed+e0, the second +d0.
const RUN_MOUNT_Z_SCALE: u64 = 0xbac4_3a7a_1c42_8aaf;
const RUN_MOUNT_Y_SCALE: u64 = 0xf92b_fcf1_f71c_9361;
const SPRINT_MOUNT_Z_SCALE: u64 = 0x136f_a41a_2878_e71e;
const SPRINT_MOUNT_Y_SCALE: u64 = 0x0e3c_6cbc_e149_4428;

/// `PhysicsAir::InitMountTrajectory`82d8abb0. The run (else sprint) launch
/// velocity is Processed.mEffectiveAnimTransform Z axis times one attribute
/// plus its Y axis times another; it is kept only while its distance from
/// the Processed+260 COM velocity is not ordered-greater than
/// `kPhysicsAirMountLaunchTrajMaxChange`.
fn init_mount_trajectory<N: ReckoningNumeric>(
    air: &mut PhysicsAirState,
    processed: &SkeletonProcessedContact,
    cvars: &InputCvarReferences,
    overlay: &DebugOverlaySlot,
    numeric: &mut N,
) -> Result<(), AirEntryError<N::Error>> {
    use AirEntryError as E;
    // abd0..abe4: Processed+260 is both the fallback and the initial result.
    let base = processed
        .prior_output()
        .centre_of_mass_velocity()
        .ok_or(E::Missing("Processed COM velocity"))?
        .value;
    let mut velocity = base;
    let keys = if air.fields.run_mount == Some(true) {
        Some((RUN_MOUNT_Z_SCALE, RUN_MOUNT_Y_SCALE))
    } else if air.fields.sprint_mount == Some(true) {
        Some((SPRINT_MOUNT_Z_SCALE, SPRINT_MOUNT_Y_SCALE))
    } else {
        None
    };
    if let Some((z_key, y_key)) = keys {
        let z_scale = air.try_scalar(z_key).map_err(E::Attribute)?;
        let effective = processed
            .ground_position_fields()
            .effective_animation
            .ok_or(E::Missing("Processed effective animation"))?;
        // vmulfp128: every lane of Processed+e0 times the splatted scalar.
        velocity = effective.columns()[2].map(|lane| lane * z_scale);
        let y_scale = air.try_scalar(y_key).map_err(E::Attribute)?;
        // vmaddfp128: Processed+d0 times the splat, fused onto the product.
        velocity = effective.columns()[1].zip(velocity, |axis, sum| axis.mul_add(y_scale, sum));
    }
    let difference = velocity.zip(base, |value, com| value - com);
    let squared = numeric
        .dot_vector(difference, difference)
        .map_err(E::Numeric)?;
    let maximum = cvars
        .try_physics_air_mount_launch_traj_max_change()
        .map_err(E::Cvar)?;
    let estimate = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(E::Numeric)?;
    // adcc..ae00: two VNMSUB/VMADD refinements, times the square, zero when
    // the square is zero; the same sequence as CalcVelFromJump's correction.
    let first = (estimate * 0.5).mul_add(-(squared.mul_add(estimate * estimate, -1.0)), estimate);
    let second = (first * 0.5).mul_add(-(squared.mul_add(first * first, -1.0)), first);
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * second
    };
    // ae0c fcmpu/ble: only an ordered greater-than restores the COM velocity.
    if length > maximum {
        velocity = base;
    }
    if cvars.try_draw_physics_air_debug_info().map_err(E::Cvar)? {
        overlay.require().map_err(E::Overlay)?;
    }
    // ae9c..aee8: same store order as the non-mount Enter branch.
    let position = processed
        .prior_output()
        .centre_of_mass()
        .ok_or(E::Missing("Processed COM position"))?
        .value;
    air.fields.com_trajectory = RetainedTrajectory {
        position,
        velocity,
        gravity: VECTOR3_GRAVITY,
        time: [-1.0; 4],
    };
    Ok(())
}
