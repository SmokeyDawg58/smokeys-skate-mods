//! PhysState_SlideGround101: Enter82d84c68, Exit82d84d48, Update82d9afa0,
//! UpdateSkateboard82d8e2a8, CalcPowerSlideForce82b27ff8. Real retained owners.
use crate::physics_state_ids::PHYSICS_CATEGORY_GROUND;
use super::{
    antiflip_source::AntiFlipRuntime,
    ground_control_complete::SteeringInput,
    ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric},
    ground_wallride::source::WallRideRuntime,
    manual_effect_source::ManualRuntime,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle,
        board_construction::ground_control::Source as PhysicsErrorSource,
    },
    input_cvar_runtime::{
        InputCvarReferences, ground_foot_force::GroundFootCvars, reckoning::ReckoningCvarAdapter,
        wallride::WallRideCvars,
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{StateKind, StatePayload},
    },
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    reckoning_numeric::{ReckoningNumeric, cross_typed, try_normalize_unsafe},
    skateboard_attributes::{SkateboardAttributeClass as Class, SkateboardAttributeRead},
    skateboard_foot_forces::{SkateboardFootForce, SkateboardFootForceServices},
    skateboard_force_runtime::{antiflip::try_antiflip, entry::manual_effect::ManualEffect},
    skateboard_slide_collision::SkateboardSlideCollisionServices,
    skateboard_slide_collision_attributes::SlideSurfaceField,
    skateboard_steering::calculate_attributes::SteeringCalculateVault,
    skeleton_processed_contact::{SkeletonProcessedContact, ground_force_input::GroundForceInput},
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) struct SlideError(String);
impl std::fmt::Display for SlideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SlideGround: {}", self.0)
    }
}
fn error(e: impl std::fmt::Debug) -> SlideError {
    SlideError(format!("{e:?}"))
}
fn need<T>(v: Option<T>, field: &'static str) -> Result<T, SlideError> {
    v.ok_or_else(|| error(field))
}
const ZERO: V = V::known([0.0; 4]);
fn force() -> SkateboardFootForce {
    SkateboardFootForce {
        world_space_force: ZERO,
        local_space_position: ZERO,
    }
}

