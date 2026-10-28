//! Skitch.UpdateSkateboard82da0960; existing board toolkit/queues/drive owners.
use super::{
    antiflip_source::AntiFlipRuntime,
    ground_control_complete::{Services, SteeringInput},
    ground_pre_solve::HostGroundPreparationNumeric,
    heading_torque_source::HeadingRuntime,
    manual_effect_source::ManualRuntime,
    skitch_session::state,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle as Handle,
        board_construction::ground_control::Source as PhysicsErrorSource,
    },
    input_cvar_runtime::{InputCvarReferences, skitch::SkitchBool as B},
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_grind_manager::{
        math::GrindNum,
        view::{GrindError as E, error, lanes, need, root},
    },
    reckoning_numeric::ReckoningNumeric,
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_force_runtime::{
        antiflip::try_antiflip,
        entry::manual_effect::ManualEffect,
        speed_wobble::calculate::{WobbleInput, attributes::WobbleVault},
    },
    skateboard_slide_collision::SkateboardSlideCollisionServices,
    skateboard_steering::calculate_attributes::SteeringCalculateVault,
    skeleton_processed_contact::ground_force_input::GroundForceInput,
    skeleton_root_transforms::SkeletonRootVector as V,
};
use std::sync::Arc;
const ZERO: V = V::known([0.; 4]);
fn force() -> SkateboardFootForce {
    SkateboardFootForce {
        world_space_force: ZERO,
        local_space_position: ZERO,
    }
}
impl PhysicalAssemblyWorld {
    pub(super) fn queue_skitch_force(
        &mut self,
        board: &Handle,
        kind: u32,
        v: [f32; 4],
    ) -> Result<(), E> {
        need(self.skaters.as_mut(), "pool")?
            .try_with_ground_force_board(&self.backend, board, |_, runtime, _| {
                runtime.queue.add(
                    kind,
                    SkateboardFootForce {
                        world_space_force: root(v),
                        local_space_position: ZERO,
                    },
                );
            })
            .map_err(error)
    }
    pub(super) fn try_update_skitch_board(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &Handle,
        skeleton: &Handle,
        refs: &InputCvarReferences,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<(), E> {
        let p = Arc::clone(owner.reckoning().lock().map_err(error)?.processed());
        need(self.skaters.as_mut(), "pool")?
            .try_with_ground_force_board(&self.backend, board, |session, runtime, _| {
                runtime
                    .steering
                    .try_update_physics_error(
                        &mut PhysicsErrorSource {
                            processed: &p,
                            board: session,
                            backend: &self.backend,
                        },
                        &mut numeric.reckoning,
                    )
                    .map_err(error)
            })
            .map_err(error)??;
        let damping = need(
            p.lock().map_err(error)?.prior_output().z_speed(),
            "truck damping speed",
        )?
        .value;
        let (ended, in_air, state_time) = state(owner, board, skeleton, |s| {
            Ok((
                need(s.skitch_state, "skitch state")? == 4,
                need(s.in_air, "skitch in air")?,
                need(s.time_in_skitch_state, "skitch time")?,
            ))
        })?;
        let mut speed = [0.; 4];
        let mut slide = force();
        let mut heading = [0.; 4];
        if !ended {
            let bone_z = need(
                self.skeleton(skeleton)
                    .map_err(error)?
                    .animation_targets()
                    .animation_row(0),
                "board raw bone",
            )?[14];
            let (mass, mirror, grab_z, deck) = {
                let p = p.lock().map_err(error)?;
                (
                    need(p.physical_mass(), "mass")?,
                    need(p.physics_state_fields().flags_8cc, "stance")? & 8 != 0,
                    need(p.animation_attribute_fields().world_grab_z, "world grab Z")?,
                    need(p.deck_transform(), "deck")?.columns().map(lanes),
                )
            };
            speed = state(owner, board, skeleton, |s| {
                s.speed_force(
                    bone_z,
                    mirror,
                    grab_z,
                    mass,
                    &mut GrindNum {
                        n: &mut numeric.reckoning,
                        s: &mut numeric.sin_cos,
                    },
                )
                .map_err(error)
            })?;
            need(self.skaters.as_mut(), "pool")?
                .try_with_ground_force_board(&self.backend, board, |_, runtime, _| {
                    let reckoning = owner.reckoning().lock().map_err(error)?;
                    SkateboardSlideCollisionServices {
                        input: &mut GroundForceInput {
                            processed: &p,
                            reckoning: reckoning.reckoning(),
                        },
                        attributes: &mut runtime.slide_collision_attributes,
                        numeric: &mut numeric.reckoning,
                    }
                    .try_slide(state_time, &mut slide)
                    .map_err(error)
                })
                .map_err(error)??;
            heading = state(owner, board, skeleton, |s| {
                s.heading_displacement(
                    deck,
                    refs.try_skitch_bool(B::kSkitchingUseTorque)
                        .map_err(error)?,
                    &mut GrindNum {
                        n: &mut numeric.reckoning,
                        s: &mut numeric.sin_cos,
                    },
                )
                .map_err(error)
            })?;
        }
        let mut tilt = 0.;
        if (ended
            || refs
                .try_skitch_bool(B::kSkitchingApplyStandardTilt)
                .map_err(error)?)
            && !in_air
        {
            let p = p.lock().map_err(error)?;
            tilt = need(self.skaters.as_mut(), "pool")?
                .try_with_ground_control_board(&self.backend, board, |runtime, manual| {
                    crate::skateboard_steering::calculate::try_calculate(
                        &*p,
                        &mut SteeringCalculateVault {
                            steering: &mut runtime.steering_attributes,
                            manual,
                        },
                        None,
                        None,
                    )
                    .map_err(error)
                })
                .map_err(error)??;
        }
        if refs
            .try_skitch_bool(B::kSkitchingApplySpeedWobble)
            .map_err(error)?
            && !in_air
        {
            // GetCOMToDeckWorld82b3b468 copies the retained world-space vector
            // at Skeleton+2a30. It must not be transformed a second time.
            let normal = self.skeleton(skeleton).map_err(error)?;
            let com = lanes(normal.current_animation().record().com_to_deck());
            let squared = numeric.reckoning.dot3(com, com).map_err(error)?;
            let estimate = numeric
                .reckoning
                .reciprocal_sqrt_estimate(squared)
                .map_err(error)?;
            let length = crate::physical_player_phys_out::construct::board_fill::board_speed_from_squared_and_estimate(squared, estimate);
            tilt = need(self.skaters.as_mut(), "pool")?
                .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                    let wobble = need(runtime.speed_wobble.value_mut(), "SpeedWobble")?;
                    let p = p.lock().map_err(error)?;
                    let input = WobbleInput {
                        tilt,
                        speed: need(p.prior_output().z_speed(), "speed")?.value,
                        com_to_deck_length: length,
                        truck_tightness: need(
                            p.animation_input().truck_tightness,
                            "truck tightness",
                        )?,
                        dt: need(p.physics_state_fields().time_delta, "DT")?,
                    };
                    let attributes = Arc::clone(wobble.attributes());
                    wobble
                        .try_calculate(
                            input,
                            &mut WobbleVault(&attributes),
                            &mut Services {
                                references: refs,
                                overlay: &self.debug_overlay,
                                numeric: &mut numeric.sin_cos,
                            },
                        )
                        .map_err(error)
                })
                .map_err(error)??;
        }
        let overlay = self.debug_overlay.clone();
        let mut anti = ZERO;
        if !ended {
            need(self.skaters.as_mut(), "pool")?
                .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                    try_antiflip(
                        &mut AntiFlipRuntime {
                            processed: &p,
                            feet: &runtime.foot_attributes,
                            references: refs,
                            overlay: &overlay,
                        },
                        &mut numeric.reckoning,
                        &mut anti,
                    )
                    .map_err(error)
                })
                .map_err(error)??;
        }
        self.try_with_constructed_board_truck_drives(board, |drives, _| {
            drives.set_speed_damping_factor(damping)
        })
        .map_err(error)?;
        self.try_update_constructed_board_steering(board, tilt, &mut SteeringInput(&p))
            .map_err(error)?;
        if ended {
            return Ok(());
        }
        let use_heading = {
            let p = p.lock().map_err(error)?;
            need(p.physics_state_fields().balance, "balance")? != 0.
                && need(p.jump_events().spin(), "spin")? != 0.
        };
        if use_heading {
            let mut torque = ZERO;
            need(self.skaters.as_mut(), "pool")?
                .try_with_ground_control_board(&self.backend, board, |runtime, attrs| {
                    runtime
                        .entry
                        .try_heading_torque(
                            &mut HeadingRuntime {
                                owner,
                                processed: &p,
                                board_attributes: attrs,
                                references: refs,
                                overlay: &overlay,
                            },
                            &mut numeric.reckoning,
                            &mut torque,
                        )
                        .map_err(error)
                })
                .map_err(error)??;
            need(self.skaters.as_mut(), "pool")?
                .try_target_board_angular_displacement(
                    &mut self.backend,
                    board,
                    torque,
                    &mut numeric.reckoning,
                )
                .map_err(error)?;
        }
        if refs
            .try_skitch_bool(B::kSkitchingAllowManuals)
            .map_err(error)?
        {
            let manual = need(self.skaters.as_mut(), "pool")?
                .try_with_ground_force_board(&self.backend, board, |session, runtime, attrs| {
                    let mut out = ManualEffect {
                        angular_displacement: ZERO,
                        corrective_force: force(),
                    };
                    runtime
                        .entry
                        .try_manual_effect(
                            &mut ManualRuntime {
                                owner,
                                processed: &p,
                                board: session,
                                backend: &self.backend,
                                attributes: attrs,
                                references: refs,
                                overlay: &overlay,
                            },
                            false,
                            &mut numeric.reckoning,
                            &mut numeric.sin_cos,
                            &mut out,
                        )
                        .map_err(error)?;
                    Ok::<_, E>(out)
                })
                .map_err(error)??;
            need(self.skaters.as_mut(), "pool")?
                .try_apply_board_angular_displacement(
                    &mut self.backend,
                    board,
                    manual.angular_displacement,
                    &mut numeric.reckoning,
                )
                .map_err(error)?;
        }
        need(self.skaters.as_mut(), "pool")?
            .try_apply_board_angular_displacement(
                &mut self.backend,
                board,
                anti,
                &mut numeric.reckoning,
            )
            .map_err(error)?;
        if state(owner, board, skeleton, |s| {
            need(s.skitch_steering_active, "skitch steering")
        })? {
            self.try_controller_board_angular_exact(board, root(heading), &mut numeric.reckoning)
                .map_err(error)?;
        }
        self.queue_skitch_force(board, 6, speed)?;
        need(self.skaters.as_mut(), "pool")?
            .try_with_ground_force_board(&self.backend, board, |_, runtime, _| {
                runtime.queue.add(1, slide)
            })
            .map_err(error)?;
        state(owner, board, skeleton, |s| {
            s.time_grabbing =
                Some(need(s.time_grabbing, "grab time")? + f32::from_bits(0x3c88_8889));
            Ok(())
        })
    }
}
