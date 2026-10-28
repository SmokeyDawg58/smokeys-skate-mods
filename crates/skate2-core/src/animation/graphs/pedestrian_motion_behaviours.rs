//! Native pedestrian motion behaviour constructors and direct callbacks.
//! All fourteen CreateInstance slots are82bb3da8 (null); generated StateUpdate
//! uses direct callbacks rather than walking the active virtual Update methods.
use super::{ActiveBehaviour, MotionDeclarations};
use crate::{
    dynamic_state_graph_controller::DynamicStateGraphContext,
    generated_action_load::Attribute,
    living_world_pedestrian_state::{PedestrianStateStorage, timer_type},
    motion_graph_entity_authority::BoundMotionGraphContext,
    pedestrian_intent_packet::{PacketState, queue::{AiGraphInput, IntentQueue, StageStatus}},
    play_remapped_animation::{PlayRemappedAnimationBehaviour, PlayRemappedAnimationCollaborator},
};
use skate2_formats::state_graph::{StateGraphBinaryAttribute, state_graph_string_hash32};

enum Behaviour {
    StopChannel(f32), KeepTimer(u32), Speech(u32), KeepMonitor(u32),
    MajorComplete, StageComplete(bool), TurnComplete, Transition, Takedown,
    Jump, ChanceDrop, SpawnHandProp,
    Animation(PlayRemappedAnimationBehaviour),
    JumpAnimation { animation: PlayRemappedAnimationBehaviour, original_length: f32, downward: bool },
}
struct Entry { enabled: bool, behaviour: Behaviour }
pub(crate) struct Behaviours { entries: Vec<Entry> }

/// These are operations on the already retained pedestrian owners, not a second
/// animation player, timer store, intent queue, RNG or hand-prop system. Bind one
/// adapter to the delegate's actual entity authority for the duration of Update.
pub(crate) trait Services: PlayRemappedAnimationCollaborator<Error = String> {
    fn has_entity(&self) -> bool;
    fn state(&mut self) -> Result<&mut PedestrianStateStorage, String>;
    fn intents(&mut self) -> Result<(&mut IntentQueue, &mut AiGraphInput), String>;
    /// Call retained PedestrianAnimation::complete_locomotion_turn/transition.
    fn complete_locomotion_turn(&mut self) -> Result<(), String>;
    fn complete_locomotion_transition(&mut self) -> Result<(), String>;
    fn stop_channels(&mut self, blend_out: f32) -> Result<(), String>;
    /// Original82e6fd68 also changes the takedown animation state and timer.
    fn set_attempting_takedown(&mut self, value: bool) -> Result<(), String>;
    fn trigger_jump(&mut self) -> Result<(), String>;
    fn jump_time(&mut self, downward: bool) -> Result<f32, String>;
    fn has_hand_prop(&mut self) -> Result<bool, String>;
    fn drop_on_large_avoid_probability(&mut self) -> Result<f32, String>;
    fn bernoulli_random(&mut self, probability: f32) -> Result<bool, String>;
    fn throw_hand_prop_away(&mut self) -> Result<(), String>;
    /// CurrentWaypoint -> group behaviour GetHandPropToSpawn; None only when
    /// native waypoint is absent. A missing required group/interface is an error.
    fn interaction_hand_prop(&mut self) -> Result<Option<u64>, String>;
    fn set_hand_prop_to_spawn(&mut self, key: u64) -> Result<(), String>;
    fn log_spawn_vending_machine_hand_prop(&mut self) -> Result<(), String>;
    fn set_speech_state(&mut self, state: u32) -> Result<(), String>;
}