fn with_state<T>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    operation: impl FnOnce(
        &mut crate::physical_player_assembly::remaining_states::SlideGround,
    ) -> Result<T, SlideError>,
) -> Result<T, SlideError> {
    if owner.physics_state().lock().map_err(error)?.current != CurrentPhysicalState::SlideGround {
        return Err(error("foreign state101"));
    }
    let mut states = owner.remaining_states().lock().map_err(error)?;
    let Some(StatePayload::SlideGround(state)) = states
        .states
        .get_mut(&StateKind::SlideGround)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(error("retained SlideGround"));
    };
    let b = &state.base.physical;
    if !b.board.same_as(board)
        || !b.skeleton.same_as(skeleton)
        || !b.owner.upgrade().is_some_and(|o| Arc::ptr_eq(&o, owner))
    {
        return Err(error("foreign SlideGround binding"));
    }
    operation(state)
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_enter_pair_slide_ground(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SlideError> {
        with_state(owner, board, skeleton, |_| Ok(()))?;
        need(self.skaters.as_mut(), "skater pool")?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.enable_physics_truck_tilt()
            })
            .map_err(error)?;
        owner
            .reckoning()
            .lock()
            .map_err(error)?
            .try_reset_body_spin()
            .map_err(error)?;
        self.try_disable_board_animation(board).map_err(error)?;
        // Native masks XYZ only; the backend setter preserves its existing W.
        let body = self
            .board(board)
            .map_err(error)?
            .try_part_contact_spy_observation(&self.backend, 6)
            .map_err(error)?;
        self.backend
            .try_set_part_angular_velocity(body, [0.0; 3])
            .map_err(error)?;
        self.try_set_board_linear_drag(board, 0.0).map_err(error)?;
        let processed = Arc::clone(owner.reckoning().lock().map_err(error)?.processed());
        if need(
            processed
                .lock()
                .map_err(error)?
                .physics_state_fields()
                .category,
            "physics category",
        )? != PHYSICS_CATEGORY_GROUND
        {
            self.try_reset_board_manual(board).map_err(error)?;
        }
        owner
            .wipeout()
            .lock()
            .map_err(error)?
            .try_set_state(1)
            .map_err(error)?;
        let speed = need(
            processed
                .lock()
                .map_err(error)?
                .prior_output()
                .surface_speed(),
            "surface speed",
        )?
        .value;
        with_state(owner, board, skeleton, |s| {
            s.board_is_flipped = Some(false);
            s.target_speed = Some(speed);
            s.push_scalar = Some(1.0);
            s.damped_turn = Some(0.0);
            Ok(())
        })
    }
    pub(crate) fn try_exit_pair_slide_ground(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SlideError> {
        with_state(owner, board, skeleton, |s| {
            s.push_scalar = Some(1.0);
            s.damped_turn = Some(0.0);
            Ok(())
        })?;
        self.try_restore_board_wheel_material(board).map_err(error)
    }
    #[expect(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(crate) fn try_update_pair_slide_ground(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
        policy: GroundPreparationGrindPolicy,
    ) -> Result<PhysicalPlayerReckoningPhase, SlideError> {
        with_state(owner, board, skeleton, |_| Ok(()))?;
        let processed = Arc::clone(owner.reckoning().lock().map_err(error)?.processed());
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
        if progress.next != crate::dynamic_assembly_session::skeleton_normal::general_owner::SkeletonGeneralContinuation::Complete { return Err(error("incomplete SlideGround Skeleton")); }

        // UpdatePhysicsError -> material -> damping -> steering, no Ground wobble.
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
        self.try_set_board_wheel_material(board, || {
            processed
                .lock()
                .map_err(|_| "processed lock")?
                .selected_surface()
                .and_then(|s| s.material.clone())
                .ok_or("selected wheel material")
        })
        .map_err(error)?;
        let damping = need(
            processed.lock().map_err(error)?.prior_output().z_speed(),
            "damping speed",
        )?
        .value;
        self.try_with_constructed_board_truck_drives(board, |drives, _| {
            drives.set_speed_damping_factor(damping)
        })
        .map_err(error)?;
        let tilt = with_state(owner, board, skeleton, |s| {
            let input = processed.lock().map_err(error)?;
            need(self.skaters.as_mut(), "pool")?
                .try_with_ground_control_board(&self.backend, board, |runtime, manual| {
                    crate::skateboard_steering::calculate::try_calculate(
                        &*input,
                        &mut SteeringCalculateVault {
                            steering: &mut runtime.steering_attributes,
                            manual,
                        },
                        s.push_scalar.as_mut(),
                        s.damped_turn.as_mut(),
                    )
                    .map_err(error)
                })
                .map_err(error)?
        })?;
        self.try_update_constructed_board_steering(board, tilt, &mut SteeringInput(&processed))
            .map_err(error)?;
        let overlay = self.debug_overlay.clone();
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
                        true,
                        &mut numeric.reckoning,
                        &mut numeric.sin_cos,
                        &mut result,
                    )
                    .map_err(error)?;
                Ok::<_, SlideError>(result)
            })
            .map_err(error)??;
        let mut wall = force();
        let mut jumping = false;
        let mut height = 0.0;
        let mut velocity = ZERO;
        let ground_time = need(
            processed
                .lock()
                .map_err(error)?
                .input_controls()
                .time_on_ground,
            "time on ground",
        )?;
        need(self.skaters.as_mut(), "pool")?
            .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                crate::skateboard_force_runtime::wallride::try_calculate(
                    &mut WallRideRuntime {
                        processed: &processed,
                        jump_flag: &mut jumping,
                        jump_height: &mut height,
                        jump_velocity: &mut velocity,
                        feet: &mut runtime.foot_attributes,
                        trajectory: &runtime.wallride_trajectory,
                        queries: self.world_line_queries.as_mut(),
                        dmos: self.dmos.as_ref(),
                        static_ground: ground,
                        debug: &mut WallRideCvars {
                            references: refs,
                            overlay: &overlay,
                        },
                    },
                    &mut numeric.reckoning,
                    ground_time,
                    0.0,
                    &mut wall,
                )
                .map_err(error)
            })
            .map_err(error)??;
        if jumping {
            super::ground_jump_runtime::launch_velocity(
                self, owner, board, skeleton, velocity, ground, numeric, policy, false,
            )
            .map_err(error)?;
            return Ok(phase);
        }
        let anti = need(self.skaters.as_mut(), "pool")?
            .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                let mut v = ZERO;
                try_antiflip(
                    &mut AntiFlipRuntime {
                        processed: &processed,
                        feet: &runtime.foot_attributes,
                        references: refs,
                        overlay: &overlay,
                    },
                    &mut numeric.reckoning,
                    &mut v,
                )
                .map_err(error)?;
                Ok::<_, SlideError>(v)
            })
            .map_err(error)??;
        let rotation = self.slide_ground_orientation(board, &processed, numeric)?;
        need(self.skaters.as_mut(), "pool")?
            .try_apply_board_angular_displacement(
                &mut self.backend,
                board,
                rotation,
                &mut numeric.reckoning,
            )
            .map_err(error)?;
        let manual_scalar = need(self.skaters.as_mut(), "pool")?
            .try_with_ground_control_board(&self.backend, board, |_, a| {
                a.read_float(Class::Manual, 0xcc65_f651_4721_1d65)
            })
            .map_err(error)?
            .map_err(error)?;
        need(self.skaters.as_mut(), "pool")?
            .try_apply_board_angular_displacement(
                &mut self.backend,
                board,
                manual.angular_displacement.map(|v| v * manual_scalar),
                &mut numeric.reckoning,
            )
            .map_err(error)?;
        overlay.require().map_err(error)?;
        if refs.try_apply_antiflip_slide().map_err(error)? {
            need(self.skaters.as_mut(), "pool")?
                .try_apply_board_angular_displacement(
                    &mut self.backend,
                    board,
                    anti,
                    &mut numeric.reckoning,
                )
                .map_err(error)?;
            overlay.require().map_err(error)?;
        }
        // CalcCollisionForce replaces this state's four ordinary forces when true.
        need(self.skaters.as_mut(), "pool")?
            .try_with_ground_force_board(&self.backend, board, |_, runtime, attributes| {
                let reckoning = owner.reckoning().lock().map_err(error)?;
                let mut input = GroundForceInput {
                    processed: &processed,
                    reckoning: reckoning.reckoning(),
                };
                let mut front = force();
                let mut rear = force();
                let mut debug = GroundFootCvars {
                    references: refs,
                    overlay: &overlay,
                };
                SkateboardFootForceServices {
                    input: &mut input,
                    attributes: &mut runtime.foot_attributes,
                    debug: &mut debug,
                    numeric: &mut numeric.reckoning,
                }
                .try_front_with_refinement(
                    ground_time,
                    0.0,
                    &mut front,
                    super::ground_ordinary_forces::source_refinement,
                )
                .map_err(error)?;
                let ground_time = need(
                    processed
                        .lock()
                        .map_err(error)?
                        .input_controls()
                        .time_on_ground,
                    "rear ground time",
                )?;
                SkateboardFootForceServices {
                    input: &mut input,
                    attributes: &mut runtime.foot_attributes,
                    debug: &mut debug,
                    numeric: &mut numeric.reckoning,
                }
                .try_rear_with_refinement(
                    ground_time,
                    0.0,
                    &mut rear,
                    super::ground_ordinary_forces::source_refinement,
                )
                .map_err(error)?;
                let slide = power_slide_force(&processed, attributes, &mut numeric.reckoning)?;
                let mut collision = force();
                let mut torque = V {
                    xyz: [f32::NAN; 3],
                    w: None,
                };
                let active = SkateboardSlideCollisionServices {
                    input: &mut input,
                    attributes: &mut runtime.slide_collision_attributes,
                    numeric: &mut numeric.reckoning,
                }
                .try_collision(&mut collision, &mut torque, None)
                .map_err(error)?;
                if active {
                    runtime.queue.add(15, collision);
                } else {
                    runtime.queue.add(4, front);
                    runtime.queue.add(5, rear);
                    runtime.queue.add(16, wall);
                    runtime.queue.add(9, slide);
                }
                Ok::<_, SlideError>(())
            })
            .map_err(error)??;
        Ok(phase)
    }
    fn slide_ground_orientation(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<V, SlideError> {
        let p = processed.lock().map_err(error)?;
        let velocity = need(p.prior_output().velocity(), "velocity")?.value;
        let normal = need(p.prior_output().ground_normal(), "normal")?.value;
        let slide = need(p.physics_state_fields().slide, "slide")?;
        let direction = try_normalize_unsafe(velocity, &mut numeric.reckoning).map_err(error)?;
        let side = try_normalize_unsafe(cross_typed(normal, direction), &mut numeric.reckoning)
            .map_err(error)?;
        let angle = slide * std::f32::consts::PI;
        let sine =
            crate::reckoning_sin_cos::sine::try_sine(angle, &mut numeric.sin_cos).map_err(error)?;
        let cosine =
            crate::reckoning_sin_cos::try_cosine(angle, &mut numeric.sin_cos).map_err(error)?;
        let desired = direction.zip(side.map(|v| v * sine), |d, s| d.mul_add(cosine, s));
        // 82d8e50c: Processed+160 `mBodyZAxis`, not the flattened +170 direction.
        let body_z = need(p.toolkit_state().body_z(), "body Z axis160")?;
        let mut turn = cross_typed(body_z, desired);
        if numeric
            .reckoning
            .dot_vector(body_z, desired)
            .map_err(error)?
            < 0.0
        {
            turn = normal.map(|v| if turn.xyz[1] < 0.0 { -v } else { v });
        }
        let hardness = need(p.animation_wheel_hardness(), "wheel hardness")?;
        let surface = need(
            p.selected_surface().and_then(|s| s.selected.as_ref()),
            "surface physics",
        )?;
        let angular = need(p.prior_output().angular_velocity(), "angular velocity")?.value;
        need(self.skaters.as_mut(), "pool")?
            .try_with_ground_control_board(&self.backend, board, |_, a| {
                let blend = a
                    .read_float(Class::Wheels, 0xca0b_c775_dff3_9de6)
                    .map_err(error)?;
                if blend < 0.0 || blend > 1.0 {
                    return Err(error("source slide hardness blend assertion"));
                }
                let damping = surface
                    .try_float(SlideSurfaceField::SlideAngularDamping)
                    .map_err(error)?;
                let strength = surface
                    .try_float(SlideSurfaceField::SlideOrientationStrength)
                    .map_err(error)?;
                let strength = ((1.0 - blend).mul_add(hardness, blend)) * strength;
                let turn = turn.zip(angular.map(|v| v * damping), |v, w| v.mul_add(strength, w));
                let scalar = a
                    .read_float(Class::Slide, 0x4515_ae74_0928_9c50)
                    .map_err(error)?;
                let turn = turn.map(|v| v * scalar);
                let along = numeric.reckoning.dot_vector(turn, normal).map_err(error)?;
                Ok(normal.map(|v| v * along))
            })
            .map_err(error)?
    }
}

