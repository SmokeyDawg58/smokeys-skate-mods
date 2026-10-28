//! PedestrianColliding826eabe8, CreateInstance82711d20, Begin82716648,
//! Update8271ae50 and End827074c0. Uses the retained collision and intent owners.
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    living_world_chase::TargetableId,
    living_world_pedestrian_entity::PedestrianConstruction,
    pedestrian_adjust_owners::HeadTrackingMode,
    pedestrian_collision::PedestrianBodyKind,
    pedestrian_intent_packet::IntentStage,
    physical_player_assembly::PhysicalAssemblyWorld,
    play_animation::FastString30,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum CollisionForce { Knockdown = 0, Standing = 1 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum CollisionDirection { Front = 0, Back = 1, Left = 2, Right = 3 }
impl CollisionDirection {
    fn intent(self) -> &'static str { match self {
        Self::Front => "FromFront", Self::Back => "FromBack", Self::Left => "FromLeft", Self::Right => "FromRight",
    } }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct PedestrianCollisionInfo {
    pub(crate) force: CollisionForce,
    pub(crate) direction: CollisionDirection,
}
impl PedestrianCollisionInfo {
    /// Entity MemStore Init82a6de38/FreeBlock82a6df68 zero the allocation;
    /// Pedestrian ctor has no stores at+A98/A9C. First real producer is
    /// OnEntityCollision82e95c80/84, after contact/velocity classification.
    pub(crate) fn new() -> Self { Self { force: CollisionForce::Knockdown, direction: CollisionDirection::Front } }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CollisionState { Standing, Falling, Recovering, Complete }
pub(crate) struct CollidingInstance {
    pub(crate) recovery_timer: f32,
    pub(crate) state: CollisionState,
    pub(crate) recovery_stage: usize,
    pub(crate) recovery_attempts: u32,
    pub(crate) getup_direction: CollisionDirection,
    pub(crate) disabled_modifier: bool,
}
impl CollidingInstance {
    pub(crate) fn new() -> Self { Self { recovery_timer: 1., state: CollisionState::Standing,
        recovery_stage: 0, recovery_attempts: 0, getup_direction: CollisionDirection::Front,
        disabled_modifier: false } }
}
pub(crate) struct CollidingBehaviour {
    pub(crate) warn_chance: f32,
    pub(crate) broadcast_radius: f32,
}
#[derive(Clone, Copy)]
pub(crate) struct CollisionMoodEvent {
    pub(crate) event_type: i32,
    pub(crate) position: [f32; 4],
    pub(crate) perpetrator: TargetableId,
    pub(crate) victim: TargetableId,
    pub(crate) magnitude: f32,
    pub(crate) exposure: f32,
}
/// Concrete external owners, called in native order. Animation data must be
/// the same selected regular/override RefSpec used by Pedestrian RemapAnim.
pub(crate) trait CollidingServices {
    fn animation_data(&mut self, entity: &PedestrianConstruction) -> Result<AttributeRuntimeInstance, String>;
    fn transitioning(&mut self, entity: &PedestrianConstruction) -> Result<bool, String>;
    fn animation_time(&mut self, entity: &PedestrianConstruction) -> Result<f32, String>;
    fn target_entity_exists(&mut self, target: TargetableId) -> Result<bool, String>;
    fn hand_prop_drop_probability(&mut self, entity: &PedestrianConstruction) -> Result<f32, String>;
    fn throw_hand_prop_away(&mut self, entity: &mut PedestrianConstruction) -> Result<(), String>;
    fn update_player_collision_stats(&mut self, entity: &PedestrianConstruction) -> Result<(), String>;
    fn broadcast_mood(&mut self, event: CollisionMoodEvent) -> Result<(), String>;
    fn guard_knocked_down(&mut self, entity_id: u32) -> Result<(), String>;
    fn career_mode(&mut self) -> Result<bool, String>;
    /// GetChasee->GetSelf->QueryInterface(IActor), followed by IsLocalPlayer.
    fn chasee_actor_is_local(&mut self, entity: &PedestrianConstruction) -> Result<Option<bool>, String>;
    fn grant_good_samaritan(&mut self) -> Result<(), String>;
}
fn hash() -> u32 { skate2_formats::hash::sk8_hash_value_32(b"collisionintent") }
fn add(stage: &mut IntentStage, name: &str) -> Result<(), String> {
    stage.add_intent(FastString30::encode(name).map_err(|e| format!("collision intent: {e:?}"))?)
}
fn stage(names: &[&str]) -> Result<IntentStage, String> {
    let mut result = IntentStage::default();
    for name in names { add(&mut result, name)?; }
    result.rotate_to_target = false;
    Ok(result)
}
fn packet(entity: &PedestrianConstruction) -> Result<u64, String> {
    entity.state.as_ref().ok_or("collision requires AI state")?.intent_monitors.get(&hash()).copied()
        .ok_or_else(|| "native collision monitored intent assertion".to_string())
}
fn scalar(attributes: &AttributeRuntimeInstance, key: u64) -> Result<f32, String> {
    Ok(f32::from_bits(attributes.try_scalar_word(key, 0).map_err(|e| e.to_string())?.unwrap_or(0)))
}
const RECOVERY_TIME: u64 = 0xad3c_483f_0c9d_ad67;
const UP_SHIFT: u64 = 0xa062_9152_63ce_2bec;

impl CollidingBehaviour {
    pub(crate) fn bind(occurrence: u16) -> Result<Self, String> {
        use crate::living_world_guard_graph::binding::float;
        Ok(Self { warn_chance: float(occurrence, "warnChance", Some(0.7))?,
            broadcast_radius: float(occurrence, "broadcastradius", Some(20.))? })
    }
    ///82714b80 consumes the original collision event bit only after dispatch.
    fn process_flags<S: CollidingServices>(&self, entity: &mut PedestrianConstruction,
        services: &mut S) -> Result<(), String> {
        let flags = entity.state.as_ref().ok_or("collision AI state absent")?.collision_flags;
        if flags & 0x10 == 0 || flags & 0x80 == 0 { return Ok(()); }
        if flags & 0x40 != 0 { services.update_player_collision_stats(entity)?; }
        // Reload after the native stats callback.
        let state = entity.state.as_ref().ok_or("collision AI state absent")?;
        entity.state_out.flags = (entity.state_out.flags & !0x40) | (state.collision_flags & 0x40);
        let event = CollisionMoodEvent { event_type: 10,
            position: entity.core.as_ref().ok_or("collision core absent")?.transform[3],
            perpetrator: state.collider, victim: TargetableId { kind: 1, id: entity.id as i32 },
            magnitude: 1., exposure: 1. };
        //82711df8 does not use its force argument, warn chance or radius.
        services.broadcast_mood(event)?;
        entity.state.as_mut().ok_or("collision AI state absent")?.collision_flags &= !0x10;
        Ok(())
    }
    pub(crate) fn begin<S: CollidingServices>(&self, instance: &mut CollidingInstance,
        entity: &mut PedestrianConstruction, services: &mut S) -> Result<(), String> {
        entity.collision.as_ref().ok_or("collision Begin requires collision object")?;
        let id = entity.intent_queue.as_mut().ok_or("collision Begin requires intent queue")?
            .create_monitored(entity.state.as_mut().ok_or("collision Begin requires state")?, hash())?;
        let modifier = &mut entity.navigation_modifiers.as_mut().ok_or("collision Begin requires modifiers")?.skater;
        if modifier.enabled { modifier.enabled = false; instance.disabled_modifier = true; }
        self.process_flags(entity, services)?;
        let info = entity.collision_info;
        let force_name = match info.force { CollisionForce::Knockdown => "Knockdown", CollisionForce::Standing => "Standing" };
        entity.intent_queue.as_mut().unwrap().packet_mut(id)?.add_stage(stage(&["Collision", force_name, info.direction.intent()])?)?;
        entity.state.as_mut().unwrap().flags[1] &= !4;
        match info.force {
            CollisionForce::Standing => {
                instance.state = CollisionState::Standing;
                let target = entity.state.as_ref().unwrap().collider;
                entity.head_tracking.as_mut().ok_or("collision Begin requires head tracking")?
                    .push_targetable(HeadTrackingMode::TargetableCenterOfMass, target,
                        || services.target_entity_exists(target))?;
                if entity.state.as_ref().unwrap().flags[2] & 2 != 0 {
                    let chance = services.hand_prop_drop_probability(entity)?;
                    let word = crate::native_skater_random::rw_math::global_next().map_err(|e| format!("collision Bernoulli RNG: {e:?}"))?;
                    if crate::native_skater_random::rw_math::bernoulli_choice(chance, word) {
                        services.throw_hand_prop_away(entity)?;
                    }
                }
            }
            CollisionForce::Knockdown => {
                let attributes = services.animation_data(entity)?;
                let timer = scalar(&attributes, RECOVERY_TIME)?;
                instance.state = CollisionState::Falling;
                instance.recovery_timer = timer;
                instance.getup_direction = entity.collision_info.direction;
                entity.intent_queue.as_mut().unwrap().packet_mut(id)?.add_stage(stage(&["Collision", "Recover"])?)?;
                instance.recovery_stage = entity.intent_queue.as_mut().unwrap().packet_mut(id)?.add_stage(stage(&["Collision"])?)?;
                let attributes = services.animation_data(entity)?;
                let shift = scalar(&attributes, UP_SHIFT)?;
                entity.spatial_adjust.as_mut().ok_or("collision Begin requires spatial adjustment")?.up_shift = shift;
                entity.head_tracking.as_mut().ok_or("collision Begin requires head tracking")?.push(HeadTrackingMode::None)?;
                entity.state.as_mut().unwrap().flags[1] |= 4;
                let guard = entity.security_guard.is_some();
                if guard { services.guard_knocked_down(entity.id)?; }
                if services.career_mode()? && entity.state.as_ref().unwrap().collision_flags & 0x40 != 0 {
                    if guard || entity.core.as_ref().ok_or("collision Begin requires core")?.entity_key == skate2_formats::attrib::string_hash64("security_street01") {
                        if services.chasee_actor_is_local(entity)? == Some(false) { services.grant_good_samaritan()?; }
                    }
                }
                services.throw_hand_prop_away(entity)?;
            }
        }
        entity.intent_queue.as_mut().unwrap().suggest(id,
            &mut entity.animation.as_mut().ok_or("collision Begin requires animation input")?.ai_input)?;
        entity.collision.as_mut().unwrap().colliding = true;
        entity.switch_navigation_mode(2);
        Ok(())
    }
    pub(crate) fn update<S: CollidingServices>(&self, instance: &mut CollidingInstance,
        entity: &mut PedestrianConstruction, dt: f32, services: &mut S) -> Result<(), String> {
        entity.collision.as_ref().ok_or("collision Update requires collision object")?;
        match instance.state {
            CollisionState::Standing => {
                entity.collision.as_mut().unwrap().required_to_begin_colliding = false;
                if complete(entity, packet(entity)?)? {
                    entity.collision.as_mut().unwrap().colliding = false;
                    instance.state = CollisionState::Complete;
                }
            }
            CollisionState::Falling => {
                instance.recovery_timer -= dt;
                let id = packet(entity)?;
                //Raw bge82714cac: unordered follows expired path.
                if !(instance.recovery_timer >= 0.) {
                    let collision = entity.collision.as_mut().unwrap();
                    let retry = collision.required_to_begin_colliding && !collision.forced_to_recover;
                    collision.required_to_begin_colliding = false;
                    if retry {
                        instance.recovery_timer = scalar(&services.animation_data(entity)?, RECOVERY_TIME)?;
                    } else {
                        self.process_flags(entity, services)?;
                        instance.state = CollisionState::Recovering;
                        entity.intent_queue.as_mut().ok_or("collision queue absent")?.packet_mut(id)?.advance_stage()?;
                        entity.spatial_adjust.as_mut().ok_or("collision spatial adjustment absent")?.up_shift = 0.;
                    }
                    instance.recovery_attempts = instance.recovery_attempts.wrapping_add(1);
                } else {
                    let packet = entity.intent_queue.as_mut().ok_or("collision queue absent")?.packet_mut(id)?;
                    if packet.active_stage_number() != 0 { packet.set_active_stage(0)?; }
                }
                entity.collision.as_mut().unwrap().num_recovery_attempts = instance.recovery_attempts;
            }
            CollisionState::Recovering => self.update_recovering(instance, entity, services)?,
            CollisionState::Complete => return Err("native PedestrianColliding Update invalid Complete state".into()),
        }
        Ok(())
    }
    fn update_recovering<S: CollidingServices>(&self, instance: &mut CollidingInstance,
        entity: &mut PedestrianConstruction, services: &mut S) -> Result<(), String> {
        let id = packet(entity)?;
        entity.state.as_mut().ok_or("collision AI state absent")?.collision_flags |= 0x20;
        let collision = entity.collision.as_mut().unwrap();
        if !collision.required_to_begin_colliding || collision.forced_to_recover {
            if complete(entity, id)? {
                entity.collision.as_mut().unwrap().colliding = false;
                instance.state = CollisionState::Complete;
            }
            return Ok(());
        }
        collision.required_to_begin_colliding = false;
        let point = getup_point(instance.getup_direction, entity, services)?;
        {
            let stage = entity.intent_queue.as_mut().ok_or("collision queue absent")?.packet_mut(id)?.stage_mut(instance.recovery_stage)?;
            stage.clear_intents(); add(stage, "Collision")?; add(stage, "RecoveryCollision")?;
        }
        self.process_flags(entity, services)?;
        {
            let stage = entity.intent_queue.as_mut().unwrap().packet_mut(id)?.stage_mut(instance.recovery_stage)?;
            match point {
                GetupPoint::Lying => { add(stage, "LyingDown")?; add(stage, instance.getup_direction.intent())?; }
                GetupPoint::Crouched | GetupPoint::Standing => {
                    add(stage, if point == GetupPoint::Crouched { "Crouched" } else { "Standing" })?;
                    add(stage, entity.collision_info.direction.intent())?;
                    instance.recovery_attempts = 0;
                    instance.getup_direction = entity.collision_info.direction;
                }
                GetupPoint::Transitioning => {}
            }
        }
        entity.intent_queue.as_mut().unwrap().packet_mut(id)?.advance_stage()?;
        instance.state = CollisionState::Falling;
        instance.recovery_timer = scalar(&services.animation_data(entity)?, RECOVERY_TIME)?;
        let shift = scalar(&services.animation_data(entity)?, UP_SHIFT)?;
        entity.spatial_adjust.as_mut().ok_or("collision spatial adjustment absent")?.up_shift = shift;
        Ok(())
    }
    pub(crate) fn end(&self, instance: &CollidingInstance, entity: &mut PedestrianConstruction,
        world: &mut PhysicalAssemblyWorld) -> Result<(), String> {
        entity.collision.as_ref().ok_or("collision End requires collision object")?;
        entity.intent_queue.as_mut().ok_or("collision End requires queue")?
            .release_monitored(entity.state.as_mut().ok_or("collision End requires state")?, hash())?;
        entity.head_tracking.as_mut().ok_or("collision End requires head tracking")?.pop();
        if instance.disabled_modifier {
            entity.navigation_modifiers.as_mut().ok_or("collision End requires modifiers")?.skater.enabled = true;
        }
        let collision = entity.collision.as_mut().unwrap();
        collision.colliding = false; collision.required_to_begin_colliding = false;
        collision.forced_to_recover = false; collision.num_recovery_attempts = 0;
        if collision.do_world_collisions_only_in_hifi()? && entity.core.as_ref().ok_or("collision End requires core")?.current_losim != 0 {
            collision.remove_body_from_world(PedestrianBodyKind::World, world)?;
        }
        let state = entity.state.as_mut().unwrap();
        state.flags[1] &= !4; state.collision_flags |= 0x20;
        Ok(())
    }
}
impl PedestrianConstruction {
    /// OnEntityCollision82e95c60..d74 suffix, reached ONLY after original
    /// contact classification, live Part linear/angular velocity thresholds,
    /// direction calculation and knockdown/standing acceptance. This method
    /// does not replace those physical producers with a hit Boolean.
    pub(crate) fn record_accepted_full_collision(&mut self,
        force: CollisionForce, direction: CollisionDirection,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
    ) -> Result<(), String> {
        let collision = self.collision.as_mut().ok_or("accepted contact requires collision owner")?;
        if !collision.colliding || force == CollisionForce::Knockdown {
            self.collision_info = PedestrianCollisionInfo { force, direction };
        }
        collision.required_to_begin_colliding = true;
        let state = self.state.as_mut().ok_or("accepted contact requires AI state")?;
        state.collision_flags &= !0x20;
        let force_recovery_with_skater = cvars.try_pedestrian_force_recovery_with_skater()
            .map_err(|e| format!("pedestrian forced recovery cvar: {e:?}"))?;
        if force_recovery_with_skater || !collision.colliding_with_skater {
            let max_recovery_attempts = cvars.try_pedestrian_max_recovery_attempts()
                .map_err(|e| format!("pedestrian recovery attempts cvar: {e:?}"))?;
            collision.forced_to_recover = collision.num_recovery_attempts > max_recovery_attempts;
        }
        if state.flags[2] & 0x40 != 0 { state.flags[2] &= !0xc0; }
        Ok(())
    }
}
fn complete(entity: &PedestrianConstruction, id: u64) -> Result<bool, String> {
    Ok(entity.intent_queue.as_ref().ok_or("collision queue absent")?.packet(id)?
        .complete(entity.animation.as_ref().ok_or("collision animation absent")?.ai_input.major_intent_complete))
}
#[derive(PartialEq)]
enum GetupPoint { Lying, Crouched, Standing, Transitioning }
fn getup_point<S: CollidingServices>(direction: CollisionDirection, entity: &PedestrianConstruction,
    services: &mut S) -> Result<GetupPoint, String> {
    if entity.animation.is_none() || services.transitioning(entity)? { return Ok(GetupPoint::Transitioning); }
    let attributes = services.animation_data(entity)?;
    let key = match direction {
        CollisionDirection::Front => 0xe37f_0040_9dea_b4d2,
        CollisionDirection::Back => 0x74c0_b722_17a7_704e,
        CollisionDirection::Left | CollisionDirection::Right => 0x0162_b7a9_b0c0_86ee,
    };
    let intervals = if let Some(data) = attributes.try_get_data(key, 0).map_err(|e| e.to_string())? {
        if data.bytes().len() != 8 { return Err("native recovery collision intervals require two floats".into()); }
        [f32::from_be_bytes(data.bytes()[..4].try_into().unwrap()), f32::from_be_bytes(data.bytes()[4..].try_into().unwrap())]
    } else { [0., 0.] }; // Generated accessor DefaultDataArea(8).
    let time = services.animation_time(entity)?;
    //Raw bge826fd850/86c: unordered falls through to the earlier interval.
    Ok(if !(time >= intervals[0]) { GetupPoint::Lying }
        else if !(time >= intervals[1]) { GetupPoint::Crouched } else { GetupPoint::Standing })
}
