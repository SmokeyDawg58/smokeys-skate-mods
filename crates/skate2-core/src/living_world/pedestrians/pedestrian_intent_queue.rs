//! IntentGen retained packet ownership and queue lifecycle. Host IDs replace
//! pointer identity; unmanaged embedded packets survive removal from the queue.
use super::{IntentPacket, IntentStage, PacketState, Suggestion, evaluate_suggestion};
use crate::play_animation::FastString30;
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StageStatus { None, Increment, Decrement }
/// Original tPedAIGraphInput, produced by pedestrian motion graph callbacks.
pub(crate) struct AiGraphInput {
    pub(crate) locomotion_turn_complete: bool,
    pub(crate) decrement_turn_packet: bool,
    pub(crate) locomotion_transition_complete: bool,
    pub(crate) major_intent_complete: bool,
    pub(crate) stage_complete: StageStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> AiGraphInput {
        AiGraphInput { locomotion_turn_complete: false, decrement_turn_packet: false,
            locomotion_transition_complete: false, major_intent_complete: false,
            stage_complete: StageStatus::None }
    }
    fn packet(name: &str, managed: bool) -> IntentPacket {
        let mut stage = IntentStage::default();
        stage.add_intent(FastString30::encode(name).unwrap()).unwrap();
        let mut packet = IntentPacket::new(managed);
        packet.add_stage(stage).unwrap();
        packet
    }
    #[test]
    fn collision_interrupts_but_cannot_be_interrupted() {
        let mut queue = IntentQueue::new();
        let mut input = input();
        let walk = queue.allocate(packet("Walk", false)).unwrap();
        assert_eq!(queue.suggest(walk, &mut input).unwrap(), Suggestion::Replace);
        let collision = queue.allocate(packet("Collision", true)).unwrap();
        assert_eq!(queue.suggest(collision, &mut input).unwrap(), Suggestion::Replace);
        assert_eq!(queue.packet(walk).unwrap().state, PacketState::ClearedByHighPriority);
        assert!(!queue.packet(walk).unwrap().in_queue);
        assert!(input.locomotion_turn_complete && input.locomotion_transition_complete);
        let avoid = queue.allocate(packet("Avoid", true)).unwrap();
        assert_eq!(queue.suggest(avoid, &mut input).unwrap(), Suggestion::Reject);
        assert!(queue.contains(collision));
        input.major_intent_complete = true;
        assert_eq!(queue.advance(&mut input).unwrap(), None);
        assert!(queue.packet(collision).is_err());
        assert!(!input.major_intent_complete);
    }
    #[test]
    fn full_queue_rejects_even_high_priority_before_clearing() {
        let mut queue = IntentQueue::new();
        let mut input = input();
        for name in ["One", "Two", "Three", "Four"] {
            let id = queue.allocate(packet(name, false)).unwrap();
            queue.suggest(id, &mut input).unwrap();
        }
        let collision = queue.allocate(packet("Collision", true)).unwrap();
        assert_eq!(queue.suggest(collision, &mut input).unwrap(), Suggestion::Reject);
        assert_eq!(queue.queue.len(), 4);
    }
}
impl AiGraphInput {
    /// cPedAnimData82e8d268 initializes the same feedback as ClearAllIntents.
    pub(crate) fn new() -> Self {
        Self { locomotion_turn_complete: true, decrement_turn_packet: false,
            locomotion_transition_complete: true, major_intent_complete: false,
            stage_complete: StageStatus::None }
    }
    /// ClearAllIntents82e8b140 stores bytes01 00 01 00 and stage status0.
    pub(crate) fn reset(&mut self) {
        self.locomotion_turn_complete = true;
        self.decrement_turn_packet = false;
        self.locomotion_transition_complete = true;
        self.major_intent_complete = false;
        self.stage_complete = StageStatus::None;
    }
}

pub(crate) struct IntentQueue {
    pub(crate) base: IntentPacket,
    pub(crate) jump: u64,
    pub(crate) avoid_jump: u64,
    pub(crate) avoid_step: u64,
    packets: BTreeMap<u64, IntentPacket>,
    queue: VecDeque<u64>,
    next_id: u64,
    previous_speed_major: FastString30,
}

