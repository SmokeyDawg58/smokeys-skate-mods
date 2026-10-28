//! Native KnownAir201 on the already constructed physical-player state owner.
use crate::physics_state_ids::PHYSICS_STATE_PHYSICS_GROUND;
use crate::simulation_assembly_backend::constraint_session::{
    air_update_finish::AirUpdateComplete,
    ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric},
};
use crate::static_ground_pool::StaticGroundPool;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle as Handle,
        board_entry::{try_current_board_deck_velocity, try_current_board_effective_transform},
    },
    input_cvar_runtime::{InputCvarReferences, known_air::KnownAirCvar as C},
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{KnownAir, StateKind, StatePayload},
    },
    reckoning::grounded_input::{GroundInputRead, GroundScalar, GroundVector},
    reckoning_numeric::{ReckoningNumeric, cross_typed, try_normalize_safe},
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as V,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum KnownAirStateError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Service(&'static str, String),
}
impl std::fmt::Display for KnownAirStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KnownAir: {self:?}")
    }
}
type E = KnownAirStateError;
pub(crate) fn service(stage: &'static str, error: impl std::fmt::Debug) -> E {
    E::Service(stage, format!("{error:?}"))
}
pub(crate) fn state<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    f: impl FnOnce(&mut KnownAir) -> Result<T, E>,
) -> Result<T, E> {
    let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
    let Some(StatePayload::KnownAir(s)) = states
        .states
        .get_mut(&StateKind::KnownAir)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(E::Missing("constructed KnownAir201"));
    };
    if !s
        .base
        .physical
        .owner
        .upgrade()
        .is_some_and(|p| Arc::ptr_eq(&p, owner))
    {
        return Err(E::ForeignOwner);
    }
    f(s)
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
fn global_air(owner: &Arc<PhysicalPlayerAssemblyOwner>, key: u64) -> Result<f32, E> {
    owner
        .state_changer()
        .lock()
        .map_err(|_| E::Poisoned)?
        .attributes
        .as_ref()
        .ok_or(E::Missing("global Air attributes"))?
        .scalar(key)
        .map_err(|e| service("Air attribute", e))
}
fn initialize<N: ReckoningNumeric>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    n: &mut N,
) -> Result<(), E>
where
    N::Error: std::fmt::Debug,
{
    use crate::reckoning_attributes::{ReckoningAttributeRead, ReckoningAttributeScope as Scope};
    let mut a = owner
        .reckoning()
        .lock()
        .map_err(|_| E::Poisoned)?
        .reckoning()
        .ok_or(E::Missing("Reckoning"))?
        .cloned_attribute_bindings();
    let pointer = a
        .lookup(Scope::Global, 0x978c_e73b_0efe_76c8)
        .map_err(|e| service("flip lookup", e))?;
    let coefficient = a
        .float(&pointer)
        .map_err(|e| service("flip coefficient", e))?;
    let trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
    let selector = trajectories
        .onboard
        .as_ref()
        .ok_or(E::Missing("selector"))?;
    state(owner, |s| {
        s.try_initialize_trajectory(selector, coefficient, n)
            .map_err(|e| service("InitTrajectoryInfo", e))?;
        #[cfg(feature = "skate3-runtime")]
        if owner.skate3_tuning.lock().map_err(|_| E::Poisoned)?
            .as_ref().is_some_and(|t| t.perfect_body_flips)
        {
            s.body_flip_speed_target = Some(skate3_core::air::known::perfect_body_flip_target(
                s.coll_time.ok_or(E::Missing("collision time"))?));
        }
        Ok(())
    })
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_exit_pair_known_air<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        next: i32,
        n: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_reset_flipping()
            .map_err(|e| service("exit ResetFlipping", e))?;
        self.known_air_dead_trucks_fetch(board)?;
        if owner
            .inputs()
            .cvars
            .try_known_air_bool(C::DrawKnownAir)
            .map_err(|e| service("exit debug", e))?
        {
            self.debug_overlay
                .require()
                .map_err(|e| service("exit overlay", e))?;
        }
        if next == PHYSICS_STATE_PHYSICS_GROUND {
            self.try_restore_known_air_velocity(owner, board, n)?;
        }
        owner
            .trajectories()
            .lock()
            .map_err(|_| E::Poisoned)?
            .onboard
            .as_mut()
            .ok_or(E::Missing("selector"))?
            .reset_after_teleport();
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_reset_body_spin()
            .map_err(|e| service("exit ResetBodySpin", e))?;
        let mut m = owner.grind_air().lock().map_err(|_| E::Poisoned)?;
        let m = m.as_mut().ok_or(E::Missing("GrindAirAdjust"))?;
        m.started = false;
        m.activated = false;
        Ok(())
    }
    fn try_restore_known_air_velocity<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        n: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        let trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
        let selector = trajectories
            .onboard
            .as_ref()
            .ok_or(E::Missing("selector"))?;
        let index = state(owner, |s| {
            let index = s.traj_index.ok_or(E::Missing("index"))?;
            let index = if index == 0 { 1 } else { index };
            s.traj_index = Some(index);
            Ok(index)
        })?;
        let velocity = crate::physical_player_trajectory_owners::offboard_runtime::velocity_at(
            &selector.winning_buffer.trajectory,
            index as f32 * f32::from_bits(0x3c888889),
        );
        let p = processed(owner)?;
        let normal = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .ground_normal()
            .ok_or(E::Missing("GroundNormal1d0"))?
            .value;
        let perpendicular = n
            .dot_vector(velocity, normal)
            .map_err(|e| service("landing dot", e))?;
        let tangent = velocity.zip(normal.map(|v| v * perpendicular), |a, b| a - b);
        let tangent_direction = try_normalize_safe(tangent, [0.0; 4], n)
            .map_err(|e| service("landing direction", e))?;
        let downslope = cross_typed(normal, cross_typed(normal, V::known([0.0, 1.0, 0.0, 0.0])));
        let downslope = try_normalize_safe(downslope, [0.0; 4], n)
            .map_err(|e| service("downslope direction", e))?;
        let dot = n
            .dot_vector(downslope, tangent_direction)
            .map_err(|e| service("downslope dot", e))?;
        let positive = if -dot >= 0.0 { 0.0 } else { dot };
        let graph_value = state(owner, |s| {
            let layout = s
                .attributes
                .as_ref()
                .ok_or(E::Missing("KnownAir attributes"))?
                .try_cached_layout(0x130)
                .map_err(|e| service("landing graph", e))?
                .ok_or(E::Missing("Air cached layout"))?;
            let word = |offset: usize| {
                f32::from_be_bytes(layout.bytes()[offset..offset + 4].try_into().unwrap())
            };
            Ok(crate::physics::PointGraph::<8> {
                x: std::array::from_fn(|i| word(0xb0 + i * 4)),
                y: std::array::from_fn(|i| word(0xd0 + i * 4)),
            }
            .evaluate(normal.xyz[1]))
        })?;
        let blend = if 1.0 - positive >= 0.0 { positive } else { 1.0 };
        let scaled = tangent.map(|v| v * (graph_value - 1.0).mul_add(blend, 1.0));
        if owner
            .inputs()
            .cvars
            .try_known_air_bool(C::DrawLandingInfo)
            .map_err(|e| service("landing debug", e))?
        {
            self.debug_overlay
                .require()
                .map_err(|e| service("landing overlay", e))?;
        }
        let actual = try_current_board_deck_velocity(
            self.board(board).map_err(|e| service("landing board", e))?,
            &self.backend,
        )
        .map_err(|e| service("landing velocity", e))?;
        let normal_speed = n
            .dot_vector(actual, normal)
            .map_err(|e| service("normal speed", e))?;
        let next = normal.zip(scaled, |a, b| a.mul_add(normal_speed, b));
        self.try_set_board_velocity(board, &next)
            .map_err(|e| service("SetVelocity", e))?;
        let transform = try_current_board_effective_transform(
            self.board(board)
                .map_err(|e| service("effective board", e))?,
            &self.backend,
            owner,
        )
        .map_err(|e| service("effective transform", e))?;
        let speed = n
            .dot_vector(next, transform.columns()[2])
            .map_err(|e| service("ZSpeed", e))?;
        p.lock()
            .map_err(|_| E::Poisoned)?
            .store_known_air_restored_speed(speed)
            .map_err(E::Missing)?;
        Ok(())
    }
    pub(crate) fn try_fill_pair_known_air<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        n: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        let trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
        let selector = trajectories
            .onboard
            .as_ref()
            .ok_or(E::Missing("selector"))?;
        let r = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
        let orientation = r.reckoning().ok_or(E::Missing("Reckoning"))?.transform();
        state(owner, |s| {
            owner
                .phys_out()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_fill_known_air_state(s, selector, orientation, &owner.inputs().cvars, n)
                .map_err(|e| service("FillPhysOut", e))
        })
    }
    pub(crate) fn try_post_pair_known_air<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        n: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        if !state(owner, |s| s.reached_apex.ok_or(E::Missing("apex")))? {
            let velocity = try_current_board_deck_velocity(
                self.board(board).map_err(|e| service("post board", e))?,
                &self.backend,
            )
            .map_err(|e| service("post velocity", e))?;
            if 0.0 > velocity.xyz[1] {
                state(owner, |s| {
                    s.reached_apex = Some(true);
                    Ok(())
                })?;
            }
        }
        let overlay = self.debug_overlay.clone();
        self.try_check_air_wipeout(board, skeleton, owner, &overlay, n, true)
            .map_err(|e| service("CheckForAirWipeout(true)", e))?;
        let p = processed(owner)?;
        if !p
            .lock()
            .map_err(|_| E::Poisoned)?
            .flag8c4(0x20)
            .map_err(|e| service("upside-down flag", e))?
        {
            use crate::skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead};
            let mut read = |key| -> Result<f32, E> {
                self.skaters
                    .as_mut()
                    .ok_or(E::Missing("pool"))?
                    .try_with_ground_force_board(&self.backend, board, |_, _, a| {
                        a.read_float(SkateboardAttributeClass::Wipeout, key)
                    })
                    .map_err(|e| service("wipeout attributes", e))?
                    .map_err(|e| service("wipeout attribute", e))
            };
            let threshold = read(0x7efc_66f9_5cfa_e01e)?;
            let up = p
                .lock()
                .map_err(|_| E::Poisoned)?
                .prior_output()
                .reckoning_up()
                .ok_or(E::Missing("up220"))?
                .value;
            if threshold > up.xyz[1] {
                let threshold = read(0x979f_d07b_fb01_21e4)?;
                let normal = state(owner, |s| {
                    s.landing_normal.ok_or(E::Missing("landing normal"))
                })?;
                if n.dot_vector(normal, up)
                    .map_err(|e| service("upside-down dot", e))?
                    < threshold
                {
                    owner
                        .wipeout()
                        .lock()
                        .map_err(|_| E::Poisoned)?
                        .try_ground_request(15)
                        .map_err(|e| service("upside-down request", e))?;
                }
            }
        }
        if owner
            .inputs()
            .cvars
            .try_known_air_bool(C::DrawUpsideWipeout)
            .map_err(|e| service("upside-down debug", e))?
        {
            self.debug_overlay
                .require()
                .map_err(|e| service("upside-down overlay", e))?;
        }
        Ok(())
    }
    fn known_air_dead_trucks_fetch(&mut self, board: &Handle) -> Result<(), E> {
        use crate::skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead};
        self.skaters
            .as_mut()
            .ok_or(E::Missing("pool"))?
            .try_with_ground_force_board(&self.backend, board, |_, _, a| {
                a.lookup(SkateboardAttributeClass::Trucks, 0x2be0_8dd7_a920_7622)
            })
            .map_err(|e| service("trucks owner", e))?
            .map_err(|e| service("trucks lookup", e))?;
        Ok(())
    }
    pub(crate) fn try_update_pair_known_air(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        ground: &StaticGroundPool,
        numeric: &mut HostGroundPreparationNumeric,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<AirUpdateComplete, E> {
        let refs = &owner.inputs().cvars;
        let p = processed(owner)?;
        // 0x82d9983c..874: `ble` resets unless the timer is ordered-greater
        // than kMinAirTimeToFootPlant, so an unordered timer also resets.
        let reset = {
            let p = p.lock().map_err(|_| E::Poisoned)?;
            let fields = p.physics_state_fields();
            fields.flags_8d0.ok_or(E::Missing("flags8d0"))? & 0x0800_0000 == 0
                || fields.flags_8cc.ok_or(E::Missing("flags8cc"))? & 0x1000_0000 != 0
                || {
                    let timer=fields.state_timer.ok_or(E::Missing("state timer"))?;
                    #[cfg(feature = "skate3-runtime")]
                    let source=owner.skate3_footplants.load(std::sync::atomic::Ordering::Relaxed);
                    #[cfg(not(feature = "skate3-runtime"))]
                    let source=false;
                    if source { timer <= f32::from_bits(0x3dcc_cccd) } else {
                        !(timer > refs.try_known_air_float(C::MinFootPlantTime)
                            .map_err(|e|service("footplant cvar",e))?)
                    }
                }
        };
        if reset {
            owner
                .footplant()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_reset(owner, skeleton)
                .map_err(|e| service("FootPlant.Reset", e))?;
        } else {
            self.try_update_pair_footplant_air(owner, board, skeleton, ground, numeric, policy)
                .map_err(|e| service("FootPlant.UpdateFootplantAir", e))?;
        }
        let changed = owner
            .trajectories()
            .lock()
            .map_err(|_| E::Poisoned)?
            .onboard
            .as_ref()
            .ok_or(E::Missing("selector"))?
            .has_just_changed;
        if changed {
            initialize(owner, &mut numeric.reckoning)?;
        }
        self.try_update_pair_air_collision(board, skeleton)
            .map_err(|e| service("air collision", e))?;
        #[cfg(feature = "skate3-runtime")]
        let perfect_flips = owner.skate3_tuning.lock().map_err(|_| E::Poisoned)?
            .as_ref().is_some_and(|t| t.perfect_body_flips);
        #[cfg(not(feature = "skate3-runtime"))]
        let perfect_flips = false;
        let flip = state(owner, |s| {
            let p = p.lock().map_err(|_| E::Poisoned)?;
            let grabbing = p.flag8c4(0x30).map_err(|e| service("grab", e))?;
            if !s.body_flipping.ok_or(E::Missing("body flipping"))?
                && grabbing
                && p.flag8c4(0xc0).map_err(|e| service("flip intent", e))?
                && s.coll_time.ok_or(E::Missing("collision time"))?
                    > s.scalar::<std::convert::Infallible>(0x7999_6a9a_f37c_fd74)
                        .map_err(|e| service("flip limit", e))?
            {
                let forward = p.flag8c4(0x80).map_err(|e| service("front flip", e))?;
                drop(p);
                owner
                    .reckoning()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .try_start_flipping(forward, &mut numeric.reckoning)
                    .map_err(|e| service("StartFlipping", e))?;
                s.body_flipping = Some(true);
                // Source rereads the Processed flags after StartFlipping.
                let p = processed(owner)?;
                let grabbing = p
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .flag8c4(0x30)
                    .map_err(|e| service("grab reread", e))?;
                return Ok(
                    if perfect_flips || grabbing
                        || s.time_in_state.ok_or(E::Missing("state time"))?
                            < s.scalar::<std::convert::Infallible>(0x1823_8d24_2ece_aaf1)
                                .map_err(|e| service("flip cutoff", e))?
                    {
                        s.body_flip_speed_target.ok_or(E::Missing("flip target"))?
                    } else {
                        0.0
                    },
                );
            }
            Ok(
                if perfect_flips || grabbing
                    || s.time_in_state.ok_or(E::Missing("state time"))?
                        < s.scalar::<std::convert::Infallible>(0x1823_8d24_2ece_aaf1)
                            .map_err(|e| service("flip cutoff", e))?
                {
                    s.body_flip_speed_target.ok_or(E::Missing("flip target"))?
                } else {
                    0.0
                },
            )
        })?;
        let spin = state(owner, |s| {
            let r = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
            let r = r.reckoning().ok_or(E::Missing("Reckoning"))?;
            s.try_body_spin(
                &*p.lock().map_err(|_| E::Poisoned)?,
                r.heading(),
                r.up(),
                r.body_spin().0,
                refs,
                &mut numeric.reckoning,
            )
            .map_err(|e| service("spin target", e))
        })?;
        let (normal, time, collision) = state(owner, |s| {
            Ok((
                s.landing_normal.ok_or(E::Missing("normal"))?,
                s.time_in_state.ok_or(E::Missing("time"))?,
                s.coll_time.ok_or(E::Missing("collision time"))?,
            ))
        })?;
        let dt = p
            .lock()
            .map_err(|_| E::Poisoned)?
            .scalar(GroundScalar::TimeDelta)
            .map_err(|e| service("dt", e))?;
        let frames = (collision - time) / dt
            - refs
                .try_known_air_float(C::AlignFrames)
                .map_err(|e| service("align frames", e))?;
        let frames = if frames - 2.0 >= 0.0 { frames } else { 2.0 };
        let threshold = global_air(owner, 0x9d86_b4c6_ce67_7cff)?;
        let adjusting = state(owner, |s| {
            if !s
                .grind_air_adjust_activated
                .ok_or(E::Missing("grind activation"))?
                && (s.reached_apex.ok_or(E::Missing("apex"))? || frames < threshold)
            {
                let mut m = owner.grind_air().lock().map_err(|_| E::Poisoned)?;
                let m = m.as_mut().ok_or(E::Missing("GrindAirAdjust"))?;
                m.activated = m.started;
                s.grind_air_adjust_activated = Some(true);
            }
            if s.targeting_grind.ok_or(E::Missing("targeting grind"))?
                && s.grind_air_adjust_activated
                    .ok_or(E::Missing("grind activation"))?
            {
                let m = owner.grind_air().lock().map_err(|_| E::Poisoned)?;
                return Ok(m.as_ref().ok_or(E::Missing("GrindAirAdjust"))?.adjusting);
            }
            Ok(false)
        })?;
        if adjusting {
            // `DisableLeftFootCollisions(2)` then `DisableRightFootCollisions(2)`.
            self.skaters
                .as_mut()
                .ok_or(E::Missing("pool"))?
                .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                    normal.disable_foot_collisions(false, 2);
                    normal.disable_foot_collisions(true, 2);
                })
                .map_err(|e| service("grind-air feet", e))?;
        }
        let reckoning = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_update_air_live(
                &p,
                normal,
                1.0 / frames,
                spin,
                flip,
                &mut crate::input_cvar_runtime::reckoning::ReckoningCvarAdapter(refs),
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                #[cfg(feature = "skate3-runtime")]
                owner.skate3_tuning.lock().map_err(|_| E::Poisoned)?.as_deref(),
            )
            .map_err(|e| service("Reckoning.UpdateAirStates", e))?;
        let correction = global_air(owner, 0x1191_b5b0_c272_4ab9)?;
        let target = {
            let trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
            let selector = trajectories
                .onboard
                .as_ref()
                .ok_or(E::Missing("selector"))?;
            state(owner, |s| {
                s.try_follow_trajectory::<std::convert::Infallible>(selector, correction)
                    .map_err(|e| service("trajectory follow", e))?;
                s.target_com_pos.ok_or(E::Missing("target COM"))
            })?
        };
        let progress = self
            .try_update_pair_known_air_skeleton(
                board,
                skeleton,
                Some(target),
                ground,
                refs,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                &mut numeric.spawn_math,
            )
            .map_err(|e| service("Skeleton.UpdateKnownAir", e))?;
        if progress.next != crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralContinuation::Complete { return Err(E::Missing("Skeleton continuation")); }
        if time < 0.06 {
            self.skaters
                .as_mut()
                .ok_or(E::Missing("pool"))?
                .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                    normal.mark_add_skateboard_error()
                })
                .map_err(|e| service("board error", e))?;
        }
        let target = if p
            .lock()
            .map_err(|_| E::Poisoned)?
            .flag8c8(0x80_0000)
            .map_err(|e| service("head flag", e))?
        {
            p.lock()
                .map_err(|_| E::Poisoned)?
                .deck_transform()
                .ok_or(E::Missing("deck transform"))?
                .columns()[3]
        } else {
            state(owner, |s| s.coll_pos.ok_or(E::Missing("collision pos")))?
        };
        self.skaters
            .as_mut()
            .ok_or(E::Missing("pool"))?
            .try_set_skeleton_head_target(&self.backend, skeleton, owner, target)
            .map_err(|e| service("head target", e))?;
        self.try_update_constructed_board_steering(board,0.0,&mut crate::simulation_assembly_backend::constraint_session::ground_control_complete::SteeringInput(&p)).map_err(|e| service("steering tilt",e))?;
        let height = self.known_air_height(board)?;
        state(owner, |s| {
            let old = s.max_y.ok_or(E::Missing("max height"))?;
            s.max_y = Some(if old > height { old } else { height });
            s.time_in_state = Some(dt + time);
            Ok(())
        })?;
        let com = self.known_air_com(owner, board, skeleton)?;
        state(owner, |s| {
            let old = s.com_max_y.ok_or(E::Missing("COM max height"))?;
            s.com_max_y = Some(if old - com.xyz[1] >= 0.0 {
                old
            } else {
                com.xyz[1]
            });
            Ok(())
        })?;
        self.known_air_dead_trucks_fetch(board)?;
        if refs
            .try_known_air_bool(C::DrawKnownAir)
            .map_err(|e| service("debug cvar", e))?
        {
            self.debug_overlay
                .require()
                .map_err(|e| service("debug overlay", e))?;
        }
        Ok(AirUpdateComplete {
            owner: Arc::clone(owner),
            reckoning,
        })
    }
    fn known_air_com(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
    ) -> Result<V, E> {
        self.skaters
            .as_mut()
            .ok_or(E::Missing("pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, actual| {
                if !Arc::ptr_eq(actual, owner) {
                    return Err(E::ForeignOwner);
                }
                Ok(normal.post_physics_records().centre_of_mass_typed())
            })
            .map_err(|e| service("COM pair", e))?
    }
    fn known_air_height(&self, board: &Handle) -> Result<f32, E> {
        Ok(self
            .skaters
            .as_ref()
            .ok_or(E::Missing("pool"))?
            .session(board)
            .map_err(|e| service("board session", e))?
            .try_live_part_transform_typed(&self.backend, 6)
            .map_err(|e| service("board transform", e))?
            .columns()[3]
            .xyz[1])
    }
    pub(crate) fn try_enter_pair_known_air<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
        n: &mut N,
    ) -> Result<(), E>
    where
        N::Error: std::fmt::Debug,
    {
        let p = processed(owner)?;
        state(owner, |s| {
            if !s.base.physical.board.same_as(board)
                || !s.base.physical.skeleton.same_as(skeleton)
                || !Arc::ptr_eq(&p, &s.base.processed)
            {
                return Err(E::ForeignOwner);
            }
            Ok(())
        })?;
        self.try_enable_board_angular_drive_only(board, 1.0, refs, n)
            .map_err(|e| service("angular drive", e))?;
        self.try_pair_skeleton_collision_transition(board,skeleton,crate::simulation_assembly_backend::constraint_session::physical_skeleton_controller::SkeletonCollisionTransition::InitAir).map_err(|e| service("InitAirCollision",e))?;
        self.skaters
            .as_mut()
            .ok_or(E::Missing("pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.try_known_air_entry_ik_and_tilt()
            })
            .map_err(|e| service("IK pair", e))?
            .map_err(|e| service("IK", e))?;
        state(owner, |s| {
            s.body_flipping = Some(false);
            s.grind_air_adjust_activated = Some(false);
            s.targeting_grind = Some(false);
            Ok(())
        })?;
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_reset_flipping()
            .map_err(|e| service("ResetFlipping", e))?;
        owner
            .footplant()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_full_reset(owner, skeleton)
            .map_err(|e| service("FullReset", e))?;
        state(owner, |s| {
            s.reached_apex = Some(false);
            s.time_in_state = Some(0.0);
            s.start_y = Some(
                p.lock()
                    .map_err(|_| E::Poisoned)?
                    .ground_position_fields()
                    .ground_position
                    .ok_or(E::Missing("GroundPos"))?
                    .xyz[1],
            );
            Ok(())
        })?;
        let height = self.known_air_height(board)?;
        state(owner, |s| {
            s.max_y = Some(height);
            Ok(())
        })?;
        let com = self.known_air_com(owner, board, skeleton)?;
        state(owner, |s| {
            s.com_max_y = Some(com.xyz[1]);
            Ok(())
        })?;
        initialize(owner, n)?;
        let trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
        let selector = trajectories
            .onboard
            .as_ref()
            .ok_or(E::Missing("selector"))?;
        if selector.targeting_grind {
            // `TrajectorySelector::GetTargetGrind` and the possible orientations
            // into `GrindAirAdjustIn`, then `GrindAirAdjust::Start`.
            let orientations = selector
                .possible_orientations
                .as_deref()
                .ok_or(E::Missing("possible grind orientations"))?;
            let direction =
                |value: Option<[f32; 4]>| value.ok_or(E::Missing("grind orientation direction"));
            let input = crate::physical_player_grind_air_adjust::update::GrindAirAdjustIn {
                start: selector.grind_primitive.start,
                end: selector.grind_primitive.end,
                vertical_bit: selector.grind_primitive.flag_word,
                geometry_type: orientations
                    .geometry_type
                    .ok_or(E::Missing("grind geometry type"))?,
                is_garbage: orientations
                    .is_garbage
                    .ok_or(E::Missing("grind garbage flag"))?,
                directions: [
                    direction(orientations.boardslide_direction)?,
                    direction(orientations.tipslide_direction)?,
                    direction(orientations.backslash_direction)?,
                    direction(orientations.high_side_direction)?,
                ],
            };
            owner
                .grind_air()
                .lock()
                .map_err(|_| E::Poisoned)?
                .as_mut()
                .ok_or(E::Missing("GrindAirAdjust"))?
                .try_start(
                    &input,
                    &owner.inputs().vault,
                    &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
                )
                .map_err(|e| service("GrindAirAdjust.Start", e))?;
            state(owner, |s| {
                s.targeting_grind = Some(true);
                Ok(())
            })?;
        }
        let (index, error) = selector
            .try_closest_com_index(com, n)
            .map_err(|e| service("closest COM", e))?;
        state(owner, |s| {
            s.traj_index = Some(index);
            s.trajectory_error = Some(error);
            Ok(())
        })?;
        let p = p.lock().map_err(|_| E::Poisoned)?;
        let forward = p
            .vector(GroundVector::DeckZ)
            .map_err(|e| service("DeckZ", e))?;
        let velocity = p
            .prior_output()
            .velocity()
            .ok_or(E::Missing("velocity190"))?
            .value;
        let flipped = 0.0
            > n.dot_vector(forward, velocity)
                .map_err(|e| service("entry direction", e))?;
        state(owner, |s| {
            s.start_flipped = Some(flipped);
            Ok(())
        })
    }
}
