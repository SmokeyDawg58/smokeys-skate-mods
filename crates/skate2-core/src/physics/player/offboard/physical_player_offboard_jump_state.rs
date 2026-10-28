//! PhysState_OffBoard_Jump (503) Enter82d9f3f8, CalculateTrajectory82d985a8,
//! Update82d98960, UpdatePostPhysics82d899f8 and FillPhysOut82d89a88. Exit is the
//! shared BLR825187c0. GetType82d98590 is folded with OffBoard_Air (li r3,0x1f6):
//! CheckForStateChange, Update5 and the next SetPhysicsState "last" all see 502.
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, wipeout::WipeoutSkeletonOperation,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{OffBoardJump, OwnerMember, StateKind, StatePayload},
    },
    physical_player_footplant::RetainedTrajectory,
    physical_player_phys_out::construct::offboard_jump_state::JumpOutputSource,
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    physical_player_trajectory_owners::offboard_runtime::{position_at, velocity_at},
    physics::PointGraph,
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric,
        length_from_squared_and_estimate, try_normalize_safe,
    },
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::physical_skeleton_controller::SkeletonCollisionTransition,
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

/// kUpdateTime at 82341f44.
const DT: f32 = f32::from_bits(0x3c88_8889);
/// Math::VECTOR3_GRAVITY, initializer82f76360.
const GRAVITY: V = V::known([0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0]);
/// Math::VECFLOAT_SMALL, initializer82f76340.
const SMALL: f32 = f32::from_bits(0x3780_0000);
/// physics_state_offboard_jump: landing COM height against landing height delta.
const COM_HEIGHT_GRAPH: u64 = 0x0dbb_a2ea_14ab_e028;
/// physics_state_offboard_jump: jump apex height against landing height delta.
const APEX_HEIGHT_GRAPH: u64 = 0xec7d_d184_f869_b215;