/// Actual navigation/modifier owner supplies these operations. Separate calls
/// preserve native order, including the second target-direction evaluation in
/// SetCharacterRotateToTarget. No default direction or turn mask is supplied.
pub(crate) trait PacketNavigation {
    fn target_direction(&mut self, queue: &mut IntentQueue, input: &mut AiGraphInput) -> Result<[f32; 4], String>;
    fn turn_mask_with_active_modifiers(&mut self) -> Result<u32, String>;
    fn rotate_to_target(&mut self, queue: &mut IntentQueue, input: &mut AiGraphInput) -> Result<Option<StageRotation>, String>;
}

/// Only the fields SetCharacterRotateToTarget actually writes. Applying this
/// after synchronous jump suggestion cannot restore stale stage intents/state.
pub(crate) struct StageRotation {
    pub(crate) direction: [f32;4],
    pub(crate) max_angle_degrees: f32,
    pub(crate) bearing: f32,
}
impl StageRotation {
    pub(crate) fn apply(&self, stage:&mut IntentStage) {
        stage.target_direction=self.direction;
        stage.rotate_to_target=true;
        stage.max_rotate_angle_degrees=self.max_angle_degrees;
    }
}
pub(crate) struct StrafeDirection {
    pub(crate) rotation: Option<StageRotation>,
    pub(crate) intent: FastString30,
}

pub(crate) trait BaseNavigation: PacketNavigation {
    fn turn_mask(&mut self) -> Result<u32, String>;
    /// Computes the original jump/velocity-dependent distance razor before
    /// GetNavSpeed. Must use the live NavRig range and selected Navigation.
    fn speed_after_clearance_update(&mut self) -> Result<f32, String>;
    fn minimum_velocity_override(&mut self) -> Result<Option<f32>, String>;
    fn modifier_count(&mut self) -> Result<usize, String>;
    fn modifier_active(&mut self, index: usize) -> Result<bool, String>;
    fn modifier_turn_mask(&mut self, index: usize) -> Result<u32, String>;
    fn modifier_speed(&mut self, index: usize) -> Result<Option<f32>, String>;
    fn strafing(&mut self) -> Result<bool, String>;
    fn strafe_rotate_to_target(&mut self, queue:&mut IntentQueue, input:&mut AiGraphInput) -> Result<Option<StageRotation>, String>;
    fn strafe_direction(&mut self, queue:&mut IntentQueue, input:&mut AiGraphInput) -> Result<StrafeDirection, String>;
}
fn idle_stage() -> IntentStage {
    let mut stage = IntentStage::default();
    stage.add_intent(FastString30::encode("Idle").expect("native name")).expect("empty stage");
    stage
}
impl IntentQueue {
    /// LockBaseLocomotion82e75068: packet+16 starts stages; stage+136
    /// is minimum velocity and stage+100 is RotateToTarget (PDB checked).
    pub(crate) fn lock_base_locomotion(&mut self) -> Result<(), String> {
        let stage = self.base.stages.get_mut(self.base.active_stage)
            .ok_or("LockBaseLocomotion: active stage outside packet")?;
        stage.min_linear_velocity = 0.;
        stage.rotate_to_target = false;
        Ok(())
    }
    /// IntentGen::Update82e937b0, excluding debug drawing. All movement inputs
    /// come from the supplied original navigation operations; graph feedback is
    /// the retained animation owner's, not a locally synthesized completion.
    pub(crate) fn update<K, W>(
        &mut self, animation: &mut crate::pedestrian_animation::PedestrianAnimation<K, W>,
        navigation: &mut impl BaseNavigation, explicit_direction: Option<[f32; 4]>,
    ) -> Result<(), String> {
        animation.motion_intents.valid = true;
        let selected=self.advance(&mut animation.ai_input)?;
        match selected {
            Some(id) => self.update_packet(id, explicit_direction, &mut animation.ai_input, navigation)?,
            None => {
                let mappings = animation.selected_animation_data()?.try_pedestrian_speed_mappings()?;
                self.update_base_locomotion(&mut animation.ai_input, navigation, &mappings)?;
            }
        }
        // Update82e937b0 retains its selected packet pointer across synchronous
        // suggestions made by GetNavTargetDirection. Do not select queue front
        // again here: a newly suggested ChaseJump starts on the next update.
        let stage=match selected { Some(id)=>self.packet(id)?.active()?,None=>self.base.active()? };
        animation.motion_intents.set_packet(stage);
        Ok(())
    }

