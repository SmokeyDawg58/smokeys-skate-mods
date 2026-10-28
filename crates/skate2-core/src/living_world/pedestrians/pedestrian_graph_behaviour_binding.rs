//! Retained occurrence bindings. Callback phase comes from the verified native
//! lifecycle, never from whether a Rust match happens to implement that phase.
#[path = "pedestrian_graph_watch_want.rs"]
mod watch_want;
use super::{lifecycle, locomotion::SuggestVelocity, simple_behaviours::SimpleBehaviour};
use crate::{living_world_pedestrian_entity::PedestrianConstruction, pedestrian_graph_data as data};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Phase { Begin, Update, End }
enum LocalBehaviour {
    Simple(SimpleBehaviour), Velocity(SuggestVelocity),
    Wander,
    WatchWant { want:usize, saved_speed:f32, stopped:bool, always_stop:bool },
    DrawTazer { want: usize, needs_spawn: bool },
    Patrol(crate::living_world_pedestrian_entity::patrol::PatrolExecution),
    Colliding { behaviour: super::colliding::CollidingBehaviour,
        instance: super::colliding::CollidingInstance },
    Modifier { kind: crate::pedestrian_navigation_modifiers::ModifierKind,
        change_to: bool, previous: bool },
    Marquee(super::marquee::MarqueeBehaviour),
}
struct Entry {
    lifecycle: lifecycle::Lifecycle,
    enabled: bool,
    local: Option<LocalBehaviour>,
    forwarded_occurrence: Option<u16>,
    fields: &'static [crate::generated_action_load::Attribute],
}
pub(crate) struct LocalBehaviourBindings { entries: Vec<Entry> }

/// Unhandled callbacks carry their real address so the concrete backend must
/// dispatch their existing nonlocal owner or report the exact missing callback.
/// This is not a successful no-op result.
pub(crate) enum LocalDispatch { Handled, Nonlocal { source: u32 } }

impl LocalBehaviourBindings {
    pub(crate) fn new() -> Result<Self, String> {
        let mut entries = Vec::with_capacity(data::BEHAVIOURS.len());
        for (index, declaration) in data::BEHAVIOURS.iter().enumerate() {
            let lifecycle = lifecycle::lookup(declaration.name)
                .ok_or_else(|| format!("pedestrian lifecycle absent: {}", declaration.name))?;
            let local = if let Some(simple) = SimpleBehaviour::bind(index as u16)? {
                Some(LocalBehaviour::Simple(simple))
            } else if declaration.name == "SuggestVelocity" {
                Some(LocalBehaviour::Velocity(SuggestVelocity::bind(index as u16)?))
            } else if declaration.name == "Wander" {
                Some(LocalBehaviour::Wander)
            } else if declaration.name == "DrawTazer" {
                let name=crate::living_world_guard_graph::binding::attribute(index as u16,"want")?
                    .ok_or("DrawTazer requires want")?;
                Some(LocalBehaviour::DrawTazer{want:super::conditions::want_type(name)
                    .ok_or("DrawTazer invalid want")?,needs_spawn:false})
            } else if matches!(declaration.name,"StopAndFaceWantTarget"|"WatchWantTargetWithoutInterruptingLocomotion") {
                let want=crate::living_world_guard_graph::binding::attribute(index as u16,"want")?
                    .and_then(super::conditions::want_type).ok_or("watch/face target requires valid want")?;
                Some(LocalBehaviour::WatchWant{want,saved_speed:0.,stopped:false,
                    always_stop:declaration.name=="StopAndFaceWantTarget"})
            } else if declaration.name == "Patrol" {
                Some(LocalBehaviour::Patrol(
                    crate::living_world_pedestrian_entity::patrol::PatrolExecution::new()))
            } else if declaration.name == "PedestrianColliding" {
                Some(LocalBehaviour::Colliding {
                    behaviour: super::colliding::CollidingBehaviour::bind(index as u16)?,
                    instance: super::colliding::CollidingInstance::new(),
                })
            } else if declaration.name == "ChangeNavModifierSetting" {
                use crate::living_world_guard_graph::binding;
                use crate::pedestrian_navigation_modifiers::ModifierKind;
                // Constructor826ee5f8 accepts these five spellings, ignoring
                // case. It does NOT expose the DynamicObject modifier here.
                let name = binding::attribute(index as u16, "modifier")?
                    .ok_or("ChangeNavModifierSetting requires modifier")?;
                let kind = [("ChaseGlue", ModifierKind::ChaseGlue),
                    ("Pedestrian", ModifierKind::General),
                    ("SkaterAvoidance", ModifierKind::Skater),
                    ("VehicleAvoidance", ModifierKind::Vehicle),
                    ("BigBlackAvoidance", ModifierKind::BigBlack)]
                    .into_iter().find(|(n, _)| n.eq_ignore_ascii_case(name))
                    .map(|(_, kind)| kind).ok_or("unknown native modifier name")?;
                Some(LocalBehaviour::Modifier { kind,
                    change_to: binding::boolean(index as u16, "set_to", false)?,
                    previous: false })
            } else { None };
            // Generated ctor82932c90 passes null parent. Base82cf6f88
            // therefore reads this occurrence's enabled attribute, default true;
            // it does not inherit the generated parent state's active flag.
            let enabled = crate::living_world_guard_graph::binding::boolean(
                index as u16, "enabled", true)?;
            entries.push(Entry { lifecycle, enabled, local, forwarded_occurrence: None, fields: &[] });
        }
        Ok(Self { entries })
    }

