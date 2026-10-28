//! RestFromChase Begin827172c0/End826eb218 over the existing intent owner.
use crate::{
    living_world_pedestrian_entity::PedestrianConstruction,
    pedestrian_adjust_owners::HeadTrackingMode,
    pedestrian_intent_packet::IntentStage,
    play_animation::FastString30,
};

fn hash() -> u32 {
    // Dynamic initializer82f60078; same hash implementation as collision intent.
    skate2_formats::hash::sk8_hash_value_32(b"chasertired")
}
fn tired_stage() -> Result<IntentStage, String> {
    let mut stage = IntentStage::default();
    stage.add_intent(FastString30::encode("Tired").expect("native intent name"))?;
    stage.rotate_to_target = false;
    Ok(stage)
}

pub(crate) fn begin(entity: &mut PedestrianConstruction) -> Result<(), String> {
    let first = tired_stage()?;
    let queue = entity.intent_queue.as_mut().ok_or("rest requires intent queue")?;
    let state = entity.state.as_mut().ok_or("rest requires AI state")?;
    let packet = queue.create_monitored(state, hash())?;
    queue.packet_mut(packet)?.add_stage(first)?;
    let mut last = tired_stage()?;
    last.add_intent(FastString30::encode("StopTired").expect("native intent name"))?;
    queue.packet_mut(packet)?.add_stage(last)?;
    queue.suggest(packet, &mut entity.animation.as_mut()
        .ok_or("rest requires animation input")?.ai_input)?;
    entity.head_tracking.as_mut().ok_or("rest requires head tracking")?
        .push(HeadTrackingMode::None)?;
    // IChaser slots172/168: ClearExhaustion82e6fd28 then SetResting82e6fd50.
    state.exhaustion = 0.;
    state.flags[1] |= 2;
    // GetRestTimerDefault82e6f228 reads the entity collection at callback time.
    let attributes = &entity.resources.as_ref().ok_or("rest requires resources")?.entity;
    if attributes.is_null() { return Err("rest timer collection absent".into()); }
    let length = f32::from_bits(attributes.try_scalar_word(0x5960_4f99_8608_081e, 0)
        .map_err(|e| e.to_string())?.unwrap_or(0));
    state.set_timer(4, length).map_err(|e| e.to_string())
}

pub(crate) fn end(entity: &mut PedestrianConstruction) -> Result<(), String> {
    let queue = entity.intent_queue.as_mut().ok_or("rest requires intent queue")?;
    let state = entity.state.as_mut().ok_or("rest requires AI state")?;
    if let Some(&packet) = state.intent_monitors.get(&hash()) {
        queue.packet_mut(packet)?.advance_stage()?;
    }
    // Release may transfer the packet to queue ownership; it does not cancel
    // its StopTired stage or remove it from the active queue.
    queue.release_monitored(state, hash())?;
    entity.head_tracking.as_mut().ok_or("rest requires head tracking")?.pop();
    state.flags[1] &= !2;
    Ok(())
}