fn value<'a>(attrs: &'a [Attribute], key: &str) -> Option<&'a str> {
    attrs.iter().find(|a| a.key == key).map(|a| a.value)
}
fn float(attrs: &[Attribute], key: &str, default: f32) -> Result<f32, String> {
    value(attrs, key).map_or(Ok(default), |s| s.parse::<f64>().map(|x| x as f32).map_err(|e| e.to_string()))
}
fn boolean(attrs: &[Attribute], key: &str, default: bool) -> Result<bool, String> {
    match value(attrs, key) { None => Ok(default), Some("true") => Ok(true), Some("false") => Ok(false),
        Some(v) => Err(format!("pedestrian motion boolean {key}={v}")) }
}
fn animation(attrs: &[Attribute]) -> Result<PlayRemappedAnimationBehaviour, String> {
    let attrs: Vec<_> = attrs.iter().map(|a| Ok(StateGraphBinaryAttribute {
        key: a.key.as_bytes().to_vec(), key_hash: state_graph_string_hash32(a.key.as_bytes()),
        text: a.value.as_bytes().to_vec(),
        float_bits: match a.key {
            "playBackSpeed" | "frameOffset" | "cycleSwappingPercentage" | "blendTime" => float(attrs, a.key, 0.)?.to_bits(),
            _ => 0,
        },
        boolean_raw: if a.key == "mirrorOverride" { u8::from(boolean(attrs, a.key, false)?) } else { 0 },
    })).collect::<Result<_, String>>()?;
    PlayRemappedAnimationBehaviour::from_attributes(&attrs).map_err(|e| e.to_string())
}
impl Behaviours {
    /// TemplateHC82930ce8 constructs all472 occurrences in declaration order.
    /// Factory82930e84 receives NULL parent, so Base82cf6f88 reads only the local
    /// enabled attribute (defaulttrue). State.active controls implicit child
    /// selection; it must not disable behaviours on explicit transition targets.
    pub(crate) fn bind(declarations: &MotionDeclarations) -> Result<Self, String> {
        let mut entries = Vec::with_capacity(declarations.behaviours.len());
        for declaration in declarations.behaviours {
            let section = declaration.attributes;
            let attrs = if section.count == 0 { &[][..] } else {
                let start = usize::from(section.index);
                declarations.attributes.get(start..start + usize::from(section.count)).ok_or("motion behaviour attributes")?
            };
            let enabled = boolean(attrs, "enabled", true)?;
            let behaviour = match declaration.name {
                "StopChannel" => Behaviour::StopChannel(float(attrs, "blendOutTime", 0.2)?), //826eba08
                "KeepSimpleTimerActive" => Behaviour::KeepTimer(timer_type(value(attrs, "timerName").unwrap_or("TimedRangeRandom")).ok_or("motion timerName")?), //826ec7e0
                "KeepMonitorIntentInQueue" => {
                    let name = value(attrs, "intentName").filter(|s| !s.is_empty()).ok_or("motion intentName")?;
                    Behaviour::KeepMonitor(skate2_formats::hash::sk8_hash_value_32(name.as_bytes())) //826ee980
                }
                "SendSpeechEvent" => { //826ed130, fctiwz followed by 1..71 assertion
                    value(attrs, "speechevent").ok_or("motion speechevent")?;
                    let number = float(attrs, "speechvalue", 0.)?;
                    if !number.is_finite() || number.trunc() <= 0. || number.trunc() >= 72. { return Err("motion speechvalue assertion".into()); }
                    Behaviour::Speech(number as u32)
                }
                "IntentStageComplete" => Behaviour::StageComplete(boolean(attrs, "decrement", false)?), //826f2df0
                "MajorIntentComplete" => Behaviour::MajorComplete,
                "LocomotionTurnComplete" => Behaviour::TurnComplete,
                "LocomotionTransition" => Behaviour::Transition,
                "SetAttemptingTakedown" => Behaviour::Takedown,
                "TriggerPedestrianJump" => Behaviour::Jump,
                "ChanceToDropHandPropOnLargeAvoid" => Behaviour::ChanceDrop,
                "SpawnInteractionBasedHandProp" => Behaviour::SpawnHandProp,
                "PlayRemappedAnimation" => Behaviour::Animation(animation(attrs)?),
                "PlayPedestrianTimeScaledJumpAnimation" => Behaviour::JumpAnimation { //82718110
                    animation: animation(attrs)?, original_length: float(attrs, "originalLength", 1.)?,
                    downward: boolean(attrs, "inDownwardMotion", false)?,
                },
                name => return Err(format!("unbound pedestrian motion behaviour {name}")),
            };
            entries.push(Entry { enabled, behaviour });
        }
        Ok(Self { entries })
    }
    fn entry(&self, occurrence: u16) -> Result<&Entry, String> {
        self.entries.get(usize::from(occurrence)).ok_or_else(|| "motion behaviour occurrence".into())
    }
    pub(crate) fn enabled(&self, occurrence: u16) -> Result<bool, String> { Ok(self.entry(occurrence)?.enabled) }
    pub(crate) fn begin(&self, occurrence: u16, backend: &mut impl Services) -> Result<(), String> {
        match &self.entry(occurrence)?.behaviour {
            Behaviour::MajorComplete => { require_entity(backend)?; backend.intents()?.1.major_intent_complete = true; }
            Behaviour::StageComplete(decrement) => { require_entity(backend)?;
                backend.intents()?.1.stage_complete = if *decrement { StageStatus::Decrement } else { StageStatus::Increment }; }
            Behaviour::TurnComplete => { require_entity(backend)?; backend.complete_locomotion_turn()?; }
            Behaviour::Transition => { //StartLocomotionTransition82e79580
                require_entity(backend)?;
                let (queue, input) = backend.intents()?;
                input.locomotion_transition_complete = false;
                queue.lock_base_locomotion()?;
            }
            Behaviour::Takedown => { require_entity(backend)?; backend.set_attempting_takedown(true)?; }
            Behaviour::Jump => { require_entity(backend)?; backend.trigger_jump()?; }
            Behaviour::StopChannel(blend) => { require_entity(backend)?; backend.stop_channels(*blend)?; }
            Behaviour::Speech(state) => { if backend.has_entity() { backend.set_speech_state(*state)?; } }
            Behaviour::ChanceDrop => { //826f3338 deliberately queries HasHandProp twice
                require_entity(backend)?;
                if backend.has_hand_prop()? {
                    let probability = if backend.has_hand_prop()? { backend.drop_on_large_avoid_probability()? } else { 0. };
                    if backend.bernoulli_random(probability)? { backend.throw_hand_prop_away()?; }
                }
            }
            Behaviour::SpawnHandProp => { //82701b08
                if backend.has_entity() {
                    if let Some(key) = backend.interaction_hand_prop()?.filter(|key| *key != 0) {
                        backend.set_hand_prop_to_spawn(key)?;
                        backend.log_spawn_vending_machine_hand_prop()?;
                    }
                }
            }
            Behaviour::Animation(animation) => animation.begin(backend)?,
            Behaviour::JumpAnimation { animation, original_length, downward } => { //82718520
                require_entity(backend)?;
                // Mirror write precedes sampling jump time, unlike merely
                // computing a speed then calling the generic Begin wrapper.
                let mut animation = animation.clone();
                if let Some(mirror) = animation.mirror_override_raw.take() { backend.write_mirror_override_raw(mirror)?; }
                animation.playback_speed_bits = (*original_length / backend.jump_time(*downward)?).to_bits();
                animation.begin(backend)?;
            }
            Behaviour::KeepTimer(_) | Behaviour::KeepMonitor(_) => {}
        }
        Ok(())
    }
    pub(crate) fn end(&self, occurrence: u16, backend: &mut impl Services) -> Result<(), String> {
        match &self.entry(occurrence)?.behaviour {
            Behaviour::Transition => { require_entity(backend)?; backend.complete_locomotion_transition() }
            Behaviour::Takedown => { require_entity(backend)?; backend.set_attempting_takedown(false) }
            _ => Ok(()), //all other End slots raw blr825187c0
        }
    }
    /// Exact direct slot callbacks from all18 StateUpdate bodies. Even a noop
    /// callback writes the ORIGINAL Context instance slot before returning.
    pub(crate) fn generated_update<K>(&self, source: u32, context: &mut BoundMotionGraphContext<K>,
        active: &[ActiveBehaviour], backend: &mut impl Services) -> Result<(), String>
    {
        use Update::*;
        let updates: &[Update] = match source {
            0x825187c0 => &[],
            0x828f5920 => &[Noop, Monitor, Timer],
            0x828f5990 => &[Noop, Noop, JumpAnimation],
            0x828f5a00 => &[Noop, JumpAnimation],
            0x828f5a60 => &[Noop, Animation, Noop],
            0x828f5ad0 => &[Animation, Noop, Noop],
            0x828f5b40 => &[Animation, Noop],
            0x828f5ba0 => &[Noop, Monitor, Timer, Animation],
            0x828f5c28 => &[Animation],
            0x828f5c40 => &[Noop, Noop, Animation, Noop],
            0x828f5cc8 => &[Noop, Noop, Noop],
            0x828f5d38 => &[Noop, Noop, Animation],
            0x828f5da8 => &[Noop, Noop, Noop, Monitor, Timer],
            0x828f5e40 => &[Noop, Noop, Noop, Animation],
            0x828f5ec8 => &[Noop, Noop, Noop, Monitor, Timer, Animation],
            0x828f81d0 => &[Noop],
            0x828f81e8 => &[Noop, Noop],
            0x828f89c0 => &[Noop, Animation],
            _ => return Err(format!("unbound pedestrian motion update {source:08x}")),
        };
        for (slot, update) in updates.iter().enumerate() {
            let active = active.get(slot).ok_or("motion generated update slot")?;
            if active.instance.is_some() { return Err("pedestrian motion native instance must be null".into()); }
            context.set_generated_instance(None);
            let behaviour = &self.entry(active.occurrence)?.behaviour;
            match (update, behaviour) {
                (Noop, _) => {},
                (Animation, Behaviour::Animation(animation)) => animation.update(backend)?,
                (JumpAnimation, Behaviour::JumpAnimation { animation, original_length, downward }) => {
                    let mut animation = animation.clone();
                    animation.playback_speed_bits = (*original_length / backend.jump_time(*downward)?).to_bits();
                    animation.update(backend)?;
                }
                (Timer, Behaviour::KeepTimer(kind)) => { //826ef798 fsubs then fsel
                    if backend.has_entity() {
                        let state = backend.state()?;
                        let value = state.timer(*kind);
                        let value = if value - 1. >= 0. { value } else { 1. };
                        state.set_timer(*kind, value).map_err(|e| format!("motion timer: {e:?}"))?;
                    }
                }
                (Monitor, Behaviour::KeepMonitor(hash)) => { //826ef6b8
                    require_entity(backend)?;
                    let id = backend.state()?.intent_monitors.get(hash).copied();
                    if let Some(id) = id {
                        let (queue, input) = backend.intents()?;
                        if queue.packet(id)?.state == PacketState::ClearedByHighPriority { queue.suggest(id, input)?; }
                    }
                }
                _ => return Err(format!("motion direct callback/slot mismatch {source:08x}:{slot}")),
            }
        }
        Ok(())
    }
}
enum Update { Noop, Monitor, Timer, Animation, JumpAnimation }
fn require_entity(backend: &impl Services) -> Result<(), String> {
    if backend.has_entity() { Ok(()) } else { Err("pedestrian motion native entity assertion".into()) }
}
