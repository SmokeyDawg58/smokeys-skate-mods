//! `PhysState_FootPlant` (601): Enter 0x82d89420 with StartFootPlantGround
//! 0x82dbc6a8, Update 0x82d976b0 with UpdateFootPlantGround 0x82dbcf40 and
//! `Skeleton::UpdateHandPlant` 0x82b5da30, Exit 0x82d89468 (FootPlantManager
//! Reset) and UpdatePostPhysics 0x82d977a8 with CheckForFootPlantWipeouts
//! 0x82d89470. Its FillPhysOut and GetSyncLog vtable slots are the shared BLR
//! 0x825187c0; the manager's own Fill runs from Update5.
use super::{
    footplant_air::{
        DEGREES_TO_RADIANS, FootPlantUpdateError, UPDATE_TIME, ZERO, graph, motion, processed,
        scalar, service, wrap_angle,
    },
    ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric},
};
use crate::{
    debug_overlay::DebugOverlaySlot,
    dynamic_assembly_session::{
        skater_pool::SimulationSkaterAssemblyHandle as Handle,
        skeleton_normal::general_owner::SkeletonGeneralContinuation,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{FootPlant, StateKind, StatePayload},
    },
    physical_player_footplant::FootPlantManager,
    physical_player_phys_out::construct::skeleton_fill_suffix::plane_angle,
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_trajectory_owners::{
        RetainedTrajectoryLaunchInfo, skeleton_launch_info::try_fill_launch_info,
    },
    reckoning_numeric::{self as math, ReckoningNumeric},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

type E = FootPlantUpdateError;

/// `FootPlantGroundOutput` (PDB 0x40): COM, COM velocity, take-off position and
/// flag. Native velocity is written only on take-off.
struct GroundOutput {
    com_position: V,
    com_velocity: Option<V>,
    take_off_position: V,
    take_off: bool,
}

fn state<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &Handle,
    skeleton: &Handle,
    f: impl FnOnce(&mut FootPlant) -> Result<T, E>,
) -> Result<T, E> {
    let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
    let Some(StatePayload::FootPlant(s)) = states
        .states
        .get_mut(&StateKind::FootPlant)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(E::Missing("retained FootPlant"));
    };
    let binding = &s.base.physical;
    if !binding.board.same_as(board)
        || !binding.skeleton.same_as(skeleton)
        || !binding
            .owner
            .upgrade()
            .is_some_and(|v| Arc::ptr_eq(&v, owner))
    {
        return Err(E::ForeignOwner);
    }
    f(s)
}