    /// The `TemplateHC` constructor's behaviour objects for the entity's delegate:
    /// HCDelegate_marquee 0x82947490 and HCDelegate_Photographer 0x8294cac0 construct each
    /// declaration through the shared factory with its own attribute rows.
    pub(crate) fn for_delegate(delegate: super::AiDelegate) -> Result<Self, String> {
        use super::{AiDelegate, marquee, marquee_data};
        let (declarations, attributes) = match delegate {
            AiDelegate::Pedestrian => return Self::new(),
            AiDelegate::BigBlack => (&super::big_black_data::big_black::BEHAVIOURS[..],
                &super::big_black_data::big_black::ATTRIBUTES[..]),
            AiDelegate::Marquee => (&marquee_data::marquee::BEHAVIOURS[..], &marquee_data::marquee::ATTRIBUTES[..]),
            AiDelegate::Photographer => (&marquee_data::photographer::BEHAVIOURS[..],
                &marquee_data::photographer::ATTRIBUTES[..]),
        };
        let mut entries = Vec::with_capacity(declarations.len());
        for declaration in declarations {
            let lifecycle = lifecycle::lookup(declaration.name)
                .ok_or_else(|| format!("pedestrian lifecycle absent: {}", declaration.name))?;
            let fields = marquee::attributes(attributes, declaration.attributes)?;
            let local = match declaration.name {
                "PedestrianColliding" => LocalBehaviour::Colliding {
                    behaviour: marquee::colliding(fields)?,
                    instance: super::colliding::CollidingInstance::new(),
                },
                "SendSpeechEvent" => LocalBehaviour::Simple(SimpleBehaviour::Speech(marquee::speech(fields)?)),
                "SuggestVelocity"=>LocalBehaviour::Velocity(SuggestVelocity::from_attributes(fields)?),
                "AllowPedestrianJumping"=>LocalBehaviour::Simple(SimpleBehaviour::AllowJumping),
                "TakedownTargetableClearOnEnd"=>LocalBehaviour::Simple(SimpleBehaviour::ClearTakeDownTargetOnEnd),
                "SetWaitingToReactFlagOnBegin"=>LocalBehaviour::Simple(SimpleBehaviour::SetWaiting),
                "SetSimpleTimer"=>LocalBehaviour::Simple(SimpleBehaviour::SetTimer {
                    timer:crate::living_world_pedestrian_state::timer_type(fields.iter().find(|f|f.key=="timerName")
                        .map_or("TimedRangeRandom",|f|f.value)).ok_or("BigBlack timer invalid")?,
                    length:marquee::float(fields,"length",0.)?,
                }),
                "ChangeNavModifierSetting"=>LocalBehaviour::Modifier {
                    kind:match fields.iter().find(|f|f.key=="modifier").ok_or("BigBlack modifier absent")?.value {
                        "ChaseGlue"=>crate::pedestrian_navigation_modifiers::ModifierKind::ChaseGlue,
                        "Pedestrian"=>crate::pedestrian_navigation_modifiers::ModifierKind::General,
                        "SkaterAvoidance"=>crate::pedestrian_navigation_modifiers::ModifierKind::Skater,
                        _=>return Err("BigBlack modifier unknown".into()),
                    },change_to:marquee::boolean(fields,"set_to",false)?,previous:false,
                },
                name if delegate!=AiDelegate::BigBlack=>LocalBehaviour::Marquee(marquee::MarqueeBehaviour::bind(name, fields)?),
                _=>{
                    // Shared callbacks retain the same original declaration parameters.
                    // Forward only an exact name/attribute match to the pedestrian binder.
                    let forwarded_occurrence=data::BEHAVIOURS.iter().enumerate().find_map(|(i,d)|{
                        if d.name!=declaration.name{return None;}
                        let other=marquee::attributes(&data::ATTRIBUTES,d.attributes).ok()?;
                        (other.len()==fields.len() && other.iter().zip(fields).all(|(a,b)|a.key==b.key&&a.value==b.value))
                            .then_some(i as u16)
                    });
                    entries.push(Entry {lifecycle,enabled:marquee::boolean(fields,"enabled",true)?,
                        local:None,forwarded_occurrence,fields});
                    continue;
                },
            };
            let enabled = marquee::boolean(fields, "enabled", true)?;
            entries.push(Entry { lifecycle, enabled, local: Some(local), forwarded_occurrence:None,fields });
        }
        Ok(Self { entries })
    }

