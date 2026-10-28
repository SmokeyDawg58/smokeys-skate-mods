//! Behaviours and conditions of HCDelegate_marquee (`marquee.xml`) and HCDelegate_Photographer
//! (`Photographer.xml`), the challenge characters' AI graphs, on the retained pedestrian owners.
use super::behaviour_binding::Phase;
use crate::{
    ai::vmx,
    generated_action_load::{Attribute, Section},
    living_world_chase::TargetableId,
    living_world_pedestrian_entity::PedestrianConstruction,
    pedestrian_adjust_owners::{HeadTrackingMode, head_update},
    pedestrian_graph::backend::frame::FrameServices,
    pedestrian_motion_delegate::channel::Status,
    pedestrian_navigation_modifiers::ModifierKind,
    play_animation::{FastString30, FastString36},
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
};

/// `RADIANS_TO_DEGREES` (0x42652ee1).
const RADIANS_TO_DEGREES: f32 = f32::from_bits(0x4265_2ee1);
/// `tPedestrianStateStorage` `ProRecTimer`.
const PRO_REC_TIMER: u32 = 19;

/// A declaration's rows of the delegate's `kAttribs` (`GenAttributeMap`).
pub(crate) fn attributes(
    table: &'static [Attribute],
    section: Section,
) -> Result<&'static [Attribute], String> {
    if section.count == 0 {
        return Ok(&[]);
    }
    let start = usize::from(section.index);
    table
        .get(start..start + usize::from(section.count))
        .ok_or_else(|| "marquee attribute section outside kAttribs".into())
}

fn text(fields: &[Attribute], key: &str) -> Option<&'static str> {
    fields
        .iter()
        .find(|field| field.key == key)
        .map(|field| field.value)
}

/// `XMLAttributeMap::GetFloat` 0x828f9518: `atof` of the text, narrowed to float.
pub(crate) fn float(fields: &[Attribute], key: &str, default: f32) -> Result<f32, String> {
    text(fields, key).map_or(Ok(default), |value| {
        value
            .parse::<f64>()
            .map(|number| number as f32)
            .map_err(|_| format!("invalid native float {key}={value}"))
    })
}

/// `GenAttributeMap::GetBool` 0x828fb8a8 accepts the lower-case literal names only.
pub(crate) fn boolean(fields: &[Attribute], key: &str, default: bool) -> Result<bool, String> {
    match text(fields, key) {
        None => Ok(default),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(value) => Err(format!("invalid native Boolean {key}={value}")),
    }
}

/// `PedestrianColliding` constructor 0x826eabe8: `warnChance` (default 0.7) and
/// `broadcastradius` (20 when absent).
pub(crate) fn colliding(
    fields: &[Attribute],
) -> Result<super::colliding::CollidingBehaviour, String> {
    Ok(super::colliding::CollidingBehaviour {
        warn_chance: float(fields, "warnChance", f32::from_bits(0x3f33_3333))?,
        broadcast_radius: float(fields, "broadcastradius", 20.)?,
    })
}

/// `SendSpeechEvent` constructor 0x826ed130: the named event, its `speechvalue` truncated and
/// asserted within the 1..71 audio states.
pub(crate) fn speech(fields: &[Attribute]) -> Result<u32, String> {
    text(fields, "speechevent").ok_or("native speech name absent")?;
    let value = HostReckoningNumeric
        .truncate_i32(float(fields, "speechvalue", 0.)?)
        .map_err(|e| format!("speech enum: {e:?}"))?;
    if !(1..72).contains(&value) {
        return Err("native speech state assertion826ed1cc".into());
    }
    u32::try_from(value).map_err(|_| "speech enum sign".into())
}

/// The local skater's `PhysOut_SystemReckoning::mCentreOfMass` (`IActor::GetPhysOut` +0x24,
/// +0x30): `ISkaterManager::GetLocal` (vtable +0x10), or `FindByIndex(GetLocalIndex())`.
pub(crate) fn local_skater_centre(frame: &FrameServices<'_>) -> Result<[f32; 4], String> {
    let index = frame
        .world
        .local_skater_identity
        .as_ref()
        .ok_or("marquee SkaterManager absent")?
        .try_local_index()
        .map_err(|e| e.to_string())?;
    let actor = frame
        .actor(TargetableId { kind: 0, id: index })?
        .ok_or("native local skater assertion")?;
    head_update::actor_centre(actor)
}