fn power_slide_force<N: ReckoningNumeric>(
    processed: &Arc<Mutex<SkeletonProcessedContact>>,
    attributes: &mut crate::skateboard_attributes::SkateboardAttributes,
    numeric: &mut N,
) -> Result<SkateboardFootForce, SlideError>
where
    N::Error: std::fmt::Debug,
{
    let p = processed.lock().map_err(error)?;
    let velocity = need(p.prior_output().velocity(), "slide velocity")?.value;
    let normal = need(p.prior_output().ground_normal(), "slide normal")?.value;
    let dot = numeric.dot_vector(velocity, normal).map_err(error)?;
    //82b28088/94 are separate VMUL/VSub, not a fused negative multiply-add.
    let tangent = velocity.zip(normal.map(|n| n * dot), |v, projection| v - projection);
    let right = need(p.deck_transform(), "slide deck")?.columns()[0];
    let dot = numeric.dot_vector(tangent, right).map_err(error)?;
    let base = right.map(|v| v * dot);
    let height = attributes
        .read_float(Class::Slide, 0x27db_0582_0432_b632)
        .map_err(error)?;
    let surface = need(
        p.selected_surface().and_then(|s| s.selected.as_ref()),
        "slide surface",
    )?;
    let graph = surface
        .try_power_slide_graph(need(p.toolkit_state().abs_body_speed(), "abs body speed")?)
        .map_err(error)?;
    let hardness = if graph <= 0.0 {
        1.0
    } else {
        need(p.animation_wheel_hardness(), "slide hardness")?
    };
    let blend = attributes
        .read_float(Class::Wheels, 0x225b_8e10_5a71_4976)
        .map_err(error)?;
    if blend < 0.0 || blend > 1.0 {
        return Err(error("source power slide blend assertion"));
    }
    let scalar = (1.0 - blend).mul_add(hardness, blend) * graph;
    let force = base.map(|v| v * scalar);
    let dot = numeric.dot_vector(force, velocity).map_err(error)?;
    Ok(SkateboardFootForce {
        world_space_force: force,
        local_space_position: V::known([0.0, if dot >= 0.0 { height } else { -height }, 0.0, 0.0]),
    })
}
