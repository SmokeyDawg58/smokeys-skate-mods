//! RevertGround102: Enter82d86bd8, Update82da0190, torque82d91390,
//! UpdateSkateboard82d9c0e0. Uses the retained constructor and shared physics.
use super::{
    antiflip_source::AntiFlipRuntime, ground_control_complete::SteeringInput,
    ground_pre_solve::HostGroundPreparationNumeric, manual_effect_source::ManualRuntime,
    pumping_force_source::PumpingForceRuntime,
};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle,
        board_construction::ground_control::Source as PhysicsErrorSource,
    },
    input_cvar_runtime::{
        InputCvarReferences, pumping::PumpingCvars, pumping_forces::PumpingForceCvars,
        reckoning::ReckoningCvarAdapter,
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{RevertGround, StateKind, StatePayload},
    },
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    physics::PointGraph,
    reckoning::grounded_input::{GroundInputRead, GroundScalar},
    reckoning_numeric::{self as math, ReckoningNumeric},
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_force_runtime::{
        antiflip::try_antiflip, entry::manual_effect::ManualEffect,
        pumping_forces::try_pump_with_refinement,
    },
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) struct RevertError(String);
impl std::fmt::Display for RevertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RevertGround: {}", self.0)
    }
}
pub(crate) fn error(e: impl std::fmt::Debug) -> RevertError {
    RevertError(format!("{e:?}"))
}
fn need<T>(v: Option<T>, name: &'static str) -> Result<T, RevertError> {
    v.ok_or_else(|| error(name))
}
const ZERO: V = V::known([0.0; 4]);
fn force() -> SkateboardFootForce {
    SkateboardFootForce {
        world_space_force: ZERO,
        local_space_position: ZERO,
    }
}
pub(crate) fn with_state<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    op: impl FnOnce(&mut RevertGround) -> Result<T, RevertError>,
) -> Result<T, RevertError> {
    if owner.physics_state().lock().map_err(error)?.current != CurrentPhysicalState::RevertGround {
        return Err(error("foreign state102"));
    }
    let mut states = owner.remaining_states().lock().map_err(error)?;
    let Some(StatePayload::RevertGround(s)) = states
        .states
        .get_mut(&StateKind::RevertGround)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(error("retained RevertGround"));
    };
    let b = &s.base.physical;
    if !b.board.same_as(board)
        || !b.skeleton.same_as(skeleton)
        || !b.owner.upgrade().is_some_and(|o| Arc::ptr_eq(&o, owner))
    {
        return Err(error("foreign RevertGround binding"));
    }
    op(s)
}
fn scalar(a: &AttributeRuntimeInstance, key: u64) -> Result<f32, RevertError> {
    a.try_scalar_word(key, 0)
        .map(|v| f32::from_bits(v.unwrap_or(0)))
        .map_err(error)
}
fn graph<const N: usize>(
    a: &AttributeRuntimeInstance,
    x: usize,
    y: usize,
    input: f32,
) -> Result<f32, RevertError> {
    let data = a.try_cached_layout(0x80).map_err(error)?;
    let word = |offset: usize| {
        data.as_ref().map_or(0.0, |d| {
            f32::from_be_bytes(
                d.bytes()[offset..offset + 4]
                    .try_into()
                    .expect("checked revert layout"),
            )
        })
    };
    Ok(PointGraph::<N> {
        x: std::array::from_fn(|i| word(x + i * 4)),
        y: std::array::from_fn(|i| word(y + i * 4)),
    }
    .evaluate(input))
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_enter_pair_revert_ground(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), RevertError> {
        with_state(owner, board, skeleton, |_| Ok(()))?;
        need(self.skaters.as_mut(), "pool")?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.enable_physics_truck_tilt()
            })
            .map_err(error)?;
        self.try_disable_board_animation(board).map_err(error)?;
        with_state(owner, board, skeleton, |s| {
            s.time_in_state = Some(0.0);
            s.has_started = Some(false);
            s.is_reverting = Some(true);
            s.revert_direction = Some(need(
                s.base
                    .processed
                    .lock()
                    .map_err(error)?
                    .physics_state_fields()
                    .start_revert,
                "start revert",
            )?);
            s.vel_start = Some(ZERO);
            Ok(())
        })?;
        owner
            .wipeout()
            .lock()
            .map_err(error)?
            .try_set_state(1)
            .map_err(error)
    }
    pub(crate) fn try_update_pair_revert_ground(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<PhysicalPlayerReckoningPhase, RevertError> {
        let processed = Arc::clone(owner.reckoning().lock().map_err(error)?.processed());
        with_state(owner, board, skeleton, |s| {
            if !need(s.has_started, "has started")? {
                let delay = scalar(
                    need(s.attribs.as_ref(), "revert attributes")?,
                    0x6bf6_5444_75d2_a5ad,
                )?;
                let started = !(need(s.time_in_state, "time")? < delay);
                s.has_started = Some(started);
                if started {
                    let p = processed.lock().map_err(error)?;
                    let v = need(p.prior_output().velocity(), "velocity")?.value;
                    let z = need(p.deck_transform(), "deck")?.columns()[2];
                    let dot = numeric.reckoning.dot_vector(v, z).map_err(error)?;
                    s.direction_scalar = Some(if dot > 0.0 { -1.0 } else { 1.0 });
                    s.vel_start = Some(v);
                }
            }
            Ok(())
        })?;
        let phase = owner
            .reckoning()
            .lock()
            .map_err(error)?
            .try_update_grounded_live(
                &processed,
                &mut ReckoningCvarAdapter(refs),
                &mut numeric.reckoning,
            )
            .map_err(error)?;
        let progress = self
            .try_update_pair_ground_skateboard(
                board,
                skeleton,
                ground,
                refs,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                &mut numeric.spawn_math,
            )
            .map_err(error)?;
        if progress.next!=crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralContinuation::Complete{return Err(error("incomplete Revert Skeleton"));}
        // Pumping.Update uses deck Z against Reckoning up, suppressed during a manual.
        let (up, normal) = {
            let r = owner.reckoning().lock().map_err(error)?;
            let r = need(r.reckoning(), "reckoning")?;
            (r.up(), r.ground_normal())
        };
        let (deck, manual, dt) = {
            let p = processed.lock().map_err(error)?;
            (
                *need(p.deck_transform(), "deck")?,
                need(p.physics_state_fields().balance, "manual")?,
                p.scalar(GroundScalar::TimeDelta).map_err(error)?,
            )
        };
        let mut angle = (math::try_unsigned_angle(deck.columns()[2], up, &mut numeric.reckoning)
            .map_err(error)?
            .abs()
            - std::f32::consts::FRAC_PI_2)
            .abs();
        if manual != 0.0 {
            angle = 0.0;
        }
        let com = need(self.skaters.as_mut(), "pool")?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.current_animation().record().com_to_deck()
            })
            .map_err(error)?;
        owner
            .pumping()
            .lock()
            .map_err(error)?
            .try_update(
                deck.columns()[3],
                normal,
                com,
                angle,
                dt,
                &mut *owner.pumping_attributes().lock().map_err(error)?,
                &mut numeric.reckoning,
                &mut PumpingCvars {
                    references: refs,
                    overlay: &self.debug_overlay,
                },
            )
            .map_err(error)?;
        need(self.skaters.as_mut(), "pool")?
            .try_with_ground_force_board(&self.backend, board, |session, runtime, _| {
                runtime
                    .steering
                    .try_update_physics_error(
                        &mut PhysicsErrorSource {
                            processed: &processed,
                            board: session,
                            backend: &self.backend,
                        },
                        &mut numeric.reckoning,
                    )
                    .map_err(error)
            })
            .map_err(error)??;
        let torque = with_state(owner, board, skeleton, |s| {
            if need(s.has_started, "has started")? {
                revert_torque(s, &mut numeric.reckoning)
            } else {
                Ok(ZERO)
            }
        })?;
        self.try_update_constructed_board_steering(board, 0.0, &mut SteeringInput(&processed))
            .map_err(error)?;
        let overlay = self.debug_overlay.clone();
        let anti = need(self.skaters.as_mut(), "pool")?
            .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                let mut result = ZERO;
                try_antiflip(
                    &mut AntiFlipRuntime {
                        processed: &processed,
                        feet: &runtime.foot_attributes,
                        references: refs,
                        overlay: &overlay,
                    },
                    &mut numeric.reckoning,
                    &mut result,
                )
                .map_err(error)?;
                Ok::<_, RevertError>(result)
            })
            .map_err(error)??;
        let pump = need(self.skaters.as_mut(), "pool")?
            .try_with_ground_force_board(&self.backend, board, |_, runtime, _| {
                let mut result = force();
                try_pump_with_refinement(
                    &mut PumpingForceRuntime {
                        processed: &processed,
                        pumping: &runtime.pumping,
                        attributes: owner.pumping_attributes(),
                        feet: &mut runtime.foot_attributes,
                        debug: &mut PumpingForceCvars {
                            references: refs,
                            overlay: &overlay,
                        },
                    },
                    &mut numeric.reckoning,
                    &mut result,
                    super::ground_ordinary_forces::source_refinement,
                )
                .map_err(error)?;
                Ok::<_, RevertError>(result)
            })
            .map_err(error)??;
        let manual = need(self.skaters.as_mut(), "pool")?
            .try_with_ground_force_board(&self.backend, board, |session, runtime, attributes| {
                let mut result = ManualEffect {
                    angular_displacement: ZERO,
                    corrective_force: force(),
                };
                runtime
                    .entry
                    .try_manual_effect(
                        &mut ManualRuntime {
                            owner,
                            processed: &processed,
                            board: session,
                            backend: &self.backend,
                            attributes,
                            references: refs,
                            overlay: &overlay,
                        },
                        false,
                        &mut numeric.reckoning,
                        &mut numeric.sin_cos,
                        &mut result,
                    )
                    .map_err(error)?;
                Ok::<_, RevertError>(result)
            })
            .map_err(error)??;
        for rotation in [torque, manual.angular_displacement, anti] {
            need(self.skaters.as_mut(), "pool")?
                .try_apply_board_angular_displacement(
                    &mut self.backend,
                    board,
                    rotation,
                    &mut numeric.reckoning,
                )
                .map_err(error)?;
        }
        need(self.skaters.as_mut(), "pool")?
            .try_with_ground_force_board(&self.backend, board, |_, runtime, _| {
                runtime.queue.add(8, pump)
            })
            .map_err(error)?;
        with_state(owner, board, skeleton, |s| {
            let limit = scalar(
                need(s.attribs.as_ref(), "attributes")?,
                0x035d_9805_eb9e_2ee2,
            )?;
            let dt = processed
                .lock()
                .map_err(error)?
                .scalar(GroundScalar::TimeDelta)
                .map_err(error)?;
            let time = need(s.time_in_state, "time")? + dt;
            s.time_in_state = Some(time);
            if time > limit {
                s.is_reverting = Some(false);
            }
            Ok(())
        })?;
        Ok(phase)
    }
}
fn revert_torque<N: ReckoningNumeric>(
    s: &mut RevertGround,
    numeric: &mut N,
) -> Result<V, RevertError>
where
    N::Error: std::fmt::Debug,
{
    let (forward, velocity, normal, omega) = {
        let p = s.base.processed.lock().map_err(error)?;
        (
            need(p.deck_transform(), "deck")?.columns()[2],
            need(s.vel_start, "start velocity")?,
            need(p.prior_output().ground_normal(), "ground normal")?.value,
            need(p.prior_output().angular_velocity(), "omega")?.value,
        )
    };
    let direction_scalar = need(s.direction_scalar, "direction scalar")?;
    let forward = forward.map(|v| v * direction_scalar);
    // Raw VMX82d91458/68 multiply each input by its own dot, not by normal.
    let f = numeric.dot_vector(forward, normal).map_err(error)?;
    let v = numeric.dot_vector(velocity, normal).map_err(error)?;
    let forward = forward.zip(forward.map(|x| x * f), |x, y| x - y);
    let velocity = velocity.zip(velocity.map(|x| x * v), |x, y| x - y);
    let product = numeric.dot_vector(forward, forward).map_err(error)?
        * numeric.dot_vector(velocity, velocity).map_err(error)?;
    if !(product > f32::from_bits(0x3780_0000)) {
        return Ok(ZERO);
    }
    let forward = math::try_normalize_safe(forward, ZERO, numeric).map_err(error)?;
    let velocity = math::try_normalize_safe(velocity, ZERO, numeric).map_err(error)?;
    let mut angle = math::try_signed_angle(velocity, forward, normal, numeric).map_err(error)?;
    angle = math::try_normalize_angle(angle, numeric).map_err(error)?;
    let direction = need(s.revert_direction, "revert direction")?;
    if direction * angle > 0.0 {
        angle = (-direction).mul_add(std::f32::consts::TAU, angle);
    }
    let attributes = need(s.attribs.as_ref(), "revert attributes")?;
    let desired = graph::<8>(attributes, 0x10, 0x30, angle.abs())? * direction;
    let threshold =
        graph::<4>(attributes, 0x60, 0x70, normal.xyz[1])? * f32::from_bits(0x3c8e_fa35);
    if angle.abs() < threshold || angle.abs() > 4.5 {
        s.is_reverting = Some(false);
    }
    let speed = numeric.dot_vector(omega, normal).map_err(error)?;
    let mut torque = (desired - speed) * scalar(attributes, 0x0e5b_939f_d4ec_dbaa)?;
    if speed * desired > 0.0 && speed.abs() > desired.abs() {
        torque = 0.0;
    }
    Ok(normal.map(|v| v * torque))
}