    pub(crate) fn callback_fields(&self, occurrence:u16)->Result<&'static [crate::generated_action_load::Attribute],String>{
        self.entries.get(occurrence as usize).map(|e|e.fields).ok_or("callback occurrence absent".into())
    }
    pub(crate) fn forwarded_occurrence(&self, occurrence:u16)->u16{
        self.entries.get(occurrence as usize).and_then(|e|e.forwarded_occurrence).unwrap_or(occurrence)
    }
    pub(crate) fn lifecycle(&self, occurrence: u16) -> Result<lifecycle::Lifecycle, String> {
        self.entries.get(usize::from(occurrence)).map(|e| e.lifecycle)
            .ok_or_else(|| format!("pedestrian occurrence {occurrence} absent"))
    }

    pub(crate) fn enabled(&self, occurrence: u16) -> Result<bool, String> {
        self.entries.get(usize::from(occurrence)).map(|entry| entry.enabled)
            .ok_or_else(|| format!("pedestrian occurrence {occurrence} absent"))
    }

    /// CreateInstance is per entry, not per graph load. Reset native payloads
    /// each time an occurrence is re-entered; null factories have no payload.
    pub(crate) fn create_instance(&mut self, occurrence: u16) -> Result<LocalDispatch, String> {
        let entry = self.entries.get_mut(usize::from(occurrence))
            .ok_or("pedestrian instance occurrence absent")?;
        let Some(source) = entry.lifecycle.create else { return Ok(LocalDispatch::Handled); };
        match &mut entry.local {
            Some(LocalBehaviour::Patrol(instance)) if source == 0x82712148 =>
                *instance = crate::living_world_pedestrian_entity::patrol::PatrolExecution::new(),
            Some(LocalBehaviour::WatchWant {saved_speed,stopped,..}) if matches!(source,0x82712148|0x82712628)=>{
                *saved_speed=0.;*stopped=false;
            }
            Some(LocalBehaviour::Modifier { previous, .. }) if source == 0x827126f8 => *previous = false,
            Some(LocalBehaviour::DrawTazer { needs_spawn, .. }) if source == 0x827126f8 => *needs_spawn = false,
            Some(LocalBehaviour::Colliding { instance, .. }) if source == 0x82711d20 =>
                *instance = super::colliding::CollidingInstance::new(),
            Some(LocalBehaviour::Marquee(behaviour)) if behaviour.has_instance_factory(source) =>
                behaviour.create_instance(),
            _ => return Ok(LocalDispatch::Nonlocal { source }),
        }
        Ok(LocalDispatch::Handled)
    }

    pub(crate) fn dispatch_collision<S: super::colliding::CollidingServices>(
        &mut self, occurrence: u16, phase: Phase, entity: &mut PedestrianConstruction,
        dt: f32, services: &mut S,
        world: &mut crate::physical_player_assembly::PhysicalAssemblyWorld,
    ) -> Result<LocalDispatch, String> {
        let entry = self.entries.get_mut(usize::from(occurrence))
            .ok_or("pedestrian collision occurrence absent")?;
        let source = match phase { Phase::Begin => entry.lifecycle.begin,
            Phase::Update => entry.lifecycle.update, Phase::End => entry.lifecycle.end };
        let Some(source) = source else { return Ok(LocalDispatch::Handled); };
        let Some(LocalBehaviour::Colliding { behaviour, instance }) = &mut entry.local else {
            return Ok(LocalDispatch::Nonlocal { source });
        };
        match phase {
            Phase::Begin => behaviour.begin(instance, entity, services)?,
            Phase::Update => behaviour.update(instance, entity, dt, services)?,
            Phase::End => behaviour.end(instance, entity, world)?,
        }
        Ok(LocalDispatch::Handled)
    }

    /// Borrow the world only for End; Begin/Update use the same live frame.
    pub(crate) fn dispatch_collision_frame(&mut self,occurrence:u16,phase:Phase,
        entity:&mut PedestrianConstruction,dt:f32,frame:&mut super::backend::frame::FrameServices<'_>,
    )->Result<(),String>{
        let entry=self.entries.get_mut(occurrence as usize).ok_or("collision occurrence absent")?;
        let Some(LocalBehaviour::Colliding{behaviour,instance})=&mut entry.local else{
            return Err("collision callback does not own Colliding instance".into());
        };
        match phase{
            Phase::Begin=>behaviour.begin(instance,entity,frame),
            Phase::Update=>behaviour.update(instance,entity,dt,frame),
            Phase::End=>behaviour.end(instance,entity,frame.world),
        }
    }
    pub(crate) fn dispatch_draw_tazer(&mut self,occurrence:u16,phase:Phase,
        entity:&mut PedestrianConstruction,frame:&mut super::backend::frame::FrameServices<'_>,
    )->Result<(),String>{
        let entry=self.entries.get_mut(occurrence as usize).ok_or("DrawTazer occurrence absent")?;
        let Some(LocalBehaviour::DrawTazer{want,needs_spawn})=&mut entry.local else{return Err("DrawTazer instance absent".into());};
        match phase{
            Phase::Begin=>{
                frame.request_channel(entity,"TazerInto")?;
                *needs_spawn=entity.held_hand_prop.is_none();
                let state=entity.state.as_mut().ok_or("DrawTazer AI state absent")?;
                state.tazer_state=1;
                state.set_timer(37,f32::from_bits(0x3e08_3127)).map_err(|e|e.to_string())?;
                state.flags[4]|=0x80;state.flags[1]|=0x20;
                state.taze_result_target=*want as i32;state.flags[4]&=!0x40;
            }
            Phase::Update=>{
                if *needs_spawn && entity.state.as_ref().ok_or("DrawTazer AI state absent")?.timer(37)<=0.{
                    entity.hand_prop_attributes=Some(frame.vault.instance(0xec8b_7565_46e2_26f6,
                        skate2_formats::attrib::string_hash64("tazr")));
                    entity.state.as_mut().unwrap().flags[3]|=2;
                    *needs_spawn=false;
                }
            }
            Phase::End=>return Err("DrawTazer End must use native empty callback".into()),
        }
        Ok(())
    }
    /// The marquee callbacks that read the frame: ProTrackSkater Update 0x82717548,
    /// PhotographSkater Update 0x82717018 and ReviewingPictures Update 0x827078f8.
    pub(crate) fn dispatch_marquee_frame(&mut self, occurrence: u16, phase: Phase,
        entity: &mut PedestrianConstruction, dt: f32,
        frame: &mut super::backend::frame::FrameServices<'_>,
    ) -> Result<(), String> {
        let entry = self.entries.get_mut(usize::from(occurrence)).ok_or("marquee occurrence absent")?;
        let Some(LocalBehaviour::Marquee(behaviour)) = &mut entry.local else {
            return Err("marquee callback does not own a marquee behaviour".into());
        };
        behaviour.dispatch_frame(phase, entity, dt, frame)
    }

    /// Session dispatch with the same retained navigation/RAIN/waypoint owners
    /// as frame movement. Resolve only source-identified callbacks here.
    pub(crate) fn dispatch_with_targeting<N: crate::reckoning_numeric::ReckoningNumeric,
        A: crate::pedestrian_navigation::wander::WanderAvoidVolumes>(
        &mut self, occurrence: u16, phase: Phase, entity: &mut PedestrianConstruction,
        facts: &crate::pedestrian_navigation::mesh::NavigationEntityFacts,
        world: &mut crate::living_world_pedestrian_entity::targeting::TargetingWorld<'_, A>,
        numeric: &mut N,
    ) -> Result<LocalDispatch, String> where N::Error: std::fmt::Debug {
        let pending = self.dispatch(occurrence, phase, entity)?;
        let LocalDispatch::Nonlocal { source } = pending else { return Ok(pending); };
        match source {
            0x826ff0e0 | 0x826ff1a8 => entity.begin_chase_navigation(facts, world, numeric)?,
            0x82722f50 => super::locomotion::wander_on_road(entity, facts, world, numeric)?,
            0x826ef650 => super::locomotion::check_for_road_target(entity, facts, world, numeric)?,
            0x826ecbe8 => super::locomotion::use_crosswalk(entity, facts, world)?,
            0x826ed228 => entity.begin_flee(facts, world, numeric)?,
            0x826ed288 => entity.update_flee(facts, world, numeric)?,
            0x826ed398 => entity.ignore_avoid_target = crate::living_world_chase::TargetableId::INVALID,
            0x826fe358 => {
                let entry = self.entries.get_mut(usize::from(occurrence))
                    .ok_or("Patrol occurrence absent")?;
                let Some(LocalBehaviour::Patrol(execution)) = &mut entry.local else {
                    return Err("Patrol callback without its occurrence instance".into());
                };
                entity.begin_patrol(execution, facts, world, numeric)?;
            }
            0x826eb108 => super::locomotion::begin_wander(entity, facts, world, numeric)?,
            0x826eccb0 => super::locomotion::begin_no_road_wander(entity, facts, world, numeric)?,
            0x826ef5c8 => super::locomotion::check_target_timeout(entity, facts, world, numeric)?,
            0x826eed40 => entity.invalidate_navigation_targets(facts, world, numeric)?,
            _ => return Ok(LocalDispatch::Nonlocal { source }),
        }
        Ok(LocalDispatch::Handled)
    }

    pub(crate) fn dispatch(&mut self, occurrence: u16, phase: Phase,
        entity: &mut PedestrianConstruction,
    ) -> Result<LocalDispatch, String> {
        let entry = self.entries.get_mut(usize::from(occurrence))
            .ok_or_else(|| format!("pedestrian occurrence {occurrence} absent"))?;
        let source = match phase {
            Phase::Begin => entry.lifecycle.begin,
            Phase::Update => entry.lifecycle.update,
            Phase::End => entry.lifecycle.end,
        };
        // Only a proven native825187c0 callback may bypass dispatch.
        let Some(source) = source else { return Ok(LocalDispatch::Handled); };
        let handled = match &mut entry.local {
            Some(LocalBehaviour::WatchWant { .. }) => false,
            Some(LocalBehaviour::DrawTazer { .. }) => false,
            Some(LocalBehaviour::Colliding { .. }) => false, // Requires collision service owners.
            Some(LocalBehaviour::Patrol(execution)) => match phase {
                Phase::Begin => false, // Requires the real targeting frame.
                Phase::Update => { entity.update_patrol()?; true }
                Phase::End => { entity.end_patrol(execution)?; true }
            },
            Some(LocalBehaviour::Wander) => match phase {
                Phase::Update => { super::locomotion::update_wander(entity); true }
                // Begin needs the actual mesh/RAIN/RNG targeting frame.
                _ => false,
            },
            Some(LocalBehaviour::Modifier { kind, change_to, previous }) => {
                let modifiers = entity.navigation_modifiers.as_mut()
                    .ok_or("ChangeNavModifierSetting requires modifiers")?;
                match phase {
                    // Begin826ff568 captures actual enabled state before set;
                    // End826ff608 restores that instance's captured value.
                    Phase::Begin => { *previous = modifiers.enabled(*kind); modifiers.set_enabled(*kind, *change_to); }
                    Phase::End => modifiers.set_enabled(*kind, *previous),
                    Phase::Update => return Err("modifier native Update is not empty".into()),
                }
                true
            }
            Some(LocalBehaviour::Simple(simple)) => match phase {
                Phase::Begin => simple.begin(entity)?,
                Phase::End => simple.end(entity)?,
                Phase::Update => false,
            },
            Some(LocalBehaviour::Velocity(velocity)) => {
                match phase {
                    Phase::Update => velocity.update(entity.navigation_mut()?),
                    Phase::End => velocity.end(entity.navigation_mut()?),
                    Phase::Begin => return Err("SuggestVelocity native Begin is not empty".into()),
                }
                true
            }
            Some(LocalBehaviour::Marquee(behaviour)) => behaviour.dispatch(phase, entity)?,
            None => false,
        };
        Ok(if handled { LocalDispatch::Handled } else { LocalDispatch::Nonlocal { source } })
    }
}
