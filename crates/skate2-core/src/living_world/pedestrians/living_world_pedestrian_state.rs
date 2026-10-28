//! Typed retained PedestrianAIState data, shared by the original graph callbacks.
//! Ctor 82e92668, Reset 82e92778. Uninitialized native fields are Options until
//! their real producer writes them; this is not a replacement AI policy.
use crate::living_world_chase::{ChaserState, TargetableId};
use std::collections::BTreeMap;

#[path = "pedestrian_state_mood.rs"]
mod mood;

#[derive(Clone, Copy, Debug)]
pub(crate) struct WantEntry {
    pub(crate) target: TargetableId,
    pub(crate) active: bool,
    pub(crate) explicit: bool,
}
impl Default for WantEntry {
    fn default() -> Self {
        Self {
            target: TargetableId::INVALID,
            active: false,
            explicit: false,
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct EntityKnowledge {
    pub(crate) target: TargetableId,
    pub(crate) position: [f32; 3],
    pub(crate) facing: [f32; 3],
    pub(crate) velocity: [f32; 3],
    pub(crate) attention_remaining: f32,
    pub(crate) total_attended: f32,
    pub(crate) mood_restriction: f32,
    pub(crate) reaction_count: u32,
    pub(crate) just_added: bool,
    pub(crate) visible: bool,
}
/// Supplied by the real targetable entity and PedestrianPerception owner.
pub(crate) struct KnowledgeObservation {
    pub(crate) visible: bool,
    pub(crate) position: [f32; 3],
    pub(crate) facing: [f32; 3],
    pub(crate) velocity: [f32; 3],
}
impl EntityKnowledge {
    pub(crate) fn perception_flags(&self) -> u8 {
        (u8::from(self.just_added) << 7) | (u8::from(self.visible) << 6)
    }
}
#[derive(Clone, Debug)]
pub(crate) struct MoodEntry {
    pub(crate) probability_timers: Vec<(u32, f32)>,
    pub(crate) position: [f32; 3],
    pub(crate) perpetrator: TargetableId,
    pub(crate) victim: TargetableId,
    pub(crate) event_type: i32,
    pub(crate) exposure: f32,
    pub(crate) occurrences: u32,
    pub(crate) cooldown: f32,
    pub(crate) since_refresh: f32,
}
#[derive(Debug)]
pub(crate) enum StateError {
    WantIndex(usize),
    TimerCapacity,
    InvalidTarget,
}
impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pedestrian state: {self:?}")
    }
}
impl std::error::Error for StateError {}

#[derive(Debug)]
pub(crate) struct PedestrianStateStorage {
    pub(crate) wants: [WantEntry; 15],
    pub(crate) knowledge: Vec<EntityKnowledge>,
    pub(crate) moods: Vec<MoodEntry>,
    pub(crate) chaser: ChaserState,
    pub(crate) attractor: TargetableId,
    pub(crate) exhaustion: f32,
    pub(crate) flags: [u8; 5],
    pub(crate) take_down_sink: TargetableId,
    pub(crate) take_down_target: [f32; 3],
    pub(crate) fixed_target: Option<[f32; 3]>,
    pub(crate) hand_prop_throw_target: Option<[f32; 4]>,
    pub(crate) dmo_shift: Option<[f32; 3]>,
    pub(crate) collider: TargetableId,
    pub(crate) highest_collision_type: i32,
    pub(crate) collision_flags: u8,
    /// Entity mSkaterCollisionFlags+1a3c; grouped here as mutable gameplay state.
    /// Pedestrian ctor82e97d38 clears it; ResetState does not.
    pub(crate) skater_collision_flags: u32,
    pub(crate) override_external_collision: bool,
    pub(crate) override_external_avoid: bool,
    pub(crate) takedown_status: Option<i32>,
    pub(crate) take_down_type: Option<u32>,
    pub(crate) honking_vehicle_id: i32,
    pub(crate) slave_id: i32,
    pub(crate) master_id: i32,
    pub(crate) lead_pedestrian_id: u32,
    pub(crate) takedown_count: Option<u32>,
    pub(crate) hand_prop_throw_velocity: Option<f32>,
    pub(crate) hand_prop_release_offset: f32,
    pub(crate) hand_prop_action: i32,
    pub(crate) tazer_state: i32,
    pub(crate) taze_result_target: i32,
    timers: BTreeMap<u32, f32>,
    pub(crate) plugin_timers: BTreeMap<u32, f32>,
    /// IDs of actual monitored intents; owning intent packets stay with their owner.
    pub(crate) intent_monitors: BTreeMap<u32, u64>,
}
impl Default for PedestrianStateStorage {
    fn default() -> Self {
        let mut state = Self {
            wants: [WantEntry::default(); 15],
            knowledge: Vec::with_capacity(5),
            moods: Vec::with_capacity(10),
            chaser: ChaserState::default(),
            attractor: TargetableId { kind: 0, id: -1 },
            exhaustion: 0.0,
            flags: [0; 5],
            take_down_sink: TargetableId::INVALID,
            take_down_target: [0.0, 500.0, 0.0],
            fixed_target: None,
            hand_prop_throw_target: None,
            // MemStore.Init82a6de38 / FreeBlock82a6df68 clear this accumulator.
            dmo_shift: Some([0.;3]),
            collider: TargetableId::INVALID,
            highest_collision_type: 0,
            collision_flags: 0,
            skater_collision_flags: 0,
            override_external_collision: false,
            override_external_avoid: false,
            takedown_status: None,
            take_down_type: None,
            honking_vehicle_id: -1,
            slave_id: -1,
            master_id: -1,
            lead_pedestrian_id: 0,
            // State MemStore Init/FreeBlock zero storage before ctor; Reset
            // deliberately does not rewrite this cumulative collision count.
            takedown_count: Some(0),
            hand_prop_throw_velocity: None,
            hand_prop_release_offset: 0.0,
            hand_prop_action: -1,
            tazer_state: 0,
            taze_result_target: -1,
            timers: BTreeMap::new(),
            plugin_timers: BTreeMap::new(),
            intent_monitors: BTreeMap::new(),
        };
        state.reset();
        state
    }
}
impl PedestrianStateStorage {
    /// Reset deliberately preserves targets, explicit bits, timers, plugin
    /// timers, chaser target and all native fields not written by 82e92778.
    pub(crate) fn reset(&mut self) {
        self.collider = TargetableId::INVALID;
        self.highest_collision_type = 0;
        self.collision_flags = (self.collision_flags & 0x0f) | 0x10;
        self.take_down_type = None;
        self.take_down_target = [0.0, 500.0, 0.0];
        self.exhaustion = 0.0;
        self.flags[..4].fill(0);
        self.flags[4] &= 0x1f;
        self.chaser.end_reason = -1;
        self.taze_result_target = -1;
        self.tazer_state = 0;
        for want in &mut self.wants {
            want.active = false;
        }
        self.intent_monitors.clear();
        self.override_external_collision = false;
        self.override_external_avoid = false;
        self.hand_prop_release_offset = 0.0;
        self.honking_vehicle_id = -1;
        self.slave_id = -1;
        self.master_id = -1;
        self.lead_pedestrian_id = 0;
        self.hand_prop_action = -1;
        self.moods.clear();
        self.knowledge.clear();
    }
    /// SetWantActive 82e9e310, including the two distinct deactivation effects.
    pub(crate) fn set_want_active(&mut self, index: usize, active: bool) -> Result<(), StateError> {
        let entry = *self.wants.get(index).ok_or(StateError::WantIndex(index))?;
        if !active && entry.active {
            if entry.explicit {
                self.mood_reset_by_target(entry.target);
            } else if let Some(k) = self.knowledge.iter_mut().find(|k| k.target == entry.target) {
                k.reaction_count = k.reaction_count.wrapping_add(1);
            }
        }
        self.wants[index].active = active;
        Ok(())
    }
    /// ActivateRelatedWant 82e9e3c8 copies the full entry before activation.
    pub(crate) fn activate_related_want(
        &mut self,
        original: usize,
        related: usize,
        deactivate_original: bool,
    ) -> Result<(), StateError> {
        let entry = *self
            .wants
            .get(original)
            .ok_or(StateError::WantIndex(original))?;
        *self
            .wants
            .get_mut(related)
            .ok_or(StateError::WantIndex(related))? = entry;
        self.set_want_active(related, true)?;
        if deactivate_original {
            self.wants[original].active = false;
        }
        Ok(())
    }
    pub(crate) fn has_unaddressed_want(&self) -> bool {
        self.wants.iter().any(|w| w.active)
    }
    /// MoodResetByTargetableId 82e9e1a0 uses stable removal, unlike ordinary
    /// RemoveEntityKnowledge which swaps the last entry into the erased slot.
    pub(crate) fn mood_reset_by_target(&mut self, target: TargetableId) {
        self.moods
            .retain(|m| m.perpetrator != target && m.victim != target);
        self.knowledge.retain(|k| k.target != target);
    }
    /// NewEntityKnowledge 82e92898; greatest attended time evicted, first tie wins.
    pub(crate) fn new_entity_knowledge(
        &mut self,
        target: TargetableId,
        how_long: f32,
    ) -> Result<&mut EntityKnowledge, StateError> {
        if !target.is_valid() {
            return Err(StateError::InvalidTarget);
        }
        if self.knowledge.len() >= 5 {
            let mut oldest = 0;
            for i in 1..self.knowledge.len() {
                if self.knowledge[i].total_attended > self.knowledge[oldest].total_attended {
                    oldest = i;
                }
            }
            self.knowledge.swap_remove(oldest);
        }
        self.knowledge.push(EntityKnowledge {
            target,
            position: [0.0; 3],
            facing: [0.0; 3],
            velocity: [0.0; 3],
            attention_remaining: how_long,
            total_attended: 0.0,
            mood_restriction: 0.0,
            reaction_count: 0,
            just_added: true,
            visible: false,
        });
        Ok(self.knowledge.last_mut().expect("entry just appended"))
    }
    /// Caller supplies real target/interface existence and the authored cvar.
    pub(crate) fn know_about_entity(
        &mut self,
        self_id: TargetableId,
        target: TargetableId,
        how_long: f32,
        suppressed: bool,
        targetable_exists: bool,
    ) -> Result<Option<&mut EntityKnowledge>, StateError> {
        if suppressed || target == self_id || !target.is_valid() || !targetable_exists {
            return Ok(None);
        }
        if let Some(index) = self.knowledge.iter().position(|k| k.target == target) {
            let k = &mut self.knowledge[index];
            if !(k.attention_remaining - how_long >= 0.0) {
                k.attention_remaining = how_long;
            }
            return Ok(Some(k));
        }
        self.new_entity_knowledge(target, how_long).map(Some)
    }
    pub(crate) fn timer(&self, kind: u32) -> f32 {
        self.timers.get(&kind).copied().unwrap_or(0.0)
    }
    pub(crate) fn set_timer(&mut self, kind: u32, value: f32) -> Result<(), StateError> {
        if value > 0.0 {
            if !self.timers.contains_key(&kind) && self.timers.len() == 14 {
                return Err(StateError::TimerCapacity);
            }
            self.timers.insert(kind, value);
        } else {
            self.timers.remove(&kind);
        }
        Ok(())
    }
    /// 82e89e90 fsel selects by OLD timer sign, so a crossing timer stays
    /// negative until the next update. Do not clamp the result to zero.
    pub(crate) fn update_timers(&mut self, dt: f32) {
        self.timers.retain(|_, t| {
            *t = if *t >= 0.0 { *t - dt } else { 0.0 };
            *t != 0.0
        });
        for timer in self.plugin_timers.values_mut() {
            if *timer >= -30.0 {
                *timer -= dt;
            }
        }
    }
    pub(crate) fn plugin_timer_expired(&self, state_hash: u32) -> bool {
        self.plugin_timers
            .get(&state_hash)
            .is_none_or(|t| *t <= 0.0)
    }

