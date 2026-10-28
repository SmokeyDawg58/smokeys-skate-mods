//! Native pedestrian intent packets, separate from skater action intents.
//! Constructors82e86eb8/82e8a558; equality82e873f8/82e87530.
use crate::play_animation::FastString30;
#[path = "pedestrian_intent_queue.rs"]
pub(crate) mod queue;

#[derive(Clone, Debug)]
pub(crate) struct IntentStage {
    intents: Vec<FastString30>,
    pub(crate) rotate_to_target: bool,
    pub(crate) use_locomotion_intents: bool,
    pub(crate) target_direction: [f32; 4],
    pub(crate) max_rotate_angle_degrees: f32,
    pub(crate) ground_normal_blend: f32,
    pub(crate) min_linear_velocity: f32,
}
impl Default for IntentStage {
    fn default() -> Self {
        Self { intents: Vec::new(), rotate_to_target: true,
            use_locomotion_intents: false, target_direction: [0.; 4],
            max_rotate_angle_degrees: 4.25, ground_normal_blend: 0.,
            min_linear_velocity: 0. }
    }
}
impl IntentStage {
    /// DetermineOverrideTurningIntents82e8b168. Turn-mask order and PPC
    /// unordered comparison fall-through are intentional; no shortest-turn
    /// heuristic substitutes for the authored locomotion branch selection.
    pub(crate) fn override_turn(
        &mut self, bearing_radians: f32, mask: u32, input: &mut queue::AiGraphInput,
    ) -> Result<bool, String> {
        let degrees = bearing_radians * f32::from_bits(0x4265_2ee1);
        let name = [
            (0x10, !(degrees < 160.), "ReverseRight"),
            (0x01, !(degrees > -160.), "ReverseLeft"),
            (0x20, !(degrees < 115.), "TurnRight135"),
            (0x02, !(degrees > -115.), "TurnLeft135"),
            (0x40, !(degrees < 70.), "TurnRight90"),
            (0x04, !(degrees > -70.), "TurnLeft90"),
            (0x80, !(degrees < 25.), "TurnRight45"),
            (0x08, !(degrees > -25.), "TurnLeft45"),
        ].into_iter().find(|(bit, angle, _)| mask & bit != 0 && *angle);
        let Some((_, _, name)) = name else { return Ok(false); };
        self.add_intent(FastString30::encode(name).expect("native turn name"))?;
        self.rotate_to_target = false;
        self.min_linear_velocity = 0.;
        input.locomotion_turn_complete = false;
        Ok(true)
    }
    pub(crate) fn add_intent(&mut self, name: FastString30) -> Result<(), String> {
        if self.intents.len() == 4 { return Err("pedestrian stage intent capacity".into()); }
        self.intents.push(name);
        Ok(())
    }
    pub(crate) fn intents(&self) -> &[FastString30] { &self.intents }
    /// Native recovery rewrites AnimIntentCount only, preserving stage controls.
    pub(crate) fn clear_intents(&mut self) { self.intents.clear(); }
    pub(crate) fn major(&self) -> Option<&FastString30> { self.intents.first() }
    /// Native equality deliberately excludes target direction and ground blend.
    pub(crate) fn equivalent(&self, rhs: &Self) -> bool {
        self.intents == rhs.intents && self.rotate_to_target == rhs.rotate_to_target
            && self.use_locomotion_intents == rhs.use_locomotion_intents
            && self.max_rotate_angle_degrees == rhs.max_rotate_angle_degrees
            && self.min_linear_velocity == rhs.min_linear_velocity
    }
}

