//! Original guard/chaser graph leaves. Native mask/expression dispatch remains
//! with PedestrianGraph; services must call real entity/nav/event owners.
#[path = "living_world_guard_graph_binding.rs"]
pub(crate) mod binding;
use crate::{
    living_world_chase::TargetableId,
    living_world_pedestrian_state::PedestrianStateStorage,
};

pub(crate) struct GuardConditionFacts {
    pub(crate) current_waypoint_exists: bool,
    pub(crate) take_down_sink_exists: bool,
    pub(crate) take_down_sink_wiped_out: bool,
    pub(crate) disable_takedowns: bool,
    pub(crate) take_down_type_exists: bool,
    pub(crate) can_taze_unreachable: bool,
    pub(crate) chasee_group_end_reason: Option<i32>,
}
/// Named ActivationFuncs only; None means this owner does not handle the name.
pub(crate) fn state_condition(
    name: &str,
    state: &PedestrianStateStorage,
    facts: &GuardConditionFacts,
    specific_reason: Option<i32>,
) -> Result<Option<bool>, String> {
    Ok(Some(match name {
        "IsPatrolling" => state.flags[0] & 8 != 0 && facts.current_waypoint_exists, // 826f0dd8
        "HasDrawnTazer" => state.flags[4] & 0x80 != 0 && state.tazer_state == 2,    // 826f1f28
        "HasLineOfSightToTazeTarget" => state.flags[4] & 0x40 != 0,                 // 826f22a8
        "CanTazeUnreachableTargets" => facts.can_taze_unreachable,                  // 826f22e8
        "HasTakeDownTargetable" => facts.take_down_sink_exists,                     // 8270c8e0
        "CanAttemptTakeDownTargetable" => {
            !facts.disable_takedowns
                && facts.take_down_sink_exists
                && !facts.take_down_sink_wiped_out
                && facts.take_down_type_exists
        } // 8270c960
        "TakeDownAttemptSuccessful" => state.takedown_status == Some(2),            // 826f0c40
        "TakeDownAttemptFailed" => state.takedown_status == Some(3),                // 826f0c98
        "IsEndChaseReason" => {
            // Pedestrian::GetEndChaseReason82e6fdd0 favors a forced group end.
            let reason = match facts.chasee_group_end_reason {
                Some(group) if group != -1 => group,
                Some(_) => state.chaser.end_reason,
                None => -1,
            };
            reason == specific_reason.ok_or("IsEndChaseReason missing native specific result")?
        }
        _ => return Ok(None),
    }))
}

/// Parsed native constructor arguments. No blanket defaults: constructor
/// dispatch must supply authored/verified values for each occurrence.
pub(crate) enum GuardBehaviour {
    UnsetWant {
        want: usize,
    },
    KnowAboutWantTarget { want: usize },
    UnsetWantAndSuppressMood { want: usize, time: f32 },
    ActivateRelatedWant {
        original: usize,
        related: usize,
        deactivate_original: bool,
    },
    SetAlertTimer {
        authored_alert_time: f32,
    },
    SetPatrolInterestTimer {
        authored_interest_time: f32,
    },
    ChaserEndChase {
        reason: i32,
    },
    ChaserGroupEndChase {
        reason: i32,
        aggressive_capture_reason: i32,
    },
    ChannelWarnWantTarget {
        want: usize,
        deactivate_want: bool,
        warn_interest_time: f32,
    },
    RegisterTazer {
        want: usize,
    },
    UnregisterTazerOnEnd,
    EndTaze,
    NewChasee,
    EndChase,
    MoodResetAboutChasee,
    SuppressMoodAboutChasee {
        time: f32,
    },
    ReturnToPatrolZone,
    SetAltTargetToChaseePosition,
    KnowAboutChasee,
    BlockChasee {
        at_target_distance_squared: f32,
        slow_speed: f32,
    },
    PursueChasee,
}

