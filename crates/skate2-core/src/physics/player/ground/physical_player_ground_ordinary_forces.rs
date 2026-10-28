//! Consecutive Ground82d9a658..a770, through retained `AntiFlip` publication.
//! Shared leaf/refinement amendments listed in ground-ordinary-force-composition.md.
use super::{
    antiflip_source::{AntiFlipRuntime, AntiFlipRuntimeError},
    ground_collision::GroundAfterCollision,
    ground_wallride::GroundBeforeFakeTorque,
    heading_torque_source::{HeadingRuntime, HeadingRuntimeError},
    pumping_force_source::{PumpingForceRuntime, PumpingForceRuntimeError},
    push_source::{PushRuntime, PushRuntimeError},
    straighten_source::{StraightenRuntime, StraightenRuntimeError},
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::SimulationConstructedBoardError,
    },
    input_cvar_runtime::{
        InputCvarReferences,
        ground_foot_force::{GroundFootCvars, GroundFootDebugError},
        pumping_forces::{PumpingForceCvars, PumpingForceDebugError},
        speed_model::{SpeedCvarError, SpeedModelCvars},
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    skateboard_brake_attributes::SkateboardBrakeAttributeError,
    skateboard_brake_drag::{SkateboardBrakeDragError, SkateboardBrakeDragServices},
    skateboard_foot_attributes::{SkateboardFootAttributeError, SkateboardFootAttributeRead},
    skateboard_foot_forces::{
        SkateboardFootForce, SkateboardFootForceError, SkateboardFootForceServices,
    },
    skateboard_force_runtime::{
        antiflip::{AntiFlipError, try_antiflip},
        entry::heading_torque::HeadingTorqueError,
        entry::speed_model_update::SpeedModelError,
        pumping_forces::{PumpForceError, try_absorption, try_pump_with_refinement},
        push,
        straighten::{StraightenError, try_straighten},
    },
    skateboard_slide_collision::{SkateboardSlideCollisionError, SkateboardSlideCollisionServices},
    skateboard_slide_collision_attributes::{
        SkateboardSlideCollisionAttributeError, speed_model::SpeedModelVault,
    },
    skeleton_processed_contact::{
        ground_force_input::{GroundForceInput, GroundForceInputError},
        speed_model_input::{SpeedModelInput, SpeedModelInputError},
        speed_model_source::{SpeedModelRuntime, SpeedRuntimeError},
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

/// Native stack results retained through the pending torque/queue suffix.
/// None of these outputs has been added to the board force queue yet.
pub(crate) struct GroundBeforeManualEffect {
    pub(crate) before: GroundBeforeFakeTorque,
    pub(crate) foot_threshold: f32,
    pub(crate) brake: SkateboardFootForce,
    pub(crate) push: SkateboardFootForce,
    pub(crate) front: SkateboardFootForce,
    pub(crate) rear: SkateboardFootForce,
    pub(crate) speed: SkateboardFootForce,
    pub(crate) slide: SkateboardFootForce,
    pub(crate) pump: SkateboardFootForce,
    pub(crate) straighten: SkeletonRootVector,
    pub(crate) heading: SkeletonRootVector,
    pub(crate) antiflip: SkeletonRootVector,
}
#[derive(Debug)]
pub(crate) enum GroundOrdinaryForceError<N> {
    Missing(&'static str),
    Poisoned,
    Foreign,
    CollisionAlreadyExited,
    Pair(SimulationSkeletonJumpPairError),
    Board(SimulationConstructedBoardError),
    Threshold(SkateboardFootAttributeError),
    Straighten(StraightenError<StraightenRuntimeError, N>),
    Heading(HeadingTorqueError<HeadingRuntimeError, N>),
    AntiFlip(AntiFlipError<AntiFlipRuntimeError, N>),
    Brake(SkateboardBrakeDragError<GroundForceInputError, SkateboardBrakeAttributeError>),
    Push(PushRuntimeError),
    Absorption(PumpingForceRuntimeError<PumpingForceDebugError>),
    Foot(
        SkateboardFootForceError<
            GroundForceInputError,
            SkateboardFootAttributeError,
            GroundFootDebugError,
            N,
        >,
    ),
    SpeedPrefix(SpeedModelInputError),
    Speed(SpeedModelError<SpeedRuntimeError<SpeedCvarError>, N>),
    Slide(
        SkateboardSlideCollisionError<
            GroundForceInputError,
            SkateboardSlideCollisionAttributeError,
            N,
        >,
    ),
    Pump(PumpForceError<PumpingForceRuntimeError<PumpingForceDebugError>, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundOrdinaryForceError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "ordinary Ground requires {s}"),
            Self::Poisoned => f.write_str("ordinary Ground owner poisoned"),
            Self::Foreign => {
                f.write_str("ordinary Ground continuation belongs to another pair/state")
            }
            Self::CollisionAlreadyExited => {
                f.write_str("collision response already exited Ground control")
            }
            Self::Pair(e) => e.fmt(f),
            Self::Board(e) => e.fmt(f),
            Self::Threshold(e) => e.fmt(f),
            Self::Straighten(e) => e.fmt(f),
            Self::Heading(e) => e.fmt(f),
            Self::AntiFlip(e) => e.fmt(f),
            Self::Brake(e) => write!(f, "ordinary Ground braking: {e:?}"),
            Self::Push(e) => e.fmt(f),
            Self::Absorption(e) => e.fmt(f),
            Self::Foot(e) => e.fmt(f),
            Self::SpeedPrefix(e) => e.fmt(f),
            Self::Speed(e) => e.fmt(f),
            Self::Slide(e) => e.fmt(f),
            Self::Pump(e) => e.fmt(f),
        }
    }
}

/// Math.NormalizeSafe824b1988/1998 final-negated FMA, distinct from drive policy.
pub(super) fn source_refinement(squared: f32, seed: f32) -> f32 {
    let square = seed * seed;
    let residual = -(squared.mul_add(square, -1.0));
    let first = (seed * 0.5).mul_add(residual, seed);
    let square = first * first;
    let residual = -(squared.mul_add(square, -1.0));
    (first * 0.5).mul_add(residual, first)
}
fn scratch_force() -> SkateboardFootForce {
    // Write-only backing, not source initialization. A successful leaf writes
    // both complete fields; errors expose no successful continuation.
    let scratch = SkeletonRootVector {
        xyz: [f32::NAN; 3],
        w: None,
    };
    SkateboardFootForce {
        world_space_force: scratch,
        local_space_position: scratch,
    }
}

impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_lines,
        reason = "one consecutive source caller with late actual-owner reads"
    )]
    pub(crate) fn try_update_pair_ground_ordinary_forces<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        after: GroundAfterCollision,
        references: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<GroundBeforeManualEffect, GroundOrdinaryForceError<N::Error>> {
        use GroundOrdinaryForceError as E;
        let before = match after {
            GroundAfterCollision::BeforeBraking(before) => before,
            GroundAfterCollision::CollisionExit(_) | GroundAfterCollision::JumpLaunch { .. } => {
                return Err(E::CollisionAlreadyExited);
            }
        };
        let owner = Arc::clone(&before.owner);
        let overlay = self.debug_overlay.clone();
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let same = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, &owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::Foreign);
        }
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.current != CurrentPhysicalState::Ground {
            return Err(E::Foreign);
        }
        let ground = state.ground.as_mut().ok_or(E::Missing("Ground"))?;
        if !ground.bindings.board.same_as(board) || !ground.bindings.skeleton.same_as(skeleton) {
            return Err(E::Foreign);
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut foot_debug = GroundFootCvars {
            references,
            overlay: &overlay,
        };
        let mut pumping_debug = PumpingForceCvars {
            references,
            overlay: &overlay,
        };
        let mut speed_debug = SpeedModelCvars {
            references,
            overlay: &overlay,
        };
        let outputs = pool
            .try_with_ground_force_board(&self.backend, board, |session, runtime, manual| {
                let pointer = runtime
                    .foot_attributes
                    .lookup(0x21b2_12a0_85d3_1d3b)
                    .map_err(E::Threshold)?;
                let foot_threshold = runtime
                    .foot_attributes
                    .float(&pointer)
                    .map_err(E::Threshold)?;
                let reckoning = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
                let mut input = GroundForceInput {
                    processed: &processed,
                    reckoning: reckoning.reckoning(),
                };
                let mut brake = scratch_force();
                SkateboardBrakeDragServices {
                    input: &mut input,
                    attributes: &mut runtime.brake_attributes,
                }
                .try_braking(&mut brake)
                .map_err(E::Brake)?;
                let mut push = scratch_force();
                push::try_calculate(
                    &mut PushRuntime {
                        processed: &processed,
                        entry: &mut runtime.entry,
                        attributes: &runtime.push_attributes,
                        owner: &owner,
                    },
                    &mut push,
                )
                .map_err(E::Push)?;
                let absorption = try_absorption(&mut PumpingForceRuntime {
                    processed: &processed,
                    pumping: &runtime.pumping,
                    attributes: owner.pumping_attributes(),
                    feet: &mut runtime.foot_attributes,
                    debug: &mut pumping_debug,
                })
                .map_err(E::Absorption)?;
                let landing = processed
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .input_controls()
                    .time_on_ground
                    .ok_or(E::Missing("Processed9c0 front"))?;
                let mut front = scratch_force();
                SkateboardFootForceServices {
                    input: &mut input,
                    attributes: &mut runtime.foot_attributes,
                    debug: &mut foot_debug,
                    numeric,
                }
                .try_front_with_refinement(landing, absorption, &mut front, source_refinement)
                .map_err(E::Foot)?;
                let landing = processed
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .input_controls()
                    .time_on_ground
                    .ok_or(E::Missing("Processed9c0 rear"))?;
                let mut rear = scratch_force();
                SkateboardFootForceServices {
                    input: &mut input,
                    attributes: &mut runtime.foot_attributes,
                    debug: &mut foot_debug,
                    numeric,
                }
                .try_rear_with_refinement(landing, absorption, &mut rear, source_refinement)
                .map_err(E::Foot)?;
                // End host borrows before the actual effective-transform getter
                // captures this same owner. No source value is prefetched.
                drop(reckoning);
                let prefix = runtime
                    .entry
                    .try_speed_model_prefix(&mut SpeedModelInput(&processed))
                    .map_err(E::SpeedPrefix)?;
                let mut speed = scratch_force();
                runtime
                    .entry
                    .try_speed_model_update_with_refinement(
                        prefix,
                        &mut SpeedModelRuntime {
                            processed: &processed,
                            board: session,
                            backend: &self.backend,
                            owner: &owner,
                            attributes: SpeedModelVault {
                                conservation: &runtime.conservation_attributes,
                                manual,
                                friction: &mut runtime.slide_collision_attributes,
                            },
                            debug: &mut speed_debug,
                        },
                        numeric,
                        &mut speed,
                        source_refinement,
                    )
                    .map_err(E::Speed)?;
                let time = ground.fields.time_in_state; //a71c, after SpeedModel mutation
                let reckoning = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
                let mut input = GroundForceInput {
                    processed: &processed,
                    reckoning: reckoning.reckoning(),
                };
                let mut slide = scratch_force();
                SkateboardSlideCollisionServices {
                    input: &mut input,
                    attributes: &mut runtime.slide_collision_attributes,
                    numeric,
                }
                .try_slide(time, &mut slide)
                .map_err(E::Slide)?;
                let mut pump = scratch_force();
                try_pump_with_refinement(
                    &mut PumpingForceRuntime {
                        processed: &processed,
                        pumping: &runtime.pumping,
                        attributes: owner.pumping_attributes(),
                        feet: &mut runtime.foot_attributes,
                        debug: &mut pumping_debug,
                    },
                    numeric,
                    &mut pump,
                    source_refinement,
                )
                .map_err(E::Pump)?;
                let time = ground.fields.time_in_state; //a738, fresh after Pump
                let mut straighten = scratch_force().world_space_force;
                try_straighten(
                    &mut StraightenRuntime {
                        input,
                        board_attributes: manual,
                        heading: owner.heading_attributes(),
                    },
                    time,
                    numeric,
                    &mut straighten,
                )
                .map_err(E::Straighten)?;
                // Heading performs a late read of this same Reckoning owner.
                drop(reckoning);
                let mut heading = scratch_force().world_space_force;
                runtime
                    .entry
                    .try_heading_torque(
                        &mut HeadingRuntime {
                            owner: &owner,
                            processed: &processed,
                            board_attributes: manual,
                            references,
                            overlay: &overlay,
                        },
                        numeric,
                        &mut heading,
                    )
                    .map_err(E::Heading)?;
                let mut antiflip = scratch_force().world_space_force;
                try_antiflip(
                    &mut AntiFlipRuntime {
                        processed: &processed,
                        feet: &runtime.foot_attributes,
                        references,
                        overlay: &overlay,
                    },
                    numeric,
                    &mut antiflip,
                )
                .map_err(E::AntiFlip)?;
                ground.fields.anti_flip_torque = Some(antiflip); //82d9a770
                Ok((
                    foot_threshold,
                    brake,
                    push,
                    front,
                    rear,
                    speed,
                    slide,
                    pump,
                    straighten,
                    heading,
                    antiflip,
                ))
            })
            .map_err(E::Board)??;
        let (
            foot_threshold,
            brake,
            push,
            front,
            rear,
            speed,
            slide,
            pump,
            straighten,
            heading,
            antiflip,
        ) = outputs;
        Ok(GroundBeforeManualEffect {
            before,
            foot_threshold,
            brake,
            push,
            front,
            rear,
            speed,
            slide,
            pump,
            straighten,
            heading,
            antiflip,
        })
    }
}

#[cfg(test)]
impl PhysicalAssemblyWorld {
    pub(crate) fn ground_force_queue_for_test(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Vec<crate::skateboard_force_queue::SkateboardQueuedForce> {
        self.skaters
            .as_mut()
            .unwrap()
            .try_with_ground_force_board(&self.backend, board, |_, runtime, _| {
                runtime.queue.records_for_test()
            })
            .unwrap()
    }
}