    /// UpdateBaseLocomotion82e8e7e0. Original modifier order is significant:
    /// the FIRST active speed recommendation ends the modifier scan.
    pub(crate) fn update_base_locomotion(
        &mut self, input: &mut AiGraphInput, navigation: &mut impl BaseNavigation,
        mappings: &[crate::attribute_runtime::PedestrianSpeedMapping],
    ) -> Result<(), String> {
        self.base.active()?;
        if !input.locomotion_turn_complete || !input.locomotion_transition_complete { return Ok(()); }
        let mut mask = navigation.turn_mask()?;
        let mut speed = navigation.speed_after_clearance_update()?;
        self.base.stages[self.base.active_stage].min_linear_velocity =
            navigation.minimum_velocity_override()?.unwrap_or(0.);
        let mut force_stop = false;
        let mut index = 0;
        while index < navigation.modifier_count()? {
            if navigation.modifier_active(index)? {
                mask &= navigation.modifier_turn_mask(index)?;
                if let Some(recommended) = navigation.modifier_speed(index)? {
                    speed = recommended;
                    self.base.stages[self.base.active_stage].min_linear_velocity = recommended;
                    force_stop = recommended == 0.;
                    break;
                }
            }
            index += 1;
        }
        if navigation.strafing()? {
            let stage_index = self.base.active_stage;
            let stage = &mut self.base.stages[stage_index];
            stage.intents.clear();
            stage.add_intent(FastString30::encode("Strafe").expect("native name"))?;
            // Source copies before strafe rotation mutates the original stage.
            let mut turn = stage.clone();
            let added = match navigation.strafe_rotate_to_target(self,input)? {
                Some(rotation) => {
                    rotation.apply(self.base.stages.get_mut(stage_index).ok_or("strafe source stage absent")?);
                    turn.override_turn(rotation.bearing, 0xcc, input)?
                },
                None => false,
            };
            if added {
                self.base.add_stage(turn)?;
                self.base.set_active_stage(self.base.active_stage + 1)?;
            } else if speed > 0. {
                let direction=navigation.strafe_direction(self,input)?;
                let stage=self.base.stages.get_mut(stage_index).ok_or("strafe direction stage absent")?;
                if let Some(rotation)=direction.rotation { rotation.apply(stage); }
                stage.add_intent(direction.intent)?;
            }
            return Ok(());
        }
        let can_turn = self.select_speed_mapping(speed, mappings)?;
        if self.base.stages.len() == 1 {
            let stage_index=self.base.active_stage;
            if let Some(rotation) = navigation.rotate_to_target(self,input)? {
                let stage=self.base.stages.get_mut(stage_index).ok_or("base rotation source stage absent")?;
                rotation.apply(stage);
                if can_turn {
                    let mut turn = stage.clone();
                    if turn.override_turn(rotation.bearing, mask, input)? {
                        self.base.add_stage(turn)?;
                        self.base.set_active_stage(self.base.active_stage + 1)?;
                    }
                }
            }
        }
        if force_stop {
            self.base.stages[self.base.active_stage]
                .add_intent(FastString30::encode("ForceStop").expect("native name"))?;
        }
        Ok(())
    }

    /// Non-strafe mapping portion82e8ecc8..82e8f190. Return whether the
    /// previous major intent permits a turn; no matching entry leaves both
    /// existing intents and previous mapping unchanged.
    pub(crate) fn select_speed_mapping(
        &mut self, speed: f32, mappings: &[crate::attribute_runtime::PedestrianSpeedMapping],
    ) -> Result<bool, String> {
        // Raw fcmpu/ble is branch-if-not-greater, including unordered.
        let Some(mapping) = mappings.iter().find(|mapping| !(speed > mapping.velocity)) else {
            return Ok(false);
        };
        let encode = |name: &str| FastString30::encode(name).map_err(|e| e.to_string());
        let major = encode(&mapping.major)?;
        let minor = encode(&mapping.minor)?;
        let stage = self.base.stages.get_mut(self.base.active_stage)
            .ok_or("pedestrian base stage absent")?;
        stage.intents.clear();
        stage.add_intent(major)?;
        // Even an empty minor name is appended by the original code.
        stage.add_intent(minor)?;
        let invalid = FastString30 { words: [0; 5] }; // original82395367 points at NUL
        let mut can_turn = false;
        for name in &mapping.allowed_previous {
            let name = encode(name)?;
            if name == self.previous_speed_major { can_turn = true; break; }
            if name == invalid { break; }
        }
        self.previous_speed_major = major;
        Ok(can_turn)
    }

