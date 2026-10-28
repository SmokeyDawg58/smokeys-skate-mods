//! PhysState_OffBoard_Air lifecycle82da08d0/82d94158/82d885a0/82d94510.
//! Owns no replacement state, trajectory, processed input, or manager.
use crate::physics_state_ids::{PHYSICS_CATEGORY_AIR, PHYSICS_STATE_OFFBOARD_PUSHING};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, wipeout::WipeoutSkeletonOperation,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences, offboard_air_state::AirCvar},
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{
            OffBoardAir, OffboardTrajectoryResult, OwnerMember, StateKind, StatePayload,
        },
    },
    physical_player_footplant::RetainedTrajectory,
    physical_player_phys_out::construct::{
        offboard_air_state::AirOutputSource, skeleton_fill_suffix::plane_angle,
    },
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    physical_player_trajectory_owners::{OffboardLaunchInfo, offboard_runtime::position_at},
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric, wrap_turns_to_radians,
    },
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::physical_skeleton_controller::SkeletonCollisionTransition,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector as V},
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};
const DT: f32 = f32::from_bits(0x3c88_8889);
const DEG: f32 = f32::from_bits(0x3c8e_fa35);
const INV_TWO_PI: f32 = f32::from_bits(0x3e22_f983);
const GRAVITY: V = V::known([0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0]); // init82f76360
const UP: V = V::known([0.0, 1.0, 0.0, 0.0]);