/// These calls are native interface boundaries, not successful placeholders.
/// Implementors propagate unavailable consumers as errors until integrated.
pub(crate) trait GuardGraphServices {
    fn targetable_exists(&mut self, target: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<bool, String>;
    fn chasee_exists(&mut self, target: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<bool, String>;
    fn is_local_player(&mut self, target: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<bool, String>;
    fn is_local_actor(&mut self, target: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<bool, String>;
    fn number_of_reactions(&mut self, target: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<i32, String>;
    fn entity_knowledge_suppressed(&self, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<bool,String>;
    fn channel(&mut self, name: &str, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn speech_event(&mut self, event: u32, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn chasee_primary_speech_event(
        &mut self,
        chasee: TargetableId,
        event: u32, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn chasee_remove_chaser(
        &mut self,
        chasee: TargetableId,
        chaser: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn chasee_add_chaser(
        &mut self,
        chasee: TargetableId,
        chaser: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<bool, String>;
    fn group_end_chase(&mut self, chasee: TargetableId, reason: i32, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn show_chase_end_message(&mut self, captured: bool, escaped: bool, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn set_nav_desired_speed(&mut self, speed: f32, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn clear_chasing_flag(&mut self, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn set_chasing_flag(&mut self, chasing: bool, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    /// Begin826ff0e0/826ff1a8: explicit-A* target mode, PedestrianChase
    /// targeting, Mesh navigation, clear entity6724 bit0x20, Seek(true).
    fn begin_chase_navigation(&mut self, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn chaser_position(&mut self, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<[f32; 3], String>;
    fn chaser_block_position(&mut self, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<[f32; 3], String>;
    fn max_chase_speed(&mut self, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<f32, String>;
    fn chasee_position(&mut self, chasee: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<[f32; 3], String>;
    fn recommended_chaser_position(
        &mut self,
        chasee: TargetableId,
        chaser: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<[f32; 3], String>;
    fn set_strafe_target(&mut self, target: [f32; 3], entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn clear_strafe_target(&mut self, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn target_explicit(&mut self, target: [f32; 3], native_mode: u32, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn set_returning_to_patrol_zone(&mut self, returning: bool, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn update_return_to_patrol_zone(&mut self, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn set_alt_target_to_chasee_position(&mut self, chasee: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<(), String>;
    fn register_warner(&mut self, id: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction)->Result<(),String>;
    fn register_tazer(&mut self, id: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction)->Result<(),String>;
    fn unregister_tazer(&mut self, id: TargetableId, entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction)->Result<(),String>;
}

pub(crate) struct GuardBehaviourContext<'a, S> {
    pub(crate) entity: &'a mut crate::living_world_pedestrian_entity::PedestrianConstruction,
    pub(crate) self_id: TargetableId,
    pub(crate) is_security_guard: bool,
    pub(crate) services: &'a mut S,
}
impl<S: GuardGraphServices> GuardBehaviourContext<'_, S> {
    fn want_target(&mut self, want: usize) -> Result<TargetableId, String> {
        self.entity.state.as_mut().expect("guard state checked at callback entry")
            .wants
            .get(want)
            .map(|w| w.target)
            .ok_or_else(|| format!("native want index {want} outside 15"))
    }
    fn set_chasee(&mut self, target: TargetableId) -> Result<(), String> {
        if self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.set_chasee(target) {
            self.entity.state.as_mut().expect("guard state checked at callback entry")
                .set_want_active(1, false)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    /// Pedestrian::SuppressMoodForATime82e98718 uses the SAME knowledge
    /// record as KnowAboutEntity, then fsel-maxes its mood restriction timer.
    fn suppress_mood(&mut self, target: TargetableId, time: f32) -> Result<(), String> {
        let exists = self.services.targetable_exists(target, self.entity)?;
        let suppressed = self.services.entity_knowledge_suppressed(self.entity)?;
        if let Some(entry) = self.entity.state.as_mut().expect("guard state checked at callback entry").know_about_entity(self.self_id, target, time, suppressed, exists)
            .map_err(|e| e.to_string())? {
            if !(entry.mood_restriction - time >= 0.0) { entry.mood_restriction = time; }
        }
        Ok(())
    }
    pub(crate) fn begin(&mut self, behaviour: &GuardBehaviour) -> Result<(), String> {
        if self.entity.state.is_none(){return Err("guard callback AI state absent".into());}
        match *behaviour {
            GuardBehaviour::KnowAboutWantTarget { want } => {
                //826edb30 does not gate on want.active.
                let target = self.want_target(want)?;
                let exists = self.services.targetable_exists(target, self.entity)?;
                let suppressed=self.services.entity_knowledge_suppressed(self.entity)?;
                self.entity.state.as_mut().expect("guard state checked at callback entry").know_about_entity(self.self_id, target, 30.,
                    suppressed, exists)
                    .map_err(|e| e.to_string())?;
            }
            GuardBehaviour::UnsetWantAndSuppressMood { want, time } => {
                //826ede20 tests active first and copies target before clearing.
                let entry = *self.entity.state.as_mut().expect("guard state checked at callback entry").wants.get(want).ok_or("invalid native want index")?;
                if entry.active {
                    self.entity.state.as_mut().expect("guard state checked at callback entry").set_want_active(want, false).map_err(|e| e.to_string())?;
                    self.suppress_mood(entry.target, time)?;
                }
            }
            // EndTaze::Begin826ef018 requests the out animation BEFORE
            // publishing tazer state3. Preserve a failed request as failure.
            GuardBehaviour::EndTaze => {
                self.services.channel("TazerOut", self.entity)?;
                self.entity.state.as_mut().expect("guard state checked at callback entry").tazer_state = 3;
            }
            GuardBehaviour::UnsetWant { want } => self.entity.state.as_mut().expect("guard state checked at callback entry")
                .set_want_active(want, false)
                .map_err(|e| e.to_string())?,
            GuardBehaviour::ActivateRelatedWant {
                original,
                related,
                deactivate_original,
            } => self.entity.state.as_mut().expect("guard state checked at callback entry")
                .activate_related_want(original, related, deactivate_original)
                .map_err(|e| e.to_string())?,
            GuardBehaviour::SetAlertTimer {
                authored_alert_time,
            } => self.entity.state.as_mut().expect("guard state checked at callback entry")
                .set_timer(7, authored_alert_time)
                .map_err(|e| e.to_string())?,
            GuardBehaviour::SetPatrolInterestTimer {
                authored_interest_time,
            } => self.entity.state.as_mut().expect("guard state checked at callback entry")
                .set_timer(15, authored_interest_time)
                .map_err(|e| e.to_string())?,
            GuardBehaviour::ChaserEndChase { reason } => {
                if self.services.chasee_exists(self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.chasee, self.entity)? {
                    self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.end_chase(reason);
                }
            }
            GuardBehaviour::ChaserGroupEndChase {
                reason,
                aggressive_capture_reason,
            } => {
                let target = self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.chasee;
                if self.services.chasee_exists(target, self.entity)? {
                    if reason == aggressive_capture_reason
                        && self.services.is_local_player(target, self.entity)?
                    {
                        self.services.show_chase_end_message(true, false, self.entity)?;
                    }
                    self.services.group_end_chase(target, reason, self.entity)?;
                }
            }
            GuardBehaviour::ChannelWarnWantTarget {
                want,
                deactivate_want,
                warn_interest_time,
            } => {
                let target = self.want_target(want)?;
                if self.services.targetable_exists(target, self.entity)? {
                    self.services.channel("WarnChannel", self.entity)?;
                    self.entity.state.as_mut().expect("guard state checked at callback entry")
                        .set_timer(36, warn_interest_time)
                        .map_err(|e| e.to_string())?;
                    let event = if self.services.number_of_reactions(target, self.entity)? != 0 {
                        60
                    } else {
                        59
                    };
                    self.services.speech_event(event, self.entity)?;
                    if self.is_security_guard && self.services.is_local_actor(target, self.entity)? {
                        self.services.register_warner(self.self_id, self.entity)?;
                    }
                } else if deactivate_want {
                    self.entity.state.as_mut().expect("guard state checked at callback entry")
                        .set_want_active(want, false)
                        .map_err(|e| e.to_string())?;
                }
            }
            GuardBehaviour::RegisterTazer { want } => {
                let target = self.want_target(want)?;
                if self.services.targetable_exists(target, self.entity)?
                    && self.services.is_local_player(target, self.entity)?
                {
                    self.services.register_tazer(self.self_id, self.entity)?;
                }
            }
            GuardBehaviour::NewChasee => {
                let current = self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.chasee;
                if self.services.chasee_exists(current, self.entity)? {
                    self.services.chasee_primary_speech_event(current, 15, self.entity)?;
                }
                self.entity.state.as_mut().expect("guard state checked at callback entry").takedown_count = Some(0);
                let next = self.want_target(1)?;
                if self.services.chasee_exists(next, self.entity)? {
                    if self.services.chasee_exists(current, self.entity)? {
                        // Native827090e8 removes from the resolved WANT target,
                        // not from the previous chasee. Preserve that receiver.
                        self.services.chasee_remove_chaser(next, self.self_id, self.entity)?;
                        self.set_chasee(TargetableId::INVALID)?;
                    }
                    if self.services.chasee_add_chaser(next, self.self_id, self.entity)? {
                        self.set_chasee(next)?;
                    }
                }
            }
            GuardBehaviour::EndChase => {
                let target = self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.chasee;
                if self.services.chasee_exists(target, self.entity)? {
                    self.services.chasee_remove_chaser(target, self.self_id, self.entity)?;
                }
                self.set_chasee(TargetableId::INVALID)?;
                self.services.set_nav_desired_speed(0.0, self.entity)?;
                self.services.clear_chasing_flag(self.entity)?;
            }
            GuardBehaviour::MoodResetAboutChasee => {
                let target = self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.chasee;
                if self.services.chasee_exists(target, self.entity)? {
                    self.entity.state.as_mut().expect("guard state checked at callback entry").mood_reset_by_target(target);
                }
            }
            GuardBehaviour::SuppressMoodAboutChasee { time } => {
                let target = self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.chasee;
                if self.services.chasee_exists(target, self.entity)? {
                    self.suppress_mood(target, time)?;
                }
            }
            GuardBehaviour::ReturnToPatrolZone => {
                self.services.set_returning_to_patrol_zone(true, self.entity)?
            }
            GuardBehaviour::BlockChasee { .. } | GuardBehaviour::PursueChasee => {
                self.services.begin_chase_navigation(self.entity)?
            }
            GuardBehaviour::UnregisterTazerOnEnd
            | GuardBehaviour::SetAltTargetToChaseePosition
            | GuardBehaviour::KnowAboutChasee => {}
        }
        Ok(())
    }
    pub(crate) fn update(&mut self, behaviour: &GuardBehaviour, dt: f32) -> Result<(), String> {
        if self.entity.state.is_none(){return Err("guard callback AI state absent".into());}
        match *behaviour {
            GuardBehaviour::ChannelWarnWantTarget {
                want,
                deactivate_want,
                ..
            } => {
                if deactivate_want && self.entity.state.as_mut().expect("guard state checked at callback entry").timer(36) <= 0.0 {
                    self.entity.state.as_mut().expect("guard state checked at callback entry")
                        .set_want_active(want, false)
                        .map_err(|e| e.to_string())?;
                }
            }
            GuardBehaviour::ReturnToPatrolZone => self.services.update_return_to_patrol_zone(self.entity)?,
            GuardBehaviour::SetAltTargetToChaseePosition => {
                let target = self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.chasee;
                if self.services.chasee_exists(target, self.entity)? {
                    self.services.set_alt_target_to_chasee_position(target, self.entity)?;
                }
            }
            GuardBehaviour::KnowAboutChasee => {
                let target = self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.chasee;
                if self.services.chasee_exists(target, self.entity)? {
                    let exists = self.services.targetable_exists(target, self.entity)?;
                    let suppressed=self.services.entity_knowledge_suppressed(self.entity)?;
                    self.entity.state.as_mut().expect("guard state checked at callback entry")
                        .know_about_entity(
                            self.self_id,
                            target,
                            30.0,
                            suppressed,
                            exists,
                        )
                        .map_err(|e| e.to_string())?;
                }
            }
            GuardBehaviour::BlockChasee {
                at_target_distance_squared,
                slow_speed,
            } => {
                let chasee = self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.chasee;
                if self.services.chasee_exists(chasee, self.entity)? {
                    let block = self.services.chaser_block_position(self.entity)?;
                    let position = self.services.chaser_position(self.entity)?;
                    let target = self.services.chasee_position(chasee, self.entity)?;
                    self.services.set_strafe_target(target, self.entity)?;
                    let d = [
                        position[0] - block[0],
                        position[1] - block[1],
                        position[2] - block[2],
                    ];
                    let distance_squared = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
                    if at_target_distance_squared > distance_squared {
                        self.services.set_chasing_flag(false, self.entity)?;
                        self.services.set_nav_desired_speed(0.0, self.entity)?;
                        self.services.target_explicit(block, 2, self.entity)?;
                        let remaining = self.entity.state.as_mut().expect("guard state checked at callback entry").exhaustion - dt;
                        self.entity.state.as_mut().expect("guard state checked at callback entry").exhaustion = if remaining >= 0.0 { remaining } else { 0.0 };
                    } else {
                        self.services.set_chasing_flag(true, self.entity)?;
                        self.services.set_nav_desired_speed(slow_speed, self.entity)?;
                        self.services.target_explicit(block, 1, self.entity)?;
                    }
                }
            }
            GuardBehaviour::PursueChasee => {
                let chasee = self.entity.state.as_mut().expect("guard state checked at callback entry").chaser.chasee;
                if self.services.chasee_exists(chasee, self.entity)? {
                    let target = self
                        .services
                        .recommended_chaser_position(chasee, self.self_id, self.entity)?;
                    self.services.set_chasing_flag(true, self.entity)?;
                    let speed = self.services.max_chase_speed(self.entity)?;
                    self.services.set_nav_desired_speed(speed, self.entity)?;
                    self.entity.state.as_mut().expect("guard state checked at callback entry").exhaustion += dt;
                    self.services.target_explicit(target, 1, self.entity)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub(crate) fn end(&mut self, behaviour: &GuardBehaviour) -> Result<(), String> {
        if self.entity.state.is_none(){return Err("guard callback AI state absent".into());}
        match behaviour {
            GuardBehaviour::UnregisterTazerOnEnd => self.services.unregister_tazer(self.self_id, self.entity)?,
            GuardBehaviour::ReturnToPatrolZone => {
                self.services.set_returning_to_patrol_zone(false, self.entity)?
            }
            GuardBehaviour::BlockChasee { .. } => self.services.clear_strafe_target(self.entity)?,
            _ => {}
        }
        Ok(())
    }
}