    /// ResetPluginTimer 82e9a230. Raw PPC 82e9a274..298 compares the
    /// numeric key to the retained timer, rather than comparing two timers.
    /// Preserve this unusual eviction order, including eviction on replacement.
    pub(crate) fn reset_plugin_timer(&mut self, hash: u32, initial_time: f32) {
        if self.plugin_timers.len() >= 16 {
            let (&first_key, &first_time) =
                self.plugin_timers.first_key_value().expect("nonempty map");
            let mut victim = first_key;
            let mut threshold = first_time;
            for (&key, &value) in &self.plugin_timers {
                if (key as f32) < threshold {
                    victim = key;
                    threshold = value;
                }
            }
            self.plugin_timers.remove(&victim);
        }
        self.plugin_timers.insert(hash, initial_time);
    }

    /// UpdateEntityKnowledge 82e9e620. Perception and targetable resolution
    /// remain real owner callbacks; neither visibility nor motion is guessed.
    pub(crate) fn update_entity_knowledge<E>(
        &mut self,
        dt: f32,
        mut target_exists: impl FnMut(TargetableId) -> Result<bool, E>,
        mut observe: impl FnMut(TargetableId, bool) -> Result<KnowledgeObservation, E>,
    ) -> Result<(), E> {
        for i in 0..self.wants.len() {
            if self.wants[i].active && !target_exists(self.wants[i].target)? {
                self.set_want_active(i, false).expect("fixed wants index");
            }
        }
        let mut i = 0;
        while i < self.knowledge.len() {
            if !target_exists(self.knowledge[i].target)? {
                self.knowledge.swap_remove(i);
                continue;
            }
            let entry = &mut self.knowledge[i];
            entry.attention_remaining -= dt;
            entry.total_attended += dt;
            if entry.attention_remaining < 0.0 {
                entry.attention_remaining = 0.0;
            }
            entry.mood_restriction -= dt;
            if entry.mood_restriction < 0.0 {
                entry.mood_restriction = 0.0;
            }
            if !entry.just_added && entry.attention_remaining == 0.0 {
                self.knowledge.swap_remove(i);
                continue;
            }
            let observation = observe(entry.target, entry.just_added)?;
            entry.visible = observation.visible;
            if entry.just_added || entry.visible {
                entry.position = observation.position;
                entry.facing = observation.facing;
                entry.velocity = observation.velocity;
            }
            entry.just_added = false;
            i += 1;
        }
        Ok(())
    }
}

/// Original GetTimerTypeEnumFromString 82e70f68 and PDB eTimerType ordinals.
pub(crate) fn timer_type(name: &str) -> Option<u32> {
    const NAMES: [&str; 44] = [
        "NextWarnTimer",
        "InvestigateTimer",
        "ChaseSkaterOutViewTimer",
        "ChaseExhaustionTimer",
        "RestTimer",
        "OutOfViewTimer",
        "LookAtTimer",
        "AlertTimer",
        "TimedRangeRandom",
        "SpectatorBoredomTimer",
        "SpectatorCheerTimer",
        "AmbientBehaviourTimer",
        "CellPhoneTimer",
        "GatherTimer",
        "WanderWaitTimer",
        "PatrolZoneMaintainInterestTimer",
        "AttemptTakeDownTimeout",
        "TakeDownSuccessTimeout",
        "ReturnToPatrolZone",
        "ProRecTimer",
        "TargetConverse",
        "StartChase",
        "ForceWarnTimer",
        "WaitAfterCheer",
        "SitTimer",
        "PresenceCheckTimer",
        "TrespassCheck",
        "PatrolTimer",
        "IdleTimer",
        "TimeUntilICanBePrimaryChaserTimer",
        "BigBlackAvoidanceDeviationTimer",
        "ConversationSpeakingTimer",
        "HandPropActionTimer",
        "HandPropUsageTimer",
        "ThrowHandPropTimer",
        "ThrowHandPropReactionTimer",
        "InterestTimer",
        "TazerIntoTime",
        "TazerCycTime",
        "WanderRoadCheck",
        "TargetUnreachableTimer",
        "WanderTargetTime",
        "CollisionVolumeUnlock",
        "PluginTimeout",
    ];
    NAMES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name))
        .map(|i| i as u32)
}
