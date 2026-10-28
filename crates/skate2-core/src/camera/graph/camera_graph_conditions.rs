//! Concrete camera leaves evaluated by the existing dynamic graph controller.
use crate::physics_state_ids::{
    PHYSICS_STATE_FOOTPLANT, PHYSICS_STATE_HANDPLANT, PHYSICS_STATE_LANDINGONDECK,
    PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_OFFBOARD_LOCOMOTION,
    PHYSICS_STATE_OFFBOARD_PUSHING, PHYSICS_STATE_SKITCHING,
};
use super::{CameraGraphBackend, CameraGraphLeafError};
use crate::{
    camera_graph::CameraContext,
    camera_graph_factory::{CameraCondition, CameraConditionData},
    generated_action_load::conditions::compare,
    reckoning_numeric::ReckoningNumeric,
    world_painter_layer::CAMERAS_RACE,
};

impl<N: ReckoningNumeric> CameraGraphBackend<'_, N> {
    #[expect(
        clippy::too_many_lines,
        reason = "source camera leaf dispatch keeps lazy owner reads local"
    )]
    pub(super) fn condition(
        &mut self,
        class: CameraCondition,
        data: &CameraConditionData,
        context: &CameraContext,
    ) -> Result<bool, CameraGraphLeafError<N::Error>> {
        use CameraCondition as C;
        use CameraGraphLeafError as E;
        if let CameraConditionData::Comparison(comparison) = data {
            // ComparisonCondition82cef7f8 calls GetValue even for NOP.
            let value = self.value(class, context)?;
            return Ok(compare(*comparison, value));
        }
        match class {
            C::CameraType => {
                let CameraConditionData::CameraType(expected) = data else {
                    return Err(E::Payload(class));
                };
                Ok(*expected == self.global.current_camera_type)
            }
            C::OnRoad => Ok(self.global.on_road),
            C::LedgeLeft => Ok(self.global.ledge_left),
            C::LedgeRight => Ok(self.global.ledge_right),
            C::RunningOut => Ok(self
                .subject
                .lock()
                .map_err(|_| E::OwnerLock)?
                .flags
                .running_out),
            C::UnderThreat => {
                if self
                    .inputs
                    .cvars
                    .try_camera_npc_threat_test()
                    .map_err(E::Cvar)?
                {
                    return Ok(true);
                }
                // SkaterIsUnderThreat 0x82e34788: `LW::Man::GetChaseMan()->ThreatIsOn()`, the living
                // world's chase (a chaser or a warning guard on the skater, or the skater jacked),
                // which turns the camera graph's NPC_Security state and its `bl_security` shot on.
                Ok(self
                    .inputs
                    .living_world
                    .lock()
                    .map_err(|_| E::OwnerLock)?
                    .chase
                    .as_ref()
                    .is_some_and(crate::living_world_chase::ChaseMan::threat_is_on))
            }
            C::PainterRegion => {
                let CameraConditionData::PainterRegion { layer, region } = data else {
                    return Err(E::Payload(class));
                };
                // SetWorldPainterRegionBlendValue 0x82e36678 reads camera
                // region data only on eCameras_Race.
                if *layer as usize != CAMERAS_RACE {
                    return Err(E::Payload(class));
                }
                let output = self
                    .global
                    .phys_out
                    .as_ref()
                    .ok_or(E::MissingPhysOut)?
                    .phys_out()
                    .lock()
                    .map_err(|_| E::OwnerLock)?;
                let hips = output
                    .try_skeleton_output()
                    .map_err(E::Component)?
                    .skeleton_hips_pos;
                let painted = self
                    .inputs
                    .layers
                    .lock()
                    .map_err(|_| E::OwnerLock)?
                    .try_camera_region([hips.xyz[0], hips.xyz[2]])
                    .map_err(E::Layer)?;
                // strncmp over the condition type's length: a prefix match,
                // which publishes the region's blend value.
                let Some((_, blend)) =
                    painted.filter(|(kind, _)| kind.as_bytes().starts_with(region.as_bytes()))
                else {
                    return Ok(false);
                };
                self.global.painted_region_blend = blend;
                Ok(true)
            }
            C::Volume => {
                let CameraConditionData::Volume(name) = data else {
                    return Err(E::Payload(class));
                };
                // Native gets the actual group before asking Subject for its root.
                let group = self.inputs.triggers.lock().map_err(|_| E::OwnerLock)?;
                let root = self
                    .subject
                    .lock()
                    .map_err(|_| E::OwnerLock)?
                    .skeleton_root
                    .columns()[3];
                if group.try_subject_box_is_empty(root).map_err(E::Trigger)? {
                    Ok(false)
                } else {
                    Err(E::PendingVolumeName(std::sync::Arc::clone(name)))
                }
            }
            C::Offboard
            | C::AirOffboard
            | C::MovingObject
            | C::Skitching
            | C::HandPlant
            | C::FootPlant
            | C::WipingOut
            | C::HippyJump
            | C::Boneless => {
                let output = self
                    .global
                    .phys_out
                    .as_ref()
                    .ok_or(E::MissingPhysOut)?
                    .phys_out()
                    .lock()
                    .map_err(|_| E::OwnerLock)?;
                Ok(match class {
                    C::Offboard => {
                        let state = output
                            .try_state_output()
                            .map_err(E::Component)?
                            .physics_state;
                        if matches!(state, PHYSICS_STATE_OFFBOARD | PHYSICS_STATE_OFFBOARD_AIR | PHYSICS_STATE_LANDINGONDECK | PHYSICS_STATE_OFFBOARD_LOCOMOTION) {
                            true
                        } else {
                            output
                                .try_animation_output()
                                .map_err(E::Component)?
                                .transitioning_on_off_board
                        }
                    }
                    C::AirOffboard => {
                        output
                            .try_state_output()
                            .map_err(E::Component)?
                            .physics_state
                            == PHYSICS_STATE_OFFBOARD_AIR
                    }
                    C::MovingObject => {
                        output
                            .try_state_output()
                            .map_err(E::Component)?
                            .physics_state
                            == PHYSICS_STATE_OFFBOARD_PUSHING
                    }
                    C::Skitching => {
                        output
                            .try_state_output()
                            .map_err(E::Component)?
                            .physics_state
                            == PHYSICS_STATE_SKITCHING
                    }
                    C::HandPlant => {
                        output
                            .try_state_output()
                            .map_err(E::Component)?
                            .physics_state
                            == PHYSICS_STATE_HANDPLANT
                    }
                    C::FootPlant => {
                        output
                            .try_state_output()
                            .map_err(E::Component)?
                            .physics_state
                            == PHYSICS_STATE_FOOTPLANT
                    }
                    C::WipingOut => {
                        output
                            .try_state_output()
                            .map_err(E::Component)?
                            .is_wiping_out
                    }
                    C::HippyJump => {
                        output
                            .try_ground_output()
                            .map_err(E::Component)?
                            .is_full_hippy_jump
                    }
                    C::Boneless => output.try_camera_is_push_tricking().map_err(E::Component)?,
                    _ => unreachable!("outer class match"),
                })
            }
            _ => {
                let Some(handle) = context.camera_man() else {
                    return Ok(false);
                };
                let man = handle.0.lock().map_err(|_| E::OwnerLock)?;
                match class {
                    C::BoardOnGround => Ok(man.rig.subject_state == 0),
                    C::Grinding => Ok(man.rig.subject_state == 2),
                    C::WasGrinding => Ok(man.rig.previous_subject_state == 2),
                    C::Reentry => Ok(man.flags & 8 != 0),
                    C::InManual | C::PreparingJump => {
                        let subject = man
                            .subject
                            .as_ref()
                            .ok_or(E::MissingSubject)?
                            .lock()
                            .map_err(|_| E::OwnerLock)?;
                        Ok(if class == C::InManual {
                            subject.flags.manual
                        } else {
                            subject.flags.anticipating
                        })
                    }
                    C::PreviousShot => {
                        let CameraConditionData::PreviousShots(hashes) = data else {
                            return Err(E::Payload(class));
                        };
                        let current = man.shots.current.name_hash;
                        Ok(hashes.contains(&current))
                    }
                    _ => Err(E::Payload(class)),
                }
            }
        }
    }

    fn value(
        &mut self,
        class: CameraCondition,
        context: &CameraContext,
    ) -> Result<f32, CameraGraphLeafError<N::Error>> {
        use CameraCondition as C;
        use CameraGraphLeafError as E;
        let Some(handle) = context.camera_man() else {
            return Ok(0.0);
        };
        let man = handle.0.lock().map_err(|_| E::OwnerLock)?;
        Ok(match class {
            C::TurningCentredTime => man.turning_centred_time,
            C::PredictedLandingHeight => man.landing_relative_height,
            C::PredictedMaxHeight => man.maximum_prediction_height,
            C::PredictedHeightDistanceRatio => man.trajectory_height_distance_ratio,
            C::FrontsideAngle => man.frontside_angle,
            C::TransferAngle => man.transfer_angle * f32::from_bits(0x4265_2ee1),
            C::GrindInclineAngle => man.grind_incline * f32::from_bits(0x4265_2ee1),
            C::LaunchInclineAngle => man.absolute_launch_incline * f32::from_bits(0x4265_2ee1),
            C::PredictedAirTime => {
                let subject = man
                    .subject
                    .as_ref()
                    .ok_or(E::MissingSubject)?
                    .lock()
                    .map_err(|_| E::OwnerLock)?;
                subject.trajectory_collision_time()
            }
            C::SubjectSpeed => man.rig.try_subject_speed(self.numeric).map_err(E::Speed)?,
            _ => return Err(E::Payload(class)),
        })
    }
}