#[derive(Debug)]
pub(crate) enum AirStateError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    Numeric(ReckoningHostNumericError),
    NegativeJumpHeight,
    Service(&'static str, String),
}
impl std::fmt::Display for AirStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OffBoardAir lifecycle: {self:?}")
    }
}
type E = AirStateError;
fn service(stage: &'static str, error: impl std::fmt::Debug) -> E {
    E::Service(stage, format!("{error:?}"))
}
fn air<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    f: impl FnOnce(&mut OffBoardAir) -> Result<T, E>,
) -> Result<T, E> {
    let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
    let Some(StatePayload::OffBoardAir(state)) = states
        .states
        .get_mut(&StateKind::OffBoardAir)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(E::Missing("constructed OffBoardAir"));
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
fn scalar(attributes: &AttributeRuntimeInstance, key: u64) -> Result<f32, E> {
    Ok(f32::from_bits(
        attributes
            .try_scalar_word(key, 0)
            .map_err(E::Attribute)?
            .unwrap_or(0),
    ))
}
fn cvar(refs: &InputCvarReferences, key: AirCvar) -> Result<f32, E> {
    refs.try_offboard_air_state(key).map_err(E::Cvar)
}
fn selector_reset(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Result<(), E> {
    let p = processed(owner)?;
    let mut selectors = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
    let s = selectors
        .offboard
        .as_mut()
        .ok_or(E::Missing("offboard selector"))?;
    if !s.bindings.matches(owner, &p) {
        return Err(E::ForeignOwner);
    }
    s.reset();
    Ok(())
}

impl PhysicalAssemblyWorld {
    fn validate_offboard_air(
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
        air(owner, |s| {
            let b = &s.base.physical;
            if !b.board.same_as(board)
                || !b.skeleton.same_as(skeleton)
                || !b.owner.upgrade().is_some_and(|v| Arc::ptr_eq(&v, owner))
            {
                return Err(E::ForeignOwner);
            }
            for (cell, member) in [
                (
                    &s.offboard_trajectory_selector,
                    OwnerMember::OffboardTrajectorySelector,
                ),
                (&s.off_board_feet_ik, OwnerMember::OffBoardFeetIK),
                (&s.skateboard_controller, OwnerMember::SkateboardController),
                (
                    &s.landing_on_deck_manager,
                    OwnerMember::LandingOnDeckManager,
                ),
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
    /// Enter82da08d0; caller has selected the same retained state502 owner.
    pub(crate) fn try_enter_pair_offboard_air<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        self.validate_offboard_air(owner, board, skeleton)?;
        air(owner, |s| {
            s.body_spin = Some(0.0);
            s.air_frame = Some(0);
            s.body_spin_adjusted = Some(0.0);
            s.has_landed = Some(false);
            s.transition_angle = Some(0.0);
            s.request_wipeout = Some(false);
            s.current_angle = Some(0.0);
            let r = s
                .offboard_trajectory_result
                .as_mut()
                .ok_or(E::Missing("trajectory result"))?;
            r.reset();
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
            .finish_consider_reset(502);
        selector_reset(owner)?;
        {
            let mut r = owner.remaining().lock().map_err(|_| E::Poisoned)?;
            let landing = r.landing.as_mut().ok_or(E::Missing("landing manager"))?;
            if !landing.skeleton.matches(owner, &p, skeleton) || !landing.world.matches(owner) {
                return Err(E::ForeignOwner);
            }
            landing.reset_after_teleport();
        }
        let launch = prepare_launch(owner, refs, numeric)?;
        self.try_launch_pair_offboard_trajectory(owner, board, skeleton, launch, ground, numeric)
            .map_err(|e| service("selector Launch", e))?;
        // Native raw HIPS transform = raw AnimToWorld * mapped volume PART_HIPS.
        let normal = self
            .skeleton(skeleton)
            .map_err(|e| service("Skeleton", e))?;
        // GetLastAnimVolumeTransform82b3d498 reads the saved AnimRecord.
        let hips = &normal.current_animation().record().poses()[23];
        let hips = normal
            .root_transforms()
            .animation_to_world()
            .compose(&SkeletonRootMatrix::known(*hips));
        let effective = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .ground_position_fields()
            .effective_animation
            .ok_or(E::Missing("effective animation"))?;
        let angle = plane_angle(hips.columns()[1], effective.columns()[2], UP, numeric)
            .map_err(|e| service("numeric", e))?;
        air(owner, |s| {
            s.transition_angle = Some(wrap_turns_to_radians(angle * INV_TWO_PI));
            Ok(())
        })?;
        self.try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::Set(5),
        )
        .map_err(|e| service("collision state5", e))?;
        Ok(())
    }
    /// Exit82d88318 has exactly the selector Reset child.
    pub(crate) fn try_exit_pair_offboard_air(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), E> {
        self.validate_offboard_air(owner, board, skeleton)?;
        selector_reset(owner)
    }
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn try_update_pair_offboard_air(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
        sin_cos: &mut HostReckoningSinCosNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, E> {
        self.validate_offboard_air(owner, board, skeleton)?;
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
        let frame = air(owner, |s| {
            let f = s.air_frame.ok_or(E::Missing("air frame"))?.wrapping_add(1);
            s.air_frame = Some(f);
            Ok(f)
        })?;
        let p = processed(owner)?;
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
        let adjusted = -fsel(height, height, 0.0);
        let result = self
            .try_pair_offboard_trajectory_frame(owner, frame, adjusted, numeric)
            .map_err(|e| service("selector Frame", e))?;
        air(owner, |s| {
            s.offboard_trajectory_result = Some(OffboardTrajectoryResult {
                com_position: result.com_position,
                com_velocity: result.com_velocity,
                landing_normal: result.landing_normal,
                landing_velocity: result.landing_velocity,
                landing_position: result.landing_position,
                time_to_land: result.time_to_land,
                frame: result.frame,
                landing_valid: result.landing_valid,
            });
            Ok(())
        })?;
        // Native BO4/BI24 skips unless height is ordered LT; timer uses GT.
        let landed = has_landed(height, || {
            let timer = p
                .lock()
                .map_err(|_| E::Poisoned)?
                .physics_state_fields()
                .state_timer
                .ok_or(E::Missing("state timer"))?;
            Ok((timer, cvar(refs, AirCvar::MinTimeInAirForRunoff)?))
        })?;
        air(owner, |s| {
            s.has_landed = Some(landed);
            Ok(())
        })?;
        if std::env::var_os("SKATE2_TRACE_OFFBOARD_AIR").is_some() {
            let feet = owner.remaining().lock().map_err(|_| E::Poisoned)?;
            let feet = feet.feet_ik.as_ref().ok_or(E::Missing("feet IK"))?;
            eprintln!(
                "offboard-air physics: frame={frame} height={height} time_to_land={} landing_valid={} com_y={} com_vy={} landed={landed} surface_valid=[{},{}] anim_toe_y=[{},{}]",
                result.time_to_land,
                result.landing_valid,
                result.com_position.xyz[1],
                result.com_velocity.xyz[1],
                feet.result_left.valid,
                feet.result_right.valid,
                feet.current_animation_position_left.xyz[1],
                feet.current_animation_position_right.xyz[1],
            );
        }
        update_spin(owner, refs, numeric)?;
        let angle = air(owner, |s| {
            s.current_angle.ok_or(E::Missing("current angle"))
        })?
        .abs();
        let air_threshold = cvar(refs, AirCvar::RunOutMinAngleForWipeoutAir)? * DEG;
        let ground_threshold = cvar(refs, AirCvar::RunOutMinAngleForWipeout)? * DEG;
        let air_bad = if angle > air_threshold {
            let flags = p
                .lock()
                .map_err(|_| E::Poisoned)?
                .physics_state_fields()
                .flags_8d4;
            if flags.1 & 0x0100_0000 == 0 {
                return Err(E::Missing("known8d4 high-byte bit0"));
            }
            flags.0 & 0x0100_0000 == 0
        } else {
            false
        };
        let ground_bad = landed && angle > ground_threshold;
        let mut trajectory_bad = false;
        {
            let guard = p.lock().map_err(|_| E::Poisoned)?;
            let flags = guard
                .physics_state_fields()
                .flags_8d0
                .ok_or(E::Missing("flags8d0"))?;
            if flags & 0x200000 == 0 && !result.landing_valid {
                trajectory_bad = guard
                    .physics_state_fields()
                    .state_timer
                    .ok_or(E::Missing("state timer"))?
                    > 0.1;
            }
        }
        if !trajectory_bad {
            let velocity = p
                .lock()
                .map_err(|_| E::Poisoned)?
                .prior_output()
                .centre_of_mass_velocity()
                .ok_or(E::Missing("COM velocity"))?
                .value;
            trajectory_bad = cvar(refs, AirCvar::OffBoarAirYSpeedForWipeout)? > velocity.xyz[1];
            if !trajectory_bad {
                let effective = p
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .ground_position_fields()
                    .effective_animation
                    .ok_or(E::Missing("effective animation"))?;
                trajectory_bad =
                    cvar(refs, AirCvar::OBAirMinUpVectorY)? > effective.columns()[1].xyz[1];
            }
        }
        let request = air_bad || ground_bad || trajectory_bad;
        air(owner, |s| {
            s.request_wipeout = Some(request);
            Ok(())
        })?;
        let velocity = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .centre_of_mass_velocity()
            .ok_or(E::Missing("COM velocity"))?
            .value;
        let assist = cvar(
            refs,
            if frame < 4 {
                AirCvar::StandingJumpMaxLandOnBoardAssist
            } else {
                AirCvar::OffboardAirMaxLandOnBoardAssist
            },
        )?;
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
        let (target, turn, reorient) = if request {
            let p = p.lock().map_err(|_| E::Poisoned)?;
            let com = p
                .prior_output()
                .centre_of_mass()
                .ok_or(E::Missing("COM"))?
                .value;
            let velocity = p
                .prior_output()
                .centre_of_mass_velocity()
                .ok_or(E::Missing("COM velocity"))?
                .value;
            let trajectory = RetainedTrajectory {
                position: com,
                velocity,
                gravity: GRAVITY,
                time: [0.0; 4],
            };
            (position_at(&trajectory, DT), 0.0, false)
        } else {
            (
                result.com_position,
                air(owner, |s| {
                    s.body_spin_adjusted.ok_or(E::Missing("body spin adjusted"))
                })?,
                true,
            )
        };
        self.try_update_pair_offboard_air_skeleton(
            owner, board, skeleton, target, turn, reorient, refs, ground, numeric, sin_cos,
        )
        .map_err(|e| service("Skeleton.UpdateOffBoardAir", e))
    }

    pub(crate) fn try_post_pair_offboard_air<N: ReckoningNumeric>(
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
        self.validate_offboard_air(owner, board, skeleton)?;
        self.try_update_pair_land_on_board_postphysics(owner, refs)
            .map_err(|e| service("Landing.UpdatePostPhysics", e))?;
        let p = processed(owner)?;
        let timer = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .state_timer
            .ok_or(E::Missing("state timer"))?;
        if timer > cvar(refs, AirCvar::MinTimeInStateForSkateboardWipeout)? {
            let wheels = p
                .lock()
                .map_err(|_| E::Poisoned)?
                .physics_state_fields()
                .num_wheels_in_contact
                .ok_or(E::Missing("wheel contacts"))?;
            if wheels > 0
                && self
                    .skeleton(skeleton)
                    .map_err(|e| service("Skeleton", e))?
                    .state()
                    .collision
                    .foot_hit_skateboard
                    != 0
            {
                owner
                    .wipeout()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .try_ground_request(29)
                    .map_err(|e| service("wipeout request29", e))?;
            }
        }
        if air(owner, |s| {
            s.request_wipeout.ok_or(E::Missing("request wipeout"))
        })? {
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_ground_request(27)
                .map_err(|e| service("wipeout request27", e))?;
        }
        self.try_check_offboard_air_wipeout(board, skeleton, owner, numeric)
            .map_err(|e| service("CheckForOffBoardAirWipeout", e))
    }
    pub(crate) fn try_fill_pair_offboard_air(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        self.validate_offboard_air(owner, board, skeleton)?;
        self.try_fill_pair_land_on_board_output(owner, refs)
            .map_err(|e| service("Landing.FillPhysOut", e))?;
        let mut source = OutputSource { owner, refs };
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_fill_offboard_air_state(&mut source)
            .map_err(|e| service("OffBoardAir.FillPhysOut", e))
    }
}

fn prepare_launch<N: ReckoningNumeric>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    refs: &InputCvarReferences,
    numeric: &mut N,
) -> Result<OffboardLaunchInfo, E>
where
    N::Error: std::fmt::Debug,
{
    let p = processed(owner)?;
    let mut velocity = p
        .lock()
        .map_err(|_| E::Poisoned)?
        .prior_output()
        .centre_of_mass_velocity()
        .ok_or(E::Missing("COM velocity"))?
        .value;
    let flags = p
        .lock()
        .map_err(|_| E::Poisoned)?
        .physics_state_fields()
        .flags_8cc
        .ok_or(E::Missing("flags8cc"))?;
    if flags & 0x100000 != 0 {
        let attrs = air(owner, |s| {
            s.attribs
                .clone()
                .ok_or(E::Missing("OffBoardAir attributes"))
        })?;
        if scalar(&attrs, 0x45c5_a49d_09a5_a3e3)? < 0.0 {
            return Err(E::NegativeJumpHeight);
        }
        let height = scalar(&attrs, 0x45c5_a49d_09a5_a3e3)?;
        let z = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .ground_position_fields()
            .effective_animation
            .ok_or(E::Missing("effective animation"))?
            .columns()[2];
        let forward = cvar(refs, AirCvar::JumpForwardDistance)?;
        velocity = standing_jump_velocity(height, z, forward, numeric)?;
    } else {
        let fields = *p.lock().map_err(|_| E::Poisoned)?.physics_state_fields();
        if fields.flags_8d0.ok_or(E::Missing("flags8d0"))? & 0x400 != 0
            && fields.last_category.ok_or(E::Missing("last category"))? != PHYSICS_CATEGORY_AIR
            && fields.last.ok_or(E::Missing("last state"))? != PHYSICS_STATE_OFFBOARD_PUSHING
        {
            let nudge = cvar(refs, AirCvar::RunoutUpwardVelocityNudge)?;
            let reduction = cvar(refs, AirCvar::RunoutZXVelocityReduction)?;
            velocity.xyz[1] = fsel(nudge - velocity.xyz[1], nudge, velocity.xyz[1]);
            velocity.xyz[0] *= reduction;
            velocity.xyz[2] *= reduction;
        }
    }
    let com = p
        .lock()
        .map_err(|_| E::Poisoned)?
        .prior_output()
        .centre_of_mass()
        .ok_or(E::Missing("COM"))?
        .value;
    Ok(OffboardLaunchInfo {
        start_velocity: velocity,
        start_com_position: com,
        cone_angle_x: 0.0,
        cone_angle_z: 0.0,
        num_trajectories: 1,
    })
}
fn standing_jump_velocity<N: ReckoningNumeric>(
    height: f32,
    z: V,
    forward: f32,
    numeric: &mut N,
) -> Result<V, E>
where
    N::Error: std::fmt::Debug,
{
    let square = ((GRAVITY.xyz[1] * -1.0) * 2.0) * height;
    let seed = numeric
        .reciprocal_sqrt_estimate(square)
        .map_err(|e| service("numeric", e))?;
    let refined = crate::simulation_drive_jacobian::refine_drive_reciprocal_sqrt(square, seed);
    let up = if square == 0.0 { 0.0 } else { square * refined };
    let time = (up * -2.0) / GRAVITY.xyz[1];
    let mut velocity = z.map(|lane| lane * (forward / time));
    velocity.xyz[1] = up;
    Ok(velocity)
}
fn update_spin(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    refs: &InputCvarReferences,
    numeric: &mut HostReckoningNumeric,
) -> Result<(), E> {
    let (velocity, attrs, transition, old, ttl) = air(owner, |s| {
        Ok((
            s.offboard_trajectory_result
                .as_ref()
                .ok_or(E::Missing("trajectory result"))?
                .com_velocity,
            s.attribs.clone().ok_or(E::Missing("attributes"))?,
            s.transition_angle.ok_or(E::Missing("transition angle"))?,
            s.body_spin.ok_or(E::Missing("body spin"))?,
            s.offboard_trajectory_result.as_ref().unwrap().time_to_land,
        ))
    })?;
    let p = processed(owner)?;
    let z = p
        .lock()
        .map_err(|_| E::Poisoned)?
        .ground_position_fields()
        .effective_animation
        .ok_or(E::Missing("effective animation"))?
        .columns()[2];
    let max_delta = scalar(&attrs, 0x8e63_3ab5_e200_94a4)?;
    let maximum = scalar(&attrs, 0x591b_1daa_4424_6a1f)?;
    let timer = p
        .lock()
        .map_err(|_| E::Poisoned)?
        .physics_state_fields()
        .state_timer
        .ok_or(E::Missing("state timer"))?;
    let mut correction = 0.0;
    let transition_time = cvar(refs, AirCvar::RunoutTransitionTime)?;
    if timer < transition_time {
        correction = -((1.0 - timer / transition_time) * transition);
    }
    let angle = plane_angle(velocity, z, UP, numeric).map_err(E::Numeric)?;
    let current = -wrap_turns_to_radians((angle + correction) * INV_TWO_PI);
    air(owner, |s| {
        s.current_angle = Some(current);
        Ok(())
    })?;
    let time = fsel(DT - ttl, DT, ttl);
    let proportional = cvar(refs, AirCvar::RunoutProportionalAngle)? * current;
    let divisor =
        (time * cvar(refs, AirCvar::RunoutTimeToLandScalar)?) * f32::from_bits(0x426f_ffff);
    let rate = current / divisor;
    // Native BO4/BI24 skips overwrite unless the comparison is ordered LT.
    let rate = if proportional.abs() < rate.abs() {
        proportional
    } else {
        rate
    };
    let bounded = fsel(
        maximum - fsel(-maximum - rate, -maximum, rate),
        fsel(-maximum - rate, -maximum, rate),
        maximum,
    );
    let lower = old - max_delta;
    let upper = old + max_delta;
    let spin = fsel(
        upper - fsel(lower - bounded, lower, bounded),
        fsel(lower - bounded, lower, bounded),
        upper,
    );
    air(owner, |s| {
        s.body_spin_adjusted = Some(spin);
        s.body_spin = Some(spin);
        Ok(())
    })?;
    let divisor = cvar(refs, AirCvar::RunoutTransitionTime)?;
    if timer < divisor {
        let adjusted = -(transition / divisor).mul_add(DT, -spin);
        air(owner, |s| {
            s.body_spin_adjusted = Some(adjusted);
            Ok(())
        })?;
    }
    Ok(())
}
fn fsel(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
fn has_landed<E>(height: f32, times: impl FnOnce() -> Result<(f32, f32), E>) -> Result<bool, E> {
    if height < 0.0 {
        let (timer, minimum) = times()?;
        Ok(timer > minimum)
    } else {
        Ok(false)
    }
}
struct OutputSource<'a> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    refs: &'a InputCvarReferences,
}
impl AirOutputSource for OutputSource<'_> {
    type Error = E;
    fn time_to_land(&mut self) -> Result<f32, E> {
        air(self.owner, |s| {
            Ok(s.offboard_trajectory_result
                .as_ref()
                .ok_or(E::Missing("trajectory result"))?
                .time_to_land)
        })
    }
    fn landing_velocity_y(&mut self) -> Result<f32, E> {
        air(self.owner, |s| {
            Ok(s.offboard_trajectory_result
                .as_ref()
                .ok_or(E::Missing("trajectory result"))?
                .landing_velocity
                .xyz[1])
        })
    }
    fn trajectory_ready(&mut self) -> Result<bool, E> {
        Ok(self
            .owner
            .trajectories()
            .lock()
            .map_err(|_| E::Poisoned)?
            .offboard
            .as_ref()
            .ok_or(E::Missing("offboard selector"))?
            .ready)
    }
    fn has_landed_for_locomotion(&mut self) -> Result<bool, E> {
        let maximum = cvar(self.refs, AirCvar::RunOutMaxAngleForLocomotion)? * DEG;
        air(self.owner, |s| {
            Ok(s.has_landed.ok_or(E::Missing("has landed"))?
                && s.current_angle.ok_or(E::Missing("current angle"))?.abs() < maximum)
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonnegative_and_unordered_height_do_not_read_timer_or_minimum() {
        for height in [0.0, -0.0, 1.0, f32::NAN] {
            assert!(!has_landed::<()>(height, || panic!("native skips these reads")).unwrap());
        }
        assert!(!has_landed::<()>(-1.0, || Ok((f32::NAN, 0.1))).unwrap());
        assert!(!has_landed::<()>(-1.0, || Ok((0.1, 0.1))).unwrap());
        assert!(has_landed::<()>(-1.0, || Ok((0.2, 0.1))).unwrap());
    }
    #[test]
    fn standing_launch_preserves_unknown_w_and_source_zero_time_nonfinite_horizontal() {
        let z = V {
            xyz: [0.0, 0.0, 1.0],
            w: None,
        };
        let mut numeric = HostReckoningNumeric;
        let jump = standing_jump_velocity(1.0, z, 2.0, &mut numeric).unwrap();
        assert!(jump.xyz[1] > 4.4 && jump.xyz[1] < 4.5);
        assert!(jump.xyz[2] > 2.2 && jump.xyz[2] < 2.3);
        assert_eq!(jump.w, None);
        let zero = standing_jump_velocity(0.0, z, 2.0, &mut numeric).unwrap();
        assert_eq!(zero.xyz[1].to_bits(), 0);
        assert!(zero.xyz[0].is_nan());
        assert!(zero.xyz[2].is_infinite());
        assert_eq!(zero.w, None);
    }
}