#[derive(Debug)]
pub(crate) enum JumpStateError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    /// offboard_jump.cpp:82, mResultValidCounter > 0.
    InvalidJumpingResult,
    /// offboard_jump.cpp:118, apex height >= 0.
    NegativeApexHeight,
    Cvar(InputCvarError),
    Numeric(ReckoningHostNumericError),
    Service(&'static str, String),
}
impl std::fmt::Display for JumpStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OffBoardJump lifecycle: {self:?}")
    }
}
type E = JumpStateError;
fn service(stage: &'static str, error: impl std::fmt::Debug) -> E {
    E::Service(stage, format!("{error:?}"))
}
fn jump<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    f: impl FnOnce(&mut OffBoardJump) -> Result<T, E>,
) -> Result<T, E> {
    let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
    let Some(StatePayload::OffBoardJump(state)) = states
        .states
        .get_mut(&StateKind::OffBoardJump)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(E::Missing("constructed OffBoardJump"));
    };
    f(state)
}
fn processed(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<Arc<Mutex<SkeletonProcessedContact>>, E> {
    Ok(Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .processed(),
    ))
}
/// `GetAttributePointer`, zeroed `DefaultDataArea` when absent, then an
/// N-point `PointGraphEval::Evaluate` with X at +0x10 and Y after N X values.
fn graph<const N: usize>(
    attributes: &AttributeRuntimeInstance,
    key: u64,
    input: f32,
) -> Result<f32, E> {
    let data = attributes
        .try_get_data(key, 0)
        .map_err(|e| service("OffBoardJump graph attribute", e))?;
    let word = |offset: usize| -> Result<f32, E> {
        match &data {
            None => Ok(0.0),
            Some(d) => Ok(f32::from_be_bytes(
                d.bytes()
                    .get(offset..offset + 4)
                    .ok_or(E::Missing("OffBoardJump graph data"))?
                    .try_into()
                    .expect("four graph bytes"),
            )),
        }
    };
    let mut x = [0.0; N];
    let mut y = [0.0; N];
    for (i, (x_point, y_point)) in x.iter_mut().zip(&mut y).enumerate() {
        *x_point = word(16 + i * 4)?;
        *y_point = word(16 + N * 4 + i * 4)?;
    }
    Ok(PointGraph::<N> { x, y }.evaluate(input))
}
fn fsel(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

impl PhysicalAssemblyWorld {
    fn validate_offboard_jump(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), E> {
        let same = self
            .try_controller_pair_matches_owner(owner, board, skeleton)
            .map_err(|e| service("pair", e))?;
        if !same {
            return Err(E::ForeignOwner);
        }
        jump(owner, |s| {
            let b = &s.base.physical;
            if !b.board.same_as(board)
                || !b.skeleton.same_as(skeleton)
                || !b.owner.upgrade().is_some_and(|v| Arc::ptr_eq(&v, owner))
            {
                return Err(E::ForeignOwner);
            }
            for (cell, member) in [
                (&s.off_board_feet_ik, OwnerMember::OffBoardFeetIK),
                (&s.skateboard_controller, OwnerMember::SkateboardController),
                (
                    &s.landing_on_deck_manager,
                    OwnerMember::LandingOnDeckManager,
                ),
                (&s.offboard_jumping_test, OwnerMember::OffboardJumpingTest),
            ] {
                let link = cell
                    .as_ref()
                    .and_then(|v| v.as_ref())
                    .ok_or(E::Missing("state collaborator link"))?;
                if link.member != member
                    || !link.owner.upgrade().is_some_and(|v| Arc::ptr_eq(&v, owner))
                {
                    return Err(E::ForeignOwner);
                }
            }
            Ok(())
        })
    }

    /// Enter82d9f3f8; the caller already selected the retained state503 owner.
    pub(crate) fn try_enter_pair_offboard_jump<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        self.validate_offboard_jump(owner, board, skeleton)?;
        jump(owner, |s| {
            s.body_spin = Some(0.0);
            s.body_spin_adjusted = Some(0.0);
            s.has_landed = Some(false);
            s.transition_angle = Some(0.0);
            s.is_leap_of_faith = Some(false);
            s.current_angle = Some(0.0);
            s.air_time = Some(0.0);
            s.total_air_time = Some(0.0);
            s.time_to_land = Some(0.0);
            s.com_height_on_landing = Some(0.0);
            s.trajectory = Some(RetainedTrajectory {
                position: V::known([0.0; 4]),
                velocity: V::known([0.0; 4]),
                gravity: V::known([0.0; 4]),
                time: [-1.0; 4],
            });
            Ok(())
        })?;
        let p = processed(owner)?;
        let last = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .last
            .ok_or(E::Missing("last state"))?;
        let reset = {
            let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
            let feet = remaining.feet_ik.as_mut().ok_or(E::Missing("feet IK"))?;
            if !feet.binding.matches(owner, &p, skeleton) {
                return Err(E::ForeignOwner);
            }
            feet.consider_reset_before_disable_ik(last)
        };
        if reset {
            self.try_wipeout_skeleton_operation(
                board,
                skeleton,
                WipeoutSkeletonOperation::DisableIk,
                numeric,
            )
            .map_err(|e| service("FeetIK.DisableIK", e))?;
        }
        owner
            .remaining()
            .lock()
            .map_err(|_| E::Poisoned)?
            .feet_ik
            .as_mut()
            .ok_or(E::Missing("feet IK"))?
            .finish_consider_reset(503);
        self.try_calculate_pair_offboard_jump_trajectory(owner, skeleton)?;
        self.try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::Set(4),
        )
        .map_err(|e| service("collision state4", e))
    }

    /// CalculateTrajectory82d985a8: ballistic COM path from the animated COM to
    /// the jumping test landing plus the authored landing COM height.
    fn try_calculate_pair_offboard_jump_trajectory(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), E> {
        let mut numeric = HostReckoningNumeric;
        let (counter, landing, valid, leap) = {
            let remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
            let test = remaining
                .jumping
                .as_ref()
                .ok_or(E::Missing("OffboardJumpingTest"))?;
            (
                test.result_valid_counter,
                V::known(test.landing_position),
                test.landing_position_valid,
                test.leap_of_faith,
            )
        };
        if counter <= 0 {
            return Err(E::InvalidJumpingResult);
        }
        jump(owner, |s| {
            s.is_leap_of_faith = Some(leap);
            Ok(())
        })?;
        let ground = processed(owner)?
            .lock()
            .map_err(|_| E::Poisoned)?
            .ground_position_fields()
            .effective_animation
            .ok_or(E::Missing("effective animation"))?
            .columns()[3];
        let landing = if valid { landing } else { ground };
        let delta = landing.xyz[1] - ground.xyz[1];
        let attributes = jump(owner, |s| {
            s.attribs
                .clone()
                .ok_or(E::Missing("OffBoardJump attributes"))
        })?;
        let com_height = graph::<8>(&attributes, COM_HEIGHT_GRAPH, delta)?;
        jump(owner, |s| {
            s.com_height_on_landing = Some(com_height);
            Ok(())
        })?;
        let anim_com = self
            .skeleton(skeleton)
            .map_err(|e| service("Skeleton", e))?
            .anim_centre_of_mass();
        let landing_com = landing.zip(V::known([0.0, com_height, 0.0, 0.0]), |a, b| a + b);
        let apex = graph::<4>(&attributes, APEX_HEIGHT_GRAPH, delta)?;
        if apex < 0.0 {
            return Err(E::NegativeApexHeight);
        }
        let square = apex * f32::from_bits(0x419c_cccd);
        let estimate = numeric
            .reciprocal_sqrt_estimate(square)
            .map_err(E::Numeric)?;
        let launch = length_from_squared_and_estimate(square, estimate);
        let rise = landing_com.xyz[1] - anim_com.xyz[1];
        let drop = -rise * f32::from_bits(0xc19c_cccd);
        let discriminant = launch.mul_add(launch, -drop);
        let clamped = fsel(discriminant, discriminant, 0.0);
        let estimate = numeric
            .reciprocal_sqrt_estimate(clamped)
            .map_err(E::Numeric)?;
        let root = length_from_squared_and_estimate(clamped, estimate);
        let time = f32::from_bits(0xbdd0_fac6) * (-launch - root);
        let total = fsel(-time, 0.0, time);
        let estimate = numeric.reciprocal_estimate(total).map_err(E::Numeric)?;
        let inverse = refine_drive_reciprocal(total, estimate);
        let displacement = landing_com.zip(anim_com, |l, a| l - a);
        let fall = GRAVITY.map(|g| (g * 0.5) * total);
        let velocity = displacement.map(|d| inverse * d).zip(fall, |d, f| d - f);
        jump(owner, |s| {
            s.total_air_time = Some(total);
            s.trajectory = Some(RetainedTrajectory {
                position: anim_com,
                velocity,
                gravity: GRAVITY,
                time: [-1.0; 4],
            });
            s.air_time = Some(0.0);
            s.time_to_land = Some(total);
            Ok(())
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "retained pair, world services and the Skeleton numeric owners"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "one native Update: FeetIK, timers, trajectory, land-on-board gate, Skeleton"
    )]
    pub(crate) fn try_update_pair_offboard_jump(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
        sin_cos: &mut HostReckoningSinCosNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, E> {
        self.validate_offboard_jump(owner, board, skeleton)?;
        self.try_update_pair_offboard_air_feet(owner, board, skeleton, refs)
            .map_err(|e| service("FeetIK.UpdateAir", e))?;
        let height = owner
            .remaining()
            .lock()
            .map_err(|_| E::Poisoned)?
            .feet_ik
            .as_ref()
            .ok_or(E::Missing("feet IK"))?
            .height_above_ground;
        let (air_time, time_to_land) = jump(owner, |s| {
            let air = s.air_time.ok_or(E::Missing("air time"))? + DT;
            let total = s.total_air_time.ok_or(E::Missing("total air time"))?;
            s.air_time = Some(air);
            s.time_to_land = Some(total - air);
            Ok((air, total - air))
        })?;
        let p = processed(owner)?;
        let mut time_to_land = time_to_land;
        // fcmpu/bge 82d989c8: only an ordered negative height reads COM velocity.
        if height < 0.0 {
            let velocity = p
                .lock()
                .map_err(|_| E::Poisoned)?
                .prior_output()
                .centre_of_mass_velocity()
                .ok_or(E::Missing("COM velocity"))?
                .value;
            if 0.0 > velocity.xyz[1] {
                time_to_land = 0.0;
                jump(owner, |s| {
                    s.time_to_land = Some(0.0);
                    Ok(())
                })?;
            }
        }
        if p.lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .flags_8d0
            .ok_or(E::Missing("flags8d0"))?
            & 0x800
            != 0
        {
            self.try_request_pair_drop_board(owner, board, skeleton, refs, numeric)
                .map_err(|e| service("RequestDropBoard", e))?;
        }
        let trajectory = jump(owner, |s| s.trajectory.ok_or(E::Missing("trajectory")))?;
        let com_position = position_at(&trajectory, air_time);
        jump(owner, |s| {
            s.has_landed = Some(!(time_to_land > 0.0));
            Ok(())
        })?;
        let (side_axis, forward_axis, anim_position, jump_x, jump_z, physical_position) = {
            let guard = p.lock().map_err(|_| E::Poisoned)?;
            let effective = guard
                .ground_position_fields()
                .effective_animation
                .ok_or(E::Missing("effective animation"))?;
            let attributes = guard.animation_attribute_fields();
            (
                effective.columns()[0],
                effective.columns()[2],
                effective.columns()[3],
                attributes.offboard_jump_x.ok_or(E::Missing("OB_JumpX"))?,
                attributes.offboard_jump_z.ok_or(E::Missing("OB_JumpZ"))?,
                guard
                    .effective_transform_field()
                    .ok_or(E::Missing("effective transform"))?
                    .columns()[3],
            )
        };
        let mut horizontal = trajectory.velocity;
        horizontal.xyz[1] = 0.0;
        let side = side_axis.map(|v| v * jump_x);
        let forward = forward_axis.map(|v| v * jump_z);
        let input = forward.zip(side, |f, s| f - s);
        let zero = V::known([0.0; 4]);
        let velocity_direction =
            try_normalize_safe(horizontal, zero, numeric).map_err(E::Numeric)?;
        let offset_direction = try_normalize_safe(
            physical_position.zip(anim_position, |b, a| b - a),
            zero,
            numeric,
        )
        .map_err(E::Numeric)?;
        let query = SMALL > numeric.dot_vector(input, input).map_err(E::Numeric)?
            || numeric
                .dot_vector(offset_direction, input)
                .map_err(E::Numeric)?
                >= 0.0
            || numeric
                .dot_vector(offset_direction, velocity_direction)
                .map_err(E::Numeric)?
                >= 0.0;
        if query {
            let velocity = velocity_at(&trajectory, air_time);
            // fcmpu/bge 82d98c4c: an ordered air time below 0.06 is the standing jump.
            let standing = air_time < f32::from_bits(0x3d75_c28f);
            let assist = refs
                .try_offboard_jump_land_on_board_assist(standing)
                .map_err(E::Cvar)?;
            let com = p
                .lock()
                .map_err(|_| E::Poisoned)?
                .prior_output()
                .centre_of_mass()
                .ok_or(E::Missing("COM"))?
                .value;
            self.try_start_pair_land_on_board_query(
                owner, skeleton, com, velocity, assist, ground, refs, numeric,
            )
            .map_err(|e| service("Landing.StartLandOnBoardQuery", e))?;
        }
        let turn = jump(owner, |s| {
            s.body_spin_adjusted.ok_or(E::Missing("body spin adjusted"))
        })?;
        self.try_update_pair_offboard_air_skeleton(
            owner,
            board,
            skeleton,
            com_position,
            turn,
            true,
            refs,
            ground,
            numeric,
            sin_cos,
        )
        .map_err(|e| service("Skeleton.UpdateOffBoardAir", e))
    }

    /// UpdatePostPhysics82d899f8: OffBoardAir wipeout check, LandingOnDeck
    /// post-physics, then the leap-of-faith request26 before landing.
    pub(crate) fn try_post_pair_offboard_jump<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        self.validate_offboard_jump(owner, board, skeleton)?;
        self.try_check_offboard_air_wipeout(board, skeleton, owner, numeric)
            .map_err(|e| service("CheckForOffBoardAirWipeout", e))?;
        self.try_update_pair_land_on_board_postphysics(owner, refs)
            .map_err(|e| service("Landing.UpdatePostPhysics", e))?;
        let (leap, time_to_land) = jump(owner, |s| {
            Ok((
                s.is_leap_of_faith.ok_or(E::Missing("leap of faith"))?,
                s.time_to_land.ok_or(E::Missing("time to land"))?,
            ))
        })?;
        if leap
            && time_to_land < f32::from_bits(0x3da3_d70a)
            && processed(owner)?
                .lock()
                .map_err(|_| E::Poisoned)?
                .physics_state_fields()
                .flags_8d0
                .ok_or(E::Missing("flags8d0"))?
                & 0x0020_0000
                == 0
        {
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_ground_request(26)
                .map_err(|e| service("wipeout request26", e))?;
        }
        Ok(())
    }

    pub(crate) fn try_fill_pair_offboard_jump(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        self.validate_offboard_jump(owner, board, skeleton)?;
        self.try_fill_pair_land_on_board_output(owner, refs)
            .map_err(|e| service("Landing.FillPhysOut", e))?;
        let mut source = OutputSource { owner };
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_fill_offboard_jump_state(&mut source)
            .map_err(|e| service("OffBoardJump.FillPhysOut", e))
    }
}

struct OutputSource<'a> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
}
impl JumpOutputSource for OutputSource<'_> {
    type Error = E;
    fn time_to_land(&mut self) -> Result<f32, E> {
        jump(self.owner, |s| {
            s.time_to_land.ok_or(E::Missing("time to land"))
        })
    }
    fn com_height_on_landing(&mut self) -> Result<f32, E> {
        jump(self.owner, |s| {
            s.com_height_on_landing
                .ok_or(E::Missing("COM height on landing"))
        })
    }
    fn has_landed(&mut self) -> Result<bool, E> {
        jump(self.owner, |s| s.has_landed.ok_or(E::Missing("has landed")))
    }
    fn foot_down(&mut self, left: bool) -> Result<bool, E> {
        let r = self.owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let f = r.feet_ik.as_ref().ok_or(E::Missing("feet IK"))?;
        Ok(if left {
            f.left_foot_down
        } else {
            f.right_foot_down
        })
    }
}