/// `IsFacingSkater::ActivationFunc` 0x826ffea8 on positions flattened to y 0: within
/// `Distance` (unless it is zero) by the guarded length, then the entity facing's dot with
/// the offset (`Math::NormalizeSafe` 0x824b18e0, default zero) above `cos(FOVAngle)`
/// (`XMVectorCos` 0x824b0130). Unordered comparisons fail.
pub(crate) fn is_facing_skater(
    transform: &[[f32; 4]; 4],
    centre: [f32; 4],
    angle: f32,
    distance: f32,
) -> bool {
    let mut skater = centre;
    skater[1] = 0.;
    let mut position = transform[3];
    position[1] = 0.;
    let delta = vmx::sub(skater, position);
    if distance != 0. {
        let length = vmx::sqrt_guarded(vmx::dot3(delta, delta));
        if !(distance > length) {
            return false;
        }
    }
    let direction = vmx::normalize_safe(delta, [0.; 4]);
    vmx::dot3(transform[2], direction) > vmx::cos(angle)
}

/// `PhotographSkater::GetPitchToTarget` 0x826fd8b0: the height difference over the guarded
/// horizontal distance (`fdivs`), through `XMVectorATan` 0x82581658, in degrees.
pub(crate) fn pitch_to_target(current: [f32; 4], target: [f32; 4]) -> Result<f32, String> {
    let mut delta = vmx::sub(target, current);
    let rise = delta[1];
    delta[1] = 0.;
    let distance = vmx::sqrt_guarded(vmx::dot3(delta, delta));
    let angle =
        crate::skeleton_query_rotation::try_atan(rise / distance, &mut HostReckoningNumeric)
            .map_err(|e| format!("pitch atan: {e:?}"))?;
    Ok(angle * RADIANS_TO_DEGREES)
}

/// The marquee delegates' behaviour objects whose callbacks are not shared with the
/// `Pedestrian.xml` behaviours (vtable slots 11..14: create, begin, update, end).
pub(crate) enum MarqueeBehaviour {
    /// `DisableNavigation` (vtable 0x820cfe24).
    DisableNavigation,
    /// `LockToCurrentPosition` (vtable 0x820cfe9c).
    LockToCurrentPosition,
    /// `ProTrackSkater` (vtable 0x820cfbcc); constructor 0x826eca58 reads `RecTimer`, default 8.
    ProTrackSkater { rec_timer: f32 },
    /// `ProWaitingForSkater` (vtable 0x820cfc08).
    ProWaitingForSkater,
    /// `IgnoreCollision` (vtable 0x820cfcf8).
    IgnoreCollision,
    /// `PhotographerActions::PhotographSkater` (vtable 0x820cf0bc).
    PhotographSkater,
    /// `PhotographerActions::ReviewingPictures` (vtable 0x820cf0f8): its instance's timer.
    ReviewingPictures { timer: f32 },
}

impl MarqueeBehaviour {
    pub(crate) fn bind(name: &str, fields: &[Attribute]) -> Result<Self, String> {
        Ok(match name {
            "DisableNavigation" => Self::DisableNavigation,
            "LockToCurrentPosition" => Self::LockToCurrentPosition,
            "ProTrackSkater" => Self::ProTrackSkater {
                rec_timer: float(fields, "RecTimer", 8.)?,
            },
            "ProWaitingForSkater" => Self::ProWaitingForSkater,
            "IgnoreCollision" => Self::IgnoreCollision,
            "PhotographSkater" => Self::PhotographSkater,
            "ReviewingPictures" => Self::ReviewingPictures { timer: 30. },
            name => return Err(format!("unbound native marquee behaviour {name}")),
        })
    }

    /// Only `ReviewingPictures::CreateInstance` 0x82711ec0 allocates an instance.
    pub(crate) fn has_instance_factory(&self, source: u32) -> bool {
        matches!(self, Self::ReviewingPictures { .. }) && source == 0x8271_1ec0
    }

    /// 0x82711ec0: the instance's timer (`Data`+8) starts at 30.
    pub(crate) fn create_instance(&mut self) {
        if let Self::ReviewingPictures { timer } = self {
            *timer = 30.;
        }
    }