impl PhysicalAssemblyWorld {
    /// Enter 0x82d89420: `EnableAngularDriveOnly(1.0)` on the board, the
    /// SkeletonController foot-collision disable, then StartFootPlantGround.
    pub(crate) fn try_enter_pair_foot_plant<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        state(owner, board, skeleton, |_| Ok(()))?;
        #[cfg(feature = "skate3-runtime")]
        let source={
            let mut foot=owner.footplant().lock().map_err(|_|E::Poisoned)?;
            foot.try_manager(owner,skeleton).map_err(|e|service("FootPlantManager",e))?
                .source3.as_ref().is_some_and(|source|source.active)
        };
        #[cfg(not(feature = "skate3-runtime"))]
        let source=false;
        if source {
            #[cfg(feature = "skate3-runtime")]
            self.try_enable_source_plant_drive(board,false).map_err(|e|service("S3 FootPlant drive",e))?;
        } else {
            self.try_enable_board_angular_drive_only(board,1.0,refs,numeric)
                .map_err(|e|service("EnableAngularDriveOnly",e))?;
        }
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.disable_feet_collisions();
            })
            .map_err(|e| service("Skeleton pair", e))?;
        let processed = processed(owner)?;
        let mut foot = owner.footplant().lock().map_err(|_| E::Poisoned)?;
        let manager = foot
            .try_manager(owner, skeleton)
            .map_err(|e| service("FootPlantManager", e))?;
        self.try_start_foot_plant_ground(owner, board, skeleton, manager, &processed, numeric)
    }

    /// StartFootPlantGround 0x82dbc6a8. The COM-velocity component along the
    /// leg (IK foot to COM) selects the graph gain; the leg swings about the
    /// COM-offset x velocity axis by the plane angle to up, limited by the
    /// lead/lag and floor attributes, which sets the total time, exit velocity
    /// and the four Bezier points. A near-zero swing axis returns before them.
    #[expect(
        clippy::too_many_lines,
        reason = "one source-ordered plant setup with its Skeleton write"
    )]
    fn try_start_foot_plant_ground<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        manager: &mut FootPlantManager,
        processed: &Mutex<SkeletonProcessedContact>,
        n: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        #[cfg(feature = "skate3-runtime")]
        if let Some(source)=manager.source3.as_mut().filter(|source|source.active) {
            let (_,com,velocity)=motion(processed)?;
            let frames=source.manager.start(super::skate3_footplant_binding::lanes(com),
                super::skate3_footplant_binding::lanes(velocity));
            if frames!=0 {
                self.skaters.as_mut().ok_or(E::Missing("skater pool"))?
                    .try_with_jump_pair(&self.backend,board,skeleton,|normal,_| {
                        normal.disable_foot_collisions(source.manager.selected_toe==Some(19),frames as i32);
                    }).map_err(|e|service("S3 footplant start Skeleton",e))?;
            }
            crate::diagnostics::test_log::event("skate3_footplant_start",format_args!(
                "toe={:?} duration={} native_s2_solver=true",source.manager.selected_toe,source.manager.scalar_596));
            return Ok(());
        }
        let refs = &owner.inputs().cvars;
        let attributes = manager.attributes().clone();
        let fields = &mut manager.fields;
        let numeric = |e| service("StartFootPlantGround numeric", e);
        fields.foot_planting = true;
        fields.time_in_footplant = 0.0;
        fields.landing_next_frame = false;
        let (_, com, velocity) = motion(processed)?;
        let foot_raise = scalar(&attributes, 0x8c1a_73c7_11a5_b513)?;
        let landing = fields
            .landing_position
            .ok_or(E::Missing("FootPlant landing+e0"))?;
        // 0x82dbc770: the splatted scalar is added to all four lanes.
        let ik_foot = landing.map(|v| v + foot_raise);
        fields.ik_foot_position = ik_foot;
        let to_com = com.zip(ik_foot, |c, f| c - f);
        let leg = math::try_normalize_safe(to_com, ZERO, n).map_err(numeric)?;
        let along = n.dot_vector(velocity, leg).map_err(numeric)?;
        let projected = leg.map(|l| l * along);
        let squared = n.dot_vector(projected, projected).map_err(numeric)?;
        let seed = n.reciprocal_sqrt_estimate(squared).map_err(numeric)?;
        let speed = math::length_from_squared_and_estimate(squared, seed);
        let gain = graph(&attributes, 0x8a88_8721_8466_ed15, speed)?;
        let lead_scale = scalar(&attributes, 0xd622_4b71_0a36_d643)?;
        let lag_scale = scalar(&attributes, 0x68c0_5a7f_cc6b_b718)?;
        let cross = math::cross_typed(to_com, velocity);
        let cross_squared = n.dot_vector(cross, cross).map_err(numeric)?;
        let cross_seed = n.reciprocal_sqrt_estimate(cross_squared).map_err(numeric)?;
        let cross_length = math::length_from_squared_and_estimate(cross_squared, cross_seed);
        // 0x82dbc90c: all-lanes length > VECFLOAT_SMALL (2^-16); unordered returns.
        if !(cross_length > f32::from_bits(0x3780_0000)) {
            return Ok(());
        }
        // vrefp then two fused Newton steps (0x82dbc95c..9f0).
        let estimate = n.reciprocal_estimate(cross_length).map_err(numeric)?;
        let inverse = refine_drive_reciprocal(cross_length, estimate);
        let axis = cross.map(|v| inverse * v);
        let rate = cross_length * gain;
        let lead = rate * lead_scale;
        let lag = rate * lag_scale;
        // 0x82dbca1c/ca38/cac8: three XMVectorSinCos results are overwritten
        // before any read; the callee is pure, so omitting them changes no state.
        let floor = scalar(&attributes, 0x4f45_da86_685c_f029)? * DEGREES_TO_RADIANS;
        let theta = plane_angle(leg, V::known([0.0, 1.0, 0.0, 0.0]), axis.map(|v| -v), n)
            .map_err(numeric)?;
        let wrapped = wrap_angle(theta);
        let upper = wrapped + lead;
        let raised = if upper - floor >= 0.0 { upper } else { floor };
        let lower = wrapped + lag;
        let limited = if lower - raised >= 0.0 { raised } else { lower };
        let swing = limited - wrapped;
        fields.total_time = swing / rate;
        let (sin, cos) = try_sin_cos(swing * 0.5, &mut HostReckoningSinCosNumeric)
            .unwrap_or_else(|never| match never {});
        let mut quaternion = axis.map(|v| v * sin);
        quaternion.w = Some(cos);
        let first = math::cross_typed(quaternion, leg);
        let inner = leg.zip(first, |l, c| l.mul_add(cos, c));
        let second = math::cross_typed(quaternion, inner);
        let rotated = second.zip(leg, |s, l| s.mul_add(2.0, l));
        let reach = scalar(&attributes, 0x0de0_027b_9913_c5d5)?;
        let end = rotated.zip(ik_foot, |r, f| r.mul_add(reach, f));
        let bisector =
            math::try_normalize_safe(rotated.zip(leg, |r, l| r + l), ZERO, n).map_err(numeric)?;
        let normal_speed = n.dot_vector(velocity, bisector).map_err(numeric)?;
        let tangential = velocity.zip(bisector.map(|b| b * normal_speed), |v, p| v - p);
        let carried = tangential.map(|t| t * gain);
        let push = scalar(&attributes, 0xc5f9_2168_dfaf_d0b8)?;
        let exit = bisector.zip(carried, |b, c| b.mul_add(push, c));
        fields.exit_velocity = exit;
        fields.bezier[0] = Some(com);
        let entry_scale = scalar(&attributes, 0x5c43_41ef_35f4_b2b0)?;
        let total = fields.total_time;
        fields.bezier[1] = Some(
            velocity
                .map(|v| (v * gain) * total)
                .zip(com, |v, c| v.mul_add(entry_scale, c)),
        );
        let exit_scale = scalar(&attributes, 0x628c_228d_cc7b_6ed5)?;
        let total = fields.total_time;
        fields.bezier[2] = Some(end.zip(exit.map(|x| (x * total) * exit_scale), |p, x| p - x));
        fields.bezier[3] = Some(end);
        if refs
            .try_debug_draw_foot_plant()
            .map_err(|e| service("kDebugDrawFootPlant", e))?
        {
            self.debug_overlay
                .require()
                .map_err(|e| service("FootPlant overlay", e))?;
        }
        // 0x82dbcef8..f10: fmuls by -59.999996, fctiwz, then the integer cvar
        // minus that truncation (wrapping subf).
        let truncated = n
            .truncate_i32(fields.total_time * f32::from_bits(0xc26f_ffff))
            .map_err(numeric)?;
        let frames = refs
            .try_frames_to_disable_feet_collision()
            .map_err(|e| service("kFramesToDisableFeetCollision", e))?
            .wrapping_sub(truncated);
        let right = fields.foot_part == 19;
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.disable_foot_collisions(right, frames);
            })
            .map_err(|e| service("Skeleton pair", e))?;
        Ok(())
    }

    /// Update 0x82d976b0: UpdateFootPlantGround, `Skeleton::UpdateHandPlant`
    /// at its COM, then on take-off GetLaunchInfo with the exit velocity, the
    /// take-off position as trajectory start and 0.1 below it as board start,
    /// before TrajectorySelector Launch and Update.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual pair, ground and source services"
    )]
    pub(crate) fn try_update_pair_foot_plant(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<(), E> {
        if owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .current
            != CurrentPhysicalState::FootPlant
        {
            return Err(E::ForeignOwner);
        }
        state(owner, board, skeleton, |_| Ok(()))?;
        let output = {
            let mut foot = owner.footplant().lock().map_err(|_| E::Poisoned)?;
            let manager = foot
                .try_manager(owner, skeleton)
                .map_err(|e| service("FootPlantManager", e))?;
            self.try_update_foot_plant_ground(owner, board, skeleton, manager, refs)?
        };
        let progress = self
            .try_update_pair_hand_plant_skeleton(
                owner,
                board,
                skeleton,
                output.com_position,
                ground,
                refs,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
            )
            .map_err(|e| service("UpdateHandPlant", e))?;
        if progress.next != SkeletonGeneralContinuation::Complete {
            return Err(E::Missing("complete hand-plant Skeleton"));
        }
        if output.take_off {
            let mut launch = RetainedTrajectoryLaunchInfo::constructed();
            {
                let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
                let trajectory = pool
                    .try_with_ground_control_board(&self.backend, board, |r, _| {
                        r.wallride_trajectory.clone()
                    })
                    .map_err(|e| service("trajectory attributes", e))?;
                let constructor = &pool
                    .skeleton_construction(skeleton)
                    .map_err(|e| service("Skeleton", e))?
                    .ok_or(E::Missing("Skeleton constructor"))?
                    .constructor;
                try_fill_launch_info(constructor, owner, &trajectory, &mut launch)
                    .map_err(|e| service("GetLaunchInfo", e))?;
            }
            let start = output.take_off_position;
            launch.player_jumped = true;
            launch.use_start_override = true;
            launch.trajectory_start_override = start;
            launch.start_velocity = output
                .com_velocity
                .ok_or(E::Missing("take-off COM velocity"))?;
            launch.board_position_override = start.zip(
                V::known([0.0, f32::from_bits(0xbdcc_cccd), 0.0, 0.0]),
                |a, b| a + b,
            );
            self.try_launch_pair_onboard(
                board,
                skeleton,
                &launch,
                ground,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
            )
            .map_err(|e| service("TrajectorySelector.Launch", e))?;
            let _ready = self
                .try_complete_pair_selector(
                    board,
                    skeleton,
                    ground,
                    &mut numeric.reckoning,
                    &mut numeric.sin_cos,
                    policy,
                )
                .map_err(|e| service("TrajectorySelector.Update", e))?;
        }
        Ok(())
    }

    /// UpdateFootPlantGround 0x82dbcf40: advance by `kUpdateTime`, evaluate the
    /// cubic Bezier at time/total, take off once the time passes the total, and
    /// set the planted foot's world IK target at the IK foot with blend 1.
    fn try_update_foot_plant_ground(
        &mut self,
        _owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        manager: &mut FootPlantManager,
        refs: &InputCvarReferences,
    ) -> Result<GroundOutput, E> {
        #[cfg(feature = "skate3-runtime")]
        if let Some(source)=manager.source3.as_mut().filter(|source|source.active) {
            let adjust=*_owner.skate3_footplant_adjust.lock().map_err(|_|E::Poisoned)?;
            let step=source.manager.ground_step(adjust);
            self.skaters.as_mut().ok_or(E::Missing("skater pool"))?
                .try_with_jump_pair(&self.backend,board,skeleton,|normal,_| {
                    normal.try_set_world_foot_ik_target(step.toe==15,V::known(step.anchor),1.)
                }).map_err(|e|service("S3 footplant hold Skeleton",e))?
                .map_err(|e|service("S3 footplant world foot IK",e))?;
            let start=V::known(step.anchor).zip(V::known([0.,0.2,0.,0.]),|a,b|a+b);
            if step.release_velocity.is_some() {
                crate::diagnostics::test_log::event("skate3_footplant_release",format_args!(
                    "toe={} velocity={:?} body_adjust={adjust:?} native_s2_solver=true",step.toe,step.release_velocity));
            }
            return Ok(GroundOutput { com_position:V::known(step.position),
                com_velocity:step.release_velocity.map(V::known),take_off_position:start,
                take_off:step.release_velocity.is_some() });
        }
        let fields = &mut manager.fields;
        fields.time_in_footplant += UPDATE_TIME;
        let t = fields.time_in_footplant / fields.total_time;
        if refs
            .try_debug_draw_foot_plant()
            .map_err(|e| service("kDebugDrawFootPlant", e))?
        {
            self.debug_overlay
                .require()
                .map_err(|e| service("FootPlant overlay", e))?;
        }
        let [p0, p1, p2, p3] = fields.bezier;
        let missing = || E::Missing("FootPlant Bezier point");
        let (p0, p1, p2, p3) = (
            p0.ok_or_else(missing)?,
            p1.ok_or_else(missing)?,
            p2.ok_or_else(missing)?,
            p3.ok_or_else(missing)?,
        );
        let u = 1.0 - t;
        let t3 = (t * t) * t;
        let u3 = (u * u) * u;
        let three = 3.0 * t;
        let b1 = (three * u) * u;
        let b2 = (three * t) * u;
        let position = p1.map(|v| v * b1);
        let position = p0.zip(position, |p, acc| p.mul_add(u3, acc));
        let position = p2.zip(position, |p, acc| p.mul_add(b2, acc));
        let position = p3.zip(position, |p, acc| p.mul_add(t3, acc));
        let mut output = GroundOutput {
            com_position: position,
            com_velocity: None,
            take_off_position: ZERO,
            take_off: false,
        };
        if fields.time_in_footplant > fields.total_time {
            fields.foot_planting = false;
            output.take_off = true;
            let offset = refs
                .try_y_trajectory_offset()
                .map_err(|e| service("kYTrajectoryOffset", e))?;
            output.take_off_position = fields
                .ik_foot_position
                .zip(V::known([0.0, offset, 0.0, 0.0]), |a, b| a + b);
            output.com_velocity = Some(fields.exit_velocity);
        }
        let left = match fields.foot_part {
            15 => true,
            19 => false,
            _ => {
                return Err(E::Missing(
                    "UpdateFootPlantGround foot Part 15/19 (assert 0x2C0)",
                ));
            }
        };
        let target = fields.ik_foot_position;
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.try_set_world_foot_ik_target(left, target, 1.0)
            })
            .map_err(|e| service("Skeleton pair", e))?
            .map_err(|e| service("SetFootIKTargetPosition", e))?;
        Ok(output)
    }

    /// UpdatePostPhysics 0x82d977a8: `Wipeout::CheckForPlantWipeout`, then
    /// CheckForFootPlantWipeouts 0x82d89470 with the state's own attributes.
    pub(crate) fn try_post_pair_foot_plant<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        let attributes = state(owner, board, skeleton, |s| {
            s.attribs.clone().ok_or(E::Missing("FootPlant attributes"))
        })?;
        self.try_check_plant_wipeout(board, skeleton, owner, overlay, numeric)
            .map_err(|e| service("CheckForPlantWipeout", e))?;
        // 0x82d89494..53c: COM velocity against -up, then the length of the
        // remainder by the two-refinement estimate (zero at zero).
        let (up, _, velocity) = motion(&*processed(owner)?)?;
        #[cfg(feature = "skate3-runtime")]
        {
            let mut foot=owner.footplant().lock().map_err(|_|E::Poisoned)?;
            let manager=foot.try_manager(owner,skeleton).map_err(|e|service("FootPlantManager",e))?;
            if let Some(source)=manager.source3.as_ref().filter(|source|source.active) {
                let requests=source.manager.plant_wipeout_requests(
                    super::skate3_footplant_binding::lanes(up),super::skate3_footplant_binding::lanes(velocity));
                for (reason,value) in requests.into_iter().flatten() {
                    owner.wipeout().lock().map_err(|_|E::Poisoned)?
                        .try_wipeout_request_value(reason,value).map_err(|e|service("S3 footplant wipeout",e))?;
                }
                return Ok(());
            }
        }
        let down = up.map(|u| -u);
        let numeric_error = |e| service("CheckForFootPlantWipeouts numeric", e);
        let falling = numeric.dot_vector(velocity, down).map_err(numeric_error)?;
        let across = velocity.zip(down.map(|d| d * falling), |v, d| v - d);
        let squared = numeric.dot_vector(across, across).map_err(numeric_error)?;
        let seed = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(numeric_error)?;
        let sliding = math::length_from_squared_and_estimate(squared, seed);
        let maximum_falling = scalar(&attributes, 0x5243_6d9f_4fe5_6bcb)?;
        let maximum_sliding = scalar(&attributes, 0x149a_5d0d_4768_d2b1)?;
        if falling > maximum_falling {
            // Request6 HeavyLanding keeps the downward speed (Wipeout+1a/+4c).
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_wipeout_request_value(6, falling)
                .map_err(|e| service("HeavyLanding request", e))?;
        }
        if sliding > maximum_sliding {
            // Request23 TooFastOffBoard with value 0 (Wipeout+2b/+90).
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_ground_request(23)
                .map_err(|e| service("TooFastOffBoard request", e))?;
        }
        Ok(())
    }
}