    /// UpdatePacket82e8e550. Source obtains the desired direction before the
    /// optional locomotion turn stage and finally applies it to the active stage.
    pub(crate) fn update_packet(
        &mut self, id: u64, explicit_direction: Option<[f32; 4]>,
        input: &mut AiGraphInput, navigation: &mut impl PacketNavigation,
    ) -> Result<(), String> {
        let direction = match explicit_direction {
            Some(direction) => direction,
            None => navigation.target_direction(self,input)?,
        };
        let mut turn = self.packet(id)?.active()?.clone();
        if input.locomotion_turn_complete && turn.use_locomotion_intents && turn.intents().len() == 1 {
            let mask = navigation.turn_mask_with_active_modifiers()?;
            if let Some(rotation) = navigation.rotate_to_target(self,input)? {
                rotation.apply(&mut turn);
                if turn.override_turn(rotation.bearing, mask, input)? {
                    let packet=self.packet_mut(id)?;
                    packet.add_stage(turn)?;
                    packet.set_active_stage(packet.active_stage + 1)?;
                }
            }
        }
        let packet=self.packet_mut(id)?;
        let stage = packet.stages.get_mut(packet.active_stage)
            .ok_or("pedestrian active stage absent")?;
        stage.target_direction = direction;
        stage.max_rotate_angle_degrees = 4.25;
        Ok(())
    }
    /// IntentGen82e8e3f0: three unmanaged embedded packets and managed base.
    pub(crate) fn new() -> Self {
        let mut base = IntentPacket::new(true);
        base.add_stage(idle_stage()).expect("empty base");
        Self { base, jump: 1, avoid_jump: 2, avoid_step: 3,
            packets: (1..=3).map(|id| (id, IntentPacket::new(false))).collect(),
            queue: VecDeque::new(), next_id: 4,
            previous_speed_major: FastString30::encode("FromPacket").expect("native name") }
    }
    pub(crate) fn allocate(&mut self, packet: IntentPacket) -> Result<u64, String> {
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or("pedestrian packet ID exhausted")?;
        self.packets.insert(id, packet);
        Ok(id)
    }
    pub(crate) fn packet(&self, id: u64) -> Result<&IntentPacket, String> {
        self.packets.get(&id).ok_or_else(|| format!("pedestrian packet {id} absent"))
    }
    pub(crate) fn packet_mut(&mut self, id: u64) -> Result<&mut IntentPacket, String> {
        self.packets.get_mut(&id).ok_or_else(|| format!("pedestrian packet {id} absent"))
    }
    pub(crate) fn contains(&self, id: u64) -> bool { self.queue.contains(&id) }
    /// CreateMonitoredIntent82e9c130/RegisterIntentMonitor82e9a338. The native
    /// fixed_map permits overflow allocation, so 16 is not a rejection limit.
    pub(crate) fn create_monitored(&mut self,
        state: &mut crate::living_world_pedestrian_state::PedestrianStateStorage,
        hash: u32,
    ) -> Result<u64, String> {
        let id = self.allocate(IntentPacket::new(false))?;
        if state.intent_monitors.get(&hash).is_some_and(|old| self.packets.contains_key(old)) {
            self.packets.remove(&id);
            return Err("native RegisterIntentMonitor rejected duplicate live packet".into());
        }
        state.intent_monitors.insert(hash, id);
        Ok(id)
    }
    /// ReleaseMonitoredIntent82e8dc60 hands an active/queued packet back to the
    /// SAME queue for managed destruction, otherwise destroys it immediately.
    pub(crate) fn release_monitored(&mut self,
        state: &mut crate::living_world_pedestrian_state::PedestrianStateStorage,
        hash: u32,
    ) -> Result<(), String> {
        let Some(&id) = state.intent_monitors.get(&hash) else { return Ok(()); };
        let Some(packet) = self.packets.get_mut(&id) else { return Ok(()); };
        state.intent_monitors.remove(&hash);
        if packet.in_queue || matches!(packet.state, PacketState::InQueue | PacketState::Active) {
            packet.managed = true;
        } else { self.packets.remove(&id); }
        Ok(())
    }
    pub(crate) fn active_stage(&self) -> Result<&IntentStage, String> {
        match self.queue.front() { Some(&id) => self.packet(id)?.active(), None => self.base.active() }
    }
    /// RemoveFrontPacket82e86e38: clear membership, destroy managed only.
    fn remove_front(&mut self) -> Result<(), String> {
        let id = *self.queue.front().ok_or("pedestrian empty intent queue")?;
        let packet = self.packet_mut(id)?;
        packet.in_queue = false;
        let managed = packet.managed;
        self.queue.pop_front();
        if managed { self.packets.remove(&id); }
        Ok(())
    }
    /// ClearAllIntents82e8b028. Normal completion is preserved when clearing.
    pub(crate) fn clear(&mut self, high_priority: bool, input: Option<&mut AiGraphInput>) -> Result<(), String> {
        while let Some(&id) = self.queue.front() {
            let packet = self.packet_mut(id)?;
            if packet.state != PacketState::CompleteNormal {
                packet.state = if high_priority { PacketState::ClearedByHighPriority } else { PacketState::ClearedByCall };
            }
            self.remove_front()?;
        }
        self.base.stages.clear();
        self.base.active_stage = 0;
        self.base.in_queue = false;
        self.base.state = PacketState::NotAdded;
        if let Some(input) = input { input.reset(); }
        self.base.add_stage(idle_stage())?;
        Ok(())
    }
    /// SuggestNextIntent82e93c70; duplicate/full checks precede replacement.
    pub(crate) fn suggest(&mut self, id: u64, input: &mut AiGraphInput) -> Result<Suggestion, String> {
        let packet = self.packet(id)?;
        let tail = self.queue.back().map(|&id| self.packet(id)).transpose()?;
        let decision = evaluate_suggestion(packet, self.queue.len(), tail, self.active_stage()?)?;
        if packet.stages.is_empty() { return Err("pedestrian suggested packet has no stages".into()); }
        if decision == Suggestion::Reject {
            self.packet_mut(id)?.state = PacketState::NotAdded;
            return Ok(decision);
        }
        if decision == Suggestion::Replace { self.clear(true, Some(input))?; }
        let packet = self.packet_mut(id)?;
        packet.state = PacketState::InQueue;
        packet.in_queue = true;
        self.queue.push_back(id);
        Ok(decision)
    }
    /// Queue/stage prefix of Update82e937b0. Returns the live queued packet,
    /// or None for base locomotion. Caller still MUST execute UpdatePacket or
    /// UpdateBaseLocomotion and SetMotionGraphFromIntentPacket afterward.
    pub(crate) fn advance(&mut self, input: &mut AiGraphInput) -> Result<Option<u64>, String> {
        if let Some(&id) = self.queue.front() {
            let packet = self.packet_mut(id)?;
            match input.stage_complete {
                StageStatus::Increment => packet.set_active_stage(packet.active_stage + 1)?,
                StageStatus::Decrement => packet.active_stage = packet.active_stage.saturating_sub(1),
                StageStatus::None => {}
            }
            input.stage_complete = StageStatus::None;
            if input.major_intent_complete || packet.state == PacketState::CompleteNormal {
                packet.state = PacketState::CompleteNormal;
                self.remove_front()?;
                input.major_intent_complete = false;
                while let Some(&next) = self.queue.front() {
                    if self.packet(next)?.state != PacketState::CompleteNormal { break; }
                    self.remove_front()?;
                }
            }
        }
        if let Some(&id) = self.queue.front() {
            self.packet_mut(id)?.state = PacketState::Active;
            Ok(Some(id))
        } else {
            if input.decrement_turn_packet {
                if self.base.active_stage > 0 {
                    self.base.active_stage -= 1;
                    self.base.stages.pop().ok_or("pedestrian empty base stage")?;
                }
                input.decrement_turn_packet = false;
            }
            Ok(None)
        }
    }
}