    /// The callbacks that need only the entity. False hands the callback to the frame:
    /// ProTrackSkater and PhotographSkater Update read the frame's owners and
    /// ReviewingPictures Update the controller's delta time.
    pub(crate) fn dispatch(
        &mut self,
        phase: Phase,
        entity: &mut PedestrianConstruction,
    ) -> Result<bool, String> {
        match (self, phase) {
            //826ece68 clears Pedestrian+1a44 bit80 every update; End826ece80 sets it.
            (Self::DisableNavigation, Phase::Update) => entity.navigation_rig_enabled = false,
            (Self::DisableNavigation, Phase::End) => entity.navigation_rig_enabled = true,
            //826ece98 locks the spatial adjust owner at GetPosition; End826ecf18 only unlocks.
            (Self::LockToCurrentPosition, Phase::Begin) => {
                let position = entity
                    .core
                    .as_ref()
                    .ok_or("position lock requires core")?
                    .transform[3];
                let adjust = entity
                    .spatial_adjust
                    .as_mut()
                    .ok_or("position lock requires spatial adjust")?;
                adjust.position_locked = true;
                adjust.locked_position = position;
            }
            (Self::LockToCurrentPosition, Phase::End) => {
                entity
                    .spatial_adjust
                    .as_mut()
                    .ok_or("position lock requires spatial adjust")?
                    .position_locked = false;
            }
            //826ecac8: SkaterCenterOfMass head tracking, then GetNavModifier(2) (skater
            // avoidance) SetEnabled(false).
            (Self::ProTrackSkater { .. }, Phase::Begin) => {
                entity
                    .head_tracking
                    .as_mut()
                    .ok_or("ProTrackSkater requires head tracking")?
                    .push(HeadTrackingMode::SkaterCenterOfMass)?;
                entity
                    .navigation_modifiers
                    .as_mut()
                    .ok_or("ProTrackSkater requires modifiers")?
                    .set_enabled(ModifierKind::Skater, false);
            }
            //826ec2c0 pops the head tracking mode.
            (Self::ProTrackSkater { .. }, Phase::End) => {
                entity
                    .head_tracking
                    .as_mut()
                    .ok_or("ProTrackSkater requires head tracking")?
                    .pop();
            }
            //826ecb50: skater avoidance off, then Pedestrian::ClearAllIntents.
            (Self::ProWaitingForSkater, Phase::Begin) => {
                entity
                    .navigation_modifiers
                    .as_mut()
                    .ok_or("ProWaitingForSkater requires modifiers")?
                    .set_enabled(ModifierKind::Skater, false);
                entity.clear_all_intents()?;
            }
            //826ecd20/826ecd38: state+ca9 bit08.
            (Self::IgnoreCollision, Phase::Begin) => {
                entity
                    .state
                    .as_mut()
                    .ok_or("IgnoreCollision requires AI state")?
                    .flags[1] |= 8;
            }
            (Self::IgnoreCollision, Phase::End) => {
                entity
                    .state
                    .as_mut()
                    .ok_or("IgnoreCollision requires AI state")?
                    .flags[1] &= !8;
            }
            //82707768 and 82707830: ClearAllIntents, then SuggestNextIntent.
            (Self::PhotographSkater, Phase::Begin) => {
                entity.clear_all_intents()?;
                entity.suggest_next_intent("PhotographSkater")?;
            }
            (Self::ReviewingPictures { .. }, Phase::Begin) => {
                entity.clear_all_intents()?;
                entity.suggest_next_intent("ReviewRecent")?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(crate) fn dispatch_frame(
        &mut self,
        phase: Phase,
        entity: &mut PedestrianConstruction,
        dt: f32,
        frame: &mut FrameServices<'_>,
    ) -> Result<(), String> {
        match (self, phase) {
            (Self::ProTrackSkater { rec_timer }, Phase::Update) => {
                update_pro_track_skater(entity, *rec_timer, frame)
            }
            (Self::PhotographSkater, Phase::Update) => update_photograph_skater(entity, frame),
            //827078f8: the controller's delta time runs the timer down; once it is no
            // longer non-negative mbPhotographerReviewingPictures (state+ca8 bit20) clears.
            (Self::ReviewingPictures { timer }, Phase::Update) => {
                *timer -= dt;
                if !(*timer >= 0.) {
                    entity
                        .state
                        .as_mut()
                        .ok_or("ReviewingPictures requires AI state")?
                        .flags[0] &= !0x20;
                }
                Ok(())
            }
            (_, phase) => Err(format!(
                "marquee {phase:?} callback does not read the frame"
            )),
        }
    }
}

/// `ProTrackSkater::Update` 0x82717548: audio state 32 (`SetAudioState`, Entity vtable
/// +0xb8), then, when the channel request has finished, `ProRecTimer` has run out and
/// `mbProChallengeStarted` (state+ca9 bit40) is clear: `GreetChannel` through
/// `IAnimRemap::RemapAnim` and `cPedAnimData::PlayChannelAnim` 0x82e93f20 (blends 0.2, not
/// cycling), audio state 28 and `RecTimer` on `ProRecTimer`.
fn update_pro_track_skater(
    entity: &mut PedestrianConstruction,
    rec_timer: f32,
    frame: &mut FrameServices<'_>,
) -> Result<(), String> {
    entity.state_out.state = 32;
    let finished = entity
        .animation
        .as_ref()
        .ok_or("ProTrackSkater requires animation")?
        .channels
        .channel
        .status
        == Status::Finished;
    let state = entity
        .state
        .as_ref()
        .ok_or("ProTrackSkater requires AI state")?;
    if !finished || state.timer(PRO_REC_TIMER) > 0. || state.flags[1] & 0x40 != 0 {
        return Ok(());
    }
    let animation = entity
        .animation
        .as_mut()
        .ok_or("ProTrackSkater requires animation")?;
    let mut failure = None;
    let random = &mut *frame.random;
    let mapped = crate::pedestrian_animation::remap_animation(
        "GreetChannel",
        &animation.motion_intents.registered_clips,
        animation.selected_animation_data()?,
        || match random
            .try_next(&crate::native_skater_random::NativeRandomGate::production_shared())
        {
            Ok(value) => value,
            Err(error) => {
                failure = Some(error);
                0
            }
        },
    )
    .map_err(|e| e.to_string())?;
    if let Some(error) = failure {
        return Err(format!("GreetChannel Manager RNG: {error:?}"));
    }
    let name =
        FastString36::encode(&mapped).map_err(|e| format!("GreetChannel remapped name: {e:?}"))?;
    animation.channels.play(name, 0.2, 0.2, false)?;
    entity.state_out.state = 28;
    entity
        .state
        .as_mut()
        .ok_or("ProTrackSkater requires AI state")?
        .set_timer(PRO_REC_TIMER, rec_timer)
        .map_err(|e| e.to_string())
}

/// `PhotographSkater::Update` 0x82717018: audio state 29, then the local skater's centre of
/// mass as `BearingToTarget` (`LivingWorldOrientationOps::GetBearingToTarget` 0x82e5acc0 of
/// the target transform) and `PitchToTarget` (from 0.9 above the position along
/// `gJVector`), written pitch first through `AddSettableAtt` (IAnimatable +0x48, not
/// normalized, sequence -1).
fn update_photograph_skater(
    entity: &mut PedestrianConstruction,
    frame: &mut FrameServices<'_>,
) -> Result<(), String> {
    entity.state_out.state = 29;
    let target = local_skater_centre(frame)?;
    let core = entity
        .core
        .as_ref()
        .ok_or("PhotographSkater requires entity core")?;
    let bearing = head_update::bearing(core.target_transform, target)?;
    let current = vmx::add(core.transform[3], [0., f32::from_bits(0x3f66_6666), 0., 0.]);
    let pitch = pitch_to_target(current, target)?;
    if std::env::var_os("SKATE2_TRACE_PHOTOGRAPHER").is_some() {
        eprintln!("photographer: pitch {pitch:.1} bearing {bearing:.1} at {:?} target {:?}", current, target);
    }
    let animation = entity
        .animation
        .as_mut()
        .ok_or("PhotographSkater requires animation")?;
    for (name, value) in [("PitchToTarget", pitch), ("BearingToTarget", bearing)] {
        let name = FastString30::encode(name).map_err(|e| format!("settable name: {e:?}"))?;
        animation
            .core
            .try_add_gesture_attribute(name, value, false, -1)
            .map_err(|e| format!("PhotographSkater AddSettableAtt: {e:?}"))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "pedestrian_graph_marquee_tests.rs"]
mod tests;
