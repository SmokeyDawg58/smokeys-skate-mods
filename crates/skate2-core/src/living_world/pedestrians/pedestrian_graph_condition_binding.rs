//! Retained constructors and activation dispatch for all 89 generated pedestrian
//! condition occurrences. Native expression/mask order stays in expressions.
use super::conditions::{self, CommonCondition, attribute, want_type};
use crate::{
    generated_action_load::construct::{Comparison, comparison_from_attributes},
    living_world_chase::TargetableId,
    living_world_guard_graph,
    living_world_pedestrian_entity::PedestrianConstruction,
    pedestrian_graph_data as data,
    reckoning_numeric::{ReckoningNumeric, try_signed_angle},
};

enum Condition {
    Common(CommonCondition),
    BigBlack(&'static str),
    DistanceFromProtectee(Comparison),
    Entity(&'static str),
    Guard { name: &'static str, reason: Option<i32> },
    Chase(&'static str),
    HasPlugin,
    TargetingWaypoint,
    Chasing,
    RoadTarget,
    EnteringCrosswalk,
    WalkSign,
    PluginCollisionOverride,
    HasHandProp,
    AttackThrow,
    HandProp(u64),
    Facing { want: usize, angle: f32, distance: f32 },
    Distance { want: usize, comparison: Comparison },
    AlertDistance(usize),
    InterestInBoundary,
    InterestInRadius,
    NeedToRest,
    InvestigateExpired,
    FacingSkater { angle: f32, distance: f32 },
    PhotographerTakingPictures,
    PhotographerReviewingPictures,
}

/// Interface calls whose retained owners are supplied by the concrete session
/// backend. A target lookup returning None is native absence, not an error
/// suppression. Guard/chase groups retain their existing source predicates;
/// their external facts are read only for the selected condition.
pub(crate) trait ConditionServices {
    fn protectee_position(&mut self, target:TargetableId)->Result<Option<[f32;4]>,String>{
        Err(format!("IProtectee position owner absent: {target:?}"))
    }
    fn guard_condition(&mut self, name: &str, reason: Option<i32>, entity: &PedestrianConstruction) -> Result<bool, String>;
    fn chase_condition(&mut self, name: &str, entity: &PedestrianConstruction) -> Result<bool, String>;
    fn chasee_exists(&mut self, entity: &PedestrianConstruction) -> Result<bool, String>;
    fn target_position(&mut self, entity: &PedestrianConstruction, target: TargetableId) -> Result<Option<[f32; 4]>, String>;
    fn has_plugin(&mut self, entity: &PedestrianConstruction) -> Result<bool, String> {
        Ok(entity.plugin.is_some())
    }
    fn has_road_target(&mut self, entity: &PedestrianConstruction) -> Result<bool, String> {
        Ok(entity.target_iterators.road.has_target())
    }
    fn entering_crosswalk(&mut self, entity: &PedestrianConstruction) -> Result<bool, String>;
    fn walk_sign_says_go(&mut self, entity: &PedestrianConstruction) -> Result<bool, String>;
    /// Native mpHandProp presence. Used only to enforce82e70040's assertion
    /// when the real-handprop flag is set; virtual holding needs no DMO.
    fn physical_hand_prop_exists(&mut self, entity: &PedestrianConstruction) -> Result<bool, String> {
        Ok(entity.held_hand_prop.is_some())
    }
    /// Patrol-zone validity is tested before resolving its entity of interest.
    fn interest_in_zone_boundary_or_buffer(&mut self, entity: &PedestrianConstruction) -> Result<bool, String>;
    fn interest_in_maintain_radius(&mut self, entity: &PedestrianConstruction) -> Result<bool, String>;
    /// The local skater's `PhysOut_SystemReckoning::mCentreOfMass`.
    fn local_skater_centre_of_mass(&mut self) -> Result<[f32; 4], String>;
}

pub(crate) struct ConditionFrame<'a, S, N> {
    pub(crate) entity: &'a PedestrianConstruction,
    pub(crate) services: &'a mut S,
    pub(crate) numeric: &'a mut N,
}

pub(crate) struct PedestrianConditions {
    entries: Vec<Condition>,
}
fn want(index: u16) -> Result<usize, String> {
    let value = attribute(index, "want")?.ok_or("condition requires want")?;
    want_type(value).ok_or_else(|| format!("invalid condition want {value}"))
}
fn scalar(index: u16, key: &str, default: f32) -> Result<f32, String> {
    attribute(index, key)?.map_or(Ok(default), |s|
        s.parse().map_err(|_| format!("invalid condition {key}={s}")))
}
impl PedestrianConditions {
    /// Uses the exported declarations and original occurrence constructor order.
    pub(crate) fn new() -> Result<Self, String> {
        let mut entries = Vec::with_capacity(data::CONDITIONS.len());
        for (index, declaration) in data::CONDITIONS.iter().enumerate() {
            let index = index as u16;
            if let Some(common) = CommonCondition::bind(index)? {
                entries.push(Condition::Common(common));
                continue;
            }
            let name = declaration.name;
            let entry = match name {
                "NeedsToBeginColliding" | "IsPedestrianColliding" | "IsOnRoad" | "IsOnIntersection" => Condition::Entity(name),
                "IsPatrolling" | "HasDrawnTazer" | "HasLineOfSightToTazeTarget"
                | "CanTazeUnreachableTargets" | "HasTakeDownTargetable"
                | "CanAttemptTakeDownTargetable" | "TakeDownAttemptSuccessful"
                | "TakeDownAttemptFailed" => Condition::Guard { name, reason: None },
                "IsEndChaseReason" => {
                    let text = attribute(index, "reason")?.ok_or("end chase condition requires reason")?;
                    let reason = living_world_guard_graph::binding::end_chase_reason(text)
                        .ok_or_else(|| format!("invalid condition end reason {text}"))?;
                    Condition::Guard { name, reason: Some(reason) }
                }
                "CanNewChaseStart" | "CanChaseeAddNewChaser" | "IsChaserEndingChase"
                | "ChaseeIsWipedOut" | "ChaseeHasProtector" | "IsPrimaryChaser"
                | "ChasersAreScared" | "CanSeeChasee" | "CannotReachChaseTarget"
                | "ChaserShouldBlock" | "ChaserShouldGiveUpDueToTakedowns"
                | "ChaseeEscaped" | "ChaseeCloseEnoughToSatisfyAggressiveChase"
                | "PatrolZoneMaintainInterestTimerActive" | "PatrolZoneOwnerIsInOwnZone"
                | "PatrolZoneOwnerIsInOwnZoneBoundaryOrBuffer" => Condition::Chase(name),
                "HasPlugin" => Condition::HasPlugin,
                "IsTargetingWaypoint" => Condition::TargetingWaypoint,
                "IsChasing" => Condition::Chasing,
                "HasRoadWanderTarget" => Condition::RoadTarget,
                "IsEnteringCrossWalk" => Condition::EnteringCrosswalk,
                "WalkSignSaysGo" => Condition::WalkSign,
                "IsPluginCollisionOverride" => Condition::PluginCollisionOverride,
                "HasHandProp" => Condition::HasHandProp,
                "CanAttackThrowHandProp" => Condition::AttackThrow,
                "IsHoldingSpecificHandProp" => Condition::HandProp(
                    skate2_formats::attrib::string_hash64(attribute(index, "handprop")?.unwrap_or(""))),
                //826efb70: conversion to radians precedes half-angle multiply.
                "IsFacingWantTarget" => Condition::Facing {
                    want: want(index)?,
                    angle: (scalar(index, "angle", 30.)? * 0.017453292) * 0.5,
                    distance: scalar(index, "distance", 0.)?,
                },
                "DistanceToWantTarget" => {
                    let section = declaration.attributes;
                    let fields = data::ATTRIBUTES.get(usize::from(section.index)..usize::from(section.index)+usize::from(section.count))
                        .ok_or("distance condition attribute section")?;
                    let mut comparison = comparison_from_attributes(name, fields).map_err(|e| e.to_string())?;
                    //826f2120 squares the constructor's reference, preserving abs/op.
                    comparison.reference *= comparison.reference;
                    Condition::Distance { want: want(index)?, comparison }
                }
                "IsWantTargetWithinAlertDistance" => Condition::AlertDistance(want(index)?),
                "EntityOfInterestIsInZoneBoundaryOrBuffer" => Condition::InterestInBoundary,
                "EntityOfInterestIsInMaintainInterestRadius" => Condition::InterestInRadius,
                "NeedToRest" => Condition::NeedToRest,
                "InvestigateTimeExceeded" => Condition::InvestigateExpired,
                _ => return Err(format!("unbound native pedestrian condition {index}: {name}")),
            };
            entries.push(entry);
        }
        Ok(Self { entries })
    }

    /// `HCDelegate_marquee::Init` 0x82927698 and `HCDelegate_Photographer::Init` 0x829279e0:
    /// the delegate's condition occurrences in declaration order.
    pub(crate) fn for_delegate(delegate: super::AiDelegate) -> Result<Self, String> {
        use super::{AiDelegate, marquee, marquee_data};
        let (declarations, attributes) = match delegate {
            AiDelegate::Pedestrian => return Self::new(),
            AiDelegate::BigBlack => (&super::big_black_data::big_black::CONDITIONS[..],
                &super::big_black_data::big_black::ATTRIBUTES[..]),
            AiDelegate::Marquee => (&marquee_data::marquee::CONDITIONS[..], &marquee_data::marquee::ATTRIBUTES[..]),
            AiDelegate::Photographer => (&marquee_data::photographer::CONDITIONS[..],
                &marquee_data::photographer::ATTRIBUTES[..]),
        };
        let mut entries = Vec::with_capacity(declarations.len());
        for declaration in declarations {
            let fields = marquee::attributes(attributes, declaration.attributes)?;
            entries.push(match declaration.name {
                "NeedsToBeginColliding" | "IsPedestrianColliding" => Condition::Entity(declaration.name),
                //826efab0: FOVAngle defaults to 0.8 and Distance to 0.
                "IsFacingSkater" => Condition::FacingSkater {
                    angle: marquee::float(fields, "FOVAngle", f32::from_bits(0x3f4c_cccd))?,
                    distance: marquee::float(fields, "Distance", 0.)?,
                },
                "WantsToProtectProtectee" | "IsProtectingProtectee" | "IsProtecteeUnderThreat"
                | "NeedsToInterceptThreat" | "NeedsToBlockThreat" => Condition::BigBlack(declaration.name),
                "DistanceFromProtectee" => Condition::DistanceFromProtectee(
                    comparison_from_attributes(declaration.name,fields).map_err(|e|e.to_string())?),
                "HasTakeDownTargetable" | "CanAttemptTakeDownTargetable" | "TakeDownAttemptSuccessful"
                | "TakeDownAttemptFailed" => Condition::Guard {name:declaration.name,reason:None},
                "HasMonitoredIntent" => Condition::Common(CommonCondition::MonitoredIntent(
                    skate2_formats::hash::sk8_hash_value_32(fields.iter().find(|a|a.key=="intentName")
                        .ok_or("BigBlack intent name absent")?.value.as_bytes()))),
                "SimpleTimerExpired" => Condition::Common(CommonCondition::Timer(
                    crate::living_world_pedestrian_state::timer_type(fields.iter().find(|a|a.key=="timerName")
                        .ok_or("BigBlack timer name absent")?.value).ok_or("BigBlack timer invalid")?)),
                "HasSpecificWantThatNeedsToBeAddressed" => Condition::Common(CommonCondition::Want(
                    want_type(fields.iter().find(|a|a.key=="want").ok_or("BigBlack want absent")?.value)
                        .ok_or("BigBlack want invalid")?)),
                "IsPhotographerTakingPictures" => Condition::PhotographerTakingPictures,
                "IsPhotographerReviewingPictures" => Condition::PhotographerReviewingPictures,
                name => return Err(format!("unbound native marquee condition {name}")),
            });
        }
        Ok(Self { entries })
    }

    pub(crate) fn evaluate<S: ConditionServices, N: ReckoningNumeric>(
        &self, index: u16, frame: &mut ConditionFrame<'_, S, N>,
    ) -> Result<bool, String> where N::Error: std::fmt::Debug {
        let condition = self.entries.get(usize::from(index)).ok_or("pedestrian condition index")?;
        let entity = frame.entity;
        let state = entity.state.as_ref().ok_or("pedestrian condition requires AI state")?;
        Ok(match condition {
            Condition::Common(common) => common.evaluate(state, |packet| {
                Ok::<_, String>(entity.intent_queue.as_ref().ok_or("monitored intent requires queue")?.packet(packet)?.in_queue)
            })?,
            Condition::BigBlack(name) => {
                let data=entity.big_black.as_ref().ok_or("BigBlack condition owner absent")?;
                match *name {
                    "WantsToProtectProtectee"=>data.wants.is_valid(),
                    "IsProtectingProtectee"=>data.protectee.is_valid(),
                    "IsProtecteeUnderThreat"=>match data.threats.first(){
                        Some(threat)=>frame.services.target_position(entity,threat.id)?.is_some(),None=>false},
                    "NeedsToInterceptThreat"=>data.threats.first().is_some_and(|t|matches!(t.action,2|3)),
                    "NeedsToBlockThreat"=>data.threats.first().is_some_and(|t|t.action==1),
                    _=>return Err("BigBlack predicate absent".into()),
                }
            },
            Condition::DistanceFromProtectee(compare)=>{
                let target=entity.big_black.as_ref().ok_or("BigBlack condition owner absent")?.wants;
                if let Some(position)=frame.services.protectee_position(target)? {
                    let own=entity.core.as_ref().ok_or("BigBlack core absent")?.transform[3];
                    let delta=crate::ai::vmx::sub(own,position);
                    crate::generated_action_load::conditions::compare(*compare,crate::ai::vmx::sqrt_guarded(crate::ai::vmx::dot3(delta,delta)))
                }else{false}
            },
            Condition::Entity(name) => conditions::entity_condition(name, entity)?.ok_or("entity condition dispatch mismatch")?,
            Condition::Guard { name, reason } => frame.services.guard_condition(name, *reason, entity)?,
            Condition::Chase(name) => frame.services.chase_condition(name, entity)?,
            Condition::HasPlugin => frame.services.has_plugin(entity)?,
            Condition::TargetingWaypoint => entity.current_waypoint.is_some(),
            Condition::Chasing => frame.services.chasee_exists(entity)?,
            Condition::RoadTarget => frame.services.has_road_target(entity)?,
            Condition::EnteringCrosswalk => frame.services.entering_crosswalk(entity)?,
            Condition::WalkSign => frame.services.walk_sign_says_go(entity)?,
            Condition::PluginCollisionOverride => state.override_external_collision,
            Condition::HasHandProp => has_hand_prop(entity, frame.services)?,
            Condition::AttackThrow => {
                if !has_hand_prop(entity, frame.services)? { false } else {
                    let attributes = entity.hand_prop_attributes.as_ref().ok_or("held hand prop requires attributes")?;
                    let field = attributes.try_cached_layout_field(62, 1).map_err(|e| e.to_string())?
                        .ok_or("attack throw requires cached hand prop layout")?;
                    field.bytes()[0] != 0
                }
            }
            Condition::HandProp(key) => has_hand_prop(entity, frame.services)?
                && entity.hand_prop_attributes.as_ref().and_then(|a| a.collection_key()).unwrap_or(0) == *key,
            Condition::InterestInBoundary => frame.services.interest_in_zone_boundary_or_buffer(entity)?,
            Condition::InterestInRadius => frame.services.interest_in_maintain_radius(entity)?,
            Condition::InvestigateExpired => state.timer(1) <= 0.,
            Condition::FacingSkater { angle, distance } => {
                let centre = frame.services.local_skater_centre_of_mass()?;
                let core = entity.core.as_ref().ok_or("facing skater condition requires entity core")?;
                super::marquee::is_facing_skater(&core.transform, centre, *angle, *distance)
            }
            //82707a10/82707aa8 read state+ca8 bits 40 and 20.
            Condition::PhotographerTakingPictures => state.flags[0] & 0x40 != 0,
            Condition::PhotographerReviewingPictures => state.flags[0] & 0x20 != 0,
            Condition::NeedToRest => {
                //IChaser slots37/38/41 resolve82e6fc78/82e89a30/82e6fd40.
                let limit = chase_scalar(entity, 0x4686_fec6_8a60_bd69)?;
                let exhausted = limit < state.exhaustion;
                exhausted || (state.flags[1] & 2 != 0 && !(state.timer(4) <= 0.))
            }
            Condition::Distance { want, comparison } => {
                let Some(target) = frame.services.target_position(entity,state.wants[*want].target)? else { return Ok(false); };
                let position = entity.core.as_ref().ok_or("distance condition requires entity core")?.transform[3];
                let delta = std::array::from_fn(|i| target[i] - position[i]);
                let squared = frame.numeric.dot3(delta, delta).map_err(numeric_error)?;
                crate::generated_action_load::conditions::compare(*comparison, squared)
            }
            Condition::AlertDistance(want) => {
                let Some(target) = frame.services.target_position(entity,state.wants[*want].target)? else { return Ok(true); };
                let distance = chase_scalar(entity, 0x3145_97ed_a6b9_21f2)?;
                let position = entity.core.as_ref().ok_or("alert condition requires entity core")?.transform[3];
                let delta = std::array::from_fn(|i| position[i] - target[i]);
                frame.numeric.dot3(delta, delta).map_err(numeric_error)? - distance * distance < 0.
            }
            Condition::Facing { want, angle, distance } => {
                let Some(mut target) = frame.services.target_position(entity,state.wants[*want].target)? else { return Ok(false); };
                let core = entity.core.as_ref().ok_or("facing condition requires entity core")?;
                let mut position = core.transform[3];
                //Raw8270c354..390 clears both Y lanes before subtraction.
                target[1] = 0.; position[1] = 0.;
                let delta = std::array::from_fn(|i| target[i] - position[i]);
                if *distance != 0. && frame.numeric.dot3(delta, delta).map_err(numeric_error)? - distance * distance > 0. {
                    return Ok(false);
                }
                let facing = [core.transform[2][0], 0., core.transform[2][2], 0.];
                let actual = try_signed_angle(facing, delta, [0., 1., 0., 0.], frame.numeric).map_err(numeric_error)?;
                //8270c468 branches on ordered <=; unordered falls into true.
                actual < *angle || !(actual <= 6.2831855 - angle)
            }
        })
    }
}
fn numeric_error(error: impl std::fmt::Debug) -> String { format!("pedestrian condition numeric: {error:?}") }
fn has_hand_prop<S: ConditionServices>(entity: &PedestrianConstruction, services: &mut S) -> Result<bool, String> {
    let state = entity.state.as_ref().ok_or("handprop condition requires state")?;
    if state.flags[2] & 2 != 0 {
        if !services.physical_hand_prop_exists(entity)? { return Err("native HasHandProp real-handprop flag requires mpHandProp".into()); }
        return Ok(true);
    }
    Ok(state.flags[3] & 2 != 0)
}
fn chase_scalar(entity: &PedestrianConstruction, key: u64) -> Result<f32, String> {
    let chase = &entity.resources.as_ref().ok_or("condition requires resources")?.chase;
    if chase.is_null() { return Err("condition requires native chase collection".into()); }
    Ok(f32::from_bits(chase.try_scalar_word(key, 0).map_err(|e| e.to_string())?.unwrap_or(0)))
}