/// Values verified against the original PDB ePacketState enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum PacketState {
    NotAdded = 0, InQueue = 1, Active = 2, CompleteNormal = 3,
    ClearedByHighPriority = 4, ClearedByCall = 5,
}
pub(crate) struct IntentPacket {
    stages: Vec<IntentStage>,
    active_stage: usize,
    pub(crate) managed: bool,
    pub(crate) in_queue: bool,
    pub(crate) state: PacketState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Suggestion { Append, Replace, Reject }

/// EvaluateSuggestType82e8ae18. The caller supplies the actual queue tail and
/// current active stage (queue front when active, otherwise base locomotion).
/// Mutation/animation clearing belongs to SuggestNextIntent, not this decision.
pub(crate) fn evaluate_suggestion(
    packet: &IntentPacket, queue_len: usize, tail: Option<&IntentPacket>,
    active: &IntentStage,
) -> Result<Suggestion, String> {
    if tail.is_some_and(|tail| packet.equivalent(tail)) || queue_len >= 4 {
        return Ok(Suggestion::Reject);
    }
    let Some(major) = packet.active()?.major() else { return Ok(Suggestion::Reject); };
    let collision = FastString30::encode("Collision").expect("native name");
    let takedown = FastString30::encode("DoTakeDown").expect("native name");
    if active.major().is_some_and(|old| *old == collision || *old == takedown && *major != collision) {
        return Ok(Suggestion::Reject);
    }
    let avoid = FastString30::encode("Avoid").expect("native name");
    let jump = FastString30::encode("ChaseJump").expect("native name");
    Ok(if *major == collision || *major == avoid || *major == jump || queue_len == 0 {
        Suggestion::Replace
    } else { Suggestion::Append })
}
impl IntentPacket {
    /// GetNavTargetDirection82e8ba1c..28 clears this embedded packet before
    /// building ChaseJump; its Managed lifetime remains unchanged.
    pub(crate) fn reset_for_jump(&mut self) {
        self.stages.clear(); self.active_stage=0; self.in_queue=false;
        self.state=PacketState::NotAdded;
    }
    /// CreateJump/StepAvoidIntentPacket82e8c410/82e8c4a0 changes only count
    /// and stage0. Existing queue/state/active-stage fields remain untouched.
    pub(crate) fn replace_single_stage(&mut self, stage: IntentStage) {
        self.stages.clear();
        self.stages.push(stage);
    }
    pub(crate) fn new(managed: bool) -> Self {
        Self { stages: Vec::new(), active_stage: 0, managed, in_queue: false,
            state: PacketState::NotAdded }
    }
    pub(crate) fn add_stage(&mut self, stage: IntentStage) -> Result<usize, String> {
        if self.stages.len() == 3 { return Err("pedestrian packet stage capacity".into()); }
        let index = self.stages.len();
        self.stages.push(stage);
        Ok(index)
    }
    pub(crate) fn set_active_stage(&mut self, index: usize) -> Result<(), String> {
        if index >= self.stages.len() { return Err("pedestrian packet stage index".into()); }
        self.active_stage = index;
        Ok(())
    }
    pub(crate) fn active(&self) -> Result<&IntentStage, String> {
        self.stages.get(self.active_stage).ok_or_else(|| "pedestrian active stage absent".into())
    }
    pub(crate) fn stage_mut(&mut self, index: usize) -> Result<&mut IntentStage, String> {
        self.stages.get_mut(index).ok_or_else(|| "pedestrian packet stage absent".into())
    }
    pub(crate) fn active_stage_number(&self) -> usize { self.active_stage }
    /// AdvanceStage stores first and asserts second, retaining mutation on error.
    pub(crate) fn advance_stage(&mut self) -> Result<(), String> {
        self.active_stage = self.active_stage.wrapping_add(1);
        if self.active_stage >= self.stages.len() { return Err("pedestrian advanced beyond packet stages".into()); }
        Ok(())
    }
    pub(crate) fn equivalent(&self, rhs: &Self) -> bool {
        self.stages.len() == rhs.stages.len()
            && self.stages.iter().zip(&rhs.stages).all(|(a,b)| a.equivalent(b))
    }
    /// Pedestrian::IsPacketComplete82e6dfd8 takes the current AI graph result.
    pub(crate) fn complete(&self, major_intent_complete: bool) -> bool {
        matches!(self.state, PacketState::CompleteNormal | PacketState::ClearedByHighPriority
            | PacketState::ClearedByCall)
            || self.state == PacketState::Active && major_intent_complete
    }
}
