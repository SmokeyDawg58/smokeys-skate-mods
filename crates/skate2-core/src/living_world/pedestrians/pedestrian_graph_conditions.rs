//! Common pedestrian condition constructors/direct callbacks, not skater leaves.
use crate::{living_world_pedestrian_state::{PedestrianStateStorage, timer_type},
    pedestrian_graph_data as data};

/// MoodMan::GetMoodResultEnumFromString82ce6d58 uses case-insensitive names.
pub(crate) fn want_type(name: &str) -> Option<usize> {
    ["none", "angrychase", "warn", "taunt", "returngreet", "greet",
     "startconversation", "alertto", "joinchase", "throwhandprop", "flee",
     "startle", "taze", "nearbycollisionreaction", "slamreaction"]
        .iter().position(|candidate| name.eq_ignore_ascii_case(candidate))
}

pub(crate) fn attribute(condition: u16, key: &str) -> Result<Option<&'static str>, String> {
    let declaration = data::CONDITIONS.get(usize::from(condition))
        .ok_or_else(|| format!("invalid pedestrian condition {condition}"))?;
    let section = declaration.attributes;
    if section.count == 0 { return Ok(None); }
    let fields = data::ATTRIBUTES.get(usize::from(section.index)..usize::from(section.index)+usize::from(section.count))
        .ok_or_else(|| format!("invalid pedestrian condition attributes {condition}"))?;
    Ok(fields.iter().find(|field| field.key == key).map(|field| field.value))
}

pub(crate) enum CommonCondition {
    BeingHonkedAt,
    Want(usize),
    Timer(u32),
    MonitoredIntent(u32),
}

/// Concrete predicates whose native interface resolves to this pedestrian's
/// collision/NavRig/RAIN owners. None means a different condition class, never
/// a false substitute. Context authority is checked by the graph dispatcher.
pub(crate) fn entity_condition(name: &str,
    entity: &crate::living_world_pedestrian_entity::PedestrianConstruction,
) -> Result<Option<bool>, String> {
    use crate::living_world_rain::spatial::RainItemKind;
    Ok(Some(match name {
        //826eadf8 asserts a collision owner then reads byte8.
        "NeedsToBeginColliding" => entity.collision.as_ref()
            .ok_or("NeedsToBeginColliding requires pedestrian collision")?
            .required_to_begin_colliding,
        //82707690 likewise reads byte10 of that same collision owner.
        "IsPedestrianColliding" => entity.collision.as_ref()
            .ok_or("IsPedestrianColliding requires pedestrian collision")?.colliding,
        //82701268 -> virtual Pedestrian::IsOnRoadNetwork82e6e378.
        // A RAIN road/intersection alone is insufficient: current cell cost
        // must pass the pedestrian override's <=0 rejection first.
        "IsOnRoad" => {
            let score = entity.nav_rig.as_ref().ok_or("IsOnRoad requires navigation rig")?
                .mesh_info.cell.score_with_threshold(0.);
            if score <= 0. { false } else {
                entity.rain_data.is_some_and(|item|
                    matches!(item.kind, RainItemKind::Road | RainItemKind::Intersection))
            }
        }
        //82701160 -> LWBasicEntity82e4c0d8; no cell-score test here.
        "IsOnIntersection" => entity.rain_data.is_some_and(|item|
            item.kind == RainItemKind::Intersection),
        _ => return Ok(None),
    }))
}
impl CommonCondition {
    /// Native constructors826f1920/826f1b28/826f1a28. Returning None delegates
    /// another class to its concrete owner; it does not evaluate it as false.
    pub(crate) fn bind(index: u16) -> Result<Option<Self>, String> {
        let name = data::CONDITIONS.get(usize::from(index))
            .ok_or_else(|| format!("invalid pedestrian condition {index}"))?.name;
        Ok(Some(match name {
            "IsBeingHonkedAt" => Self::BeingHonkedAt,
            "HasSpecificWantThatNeedsToBeAddressed" => {
                let name = attribute(index, "want")?.ok_or("missing pedestrian want")?;
                Self::Want(want_type(name).ok_or_else(|| format!("invalid pedestrian want {name}"))?)
            }
            "SimpleTimerExpired" => {
                let name = attribute(index, "timerName")?.unwrap_or("TimedRangeRandom");
                Self::Timer(timer_type(name).ok_or_else(|| format!("invalid pedestrian timer {name}"))?)
            }
            "HasMonitoredIntent" => {
                let name = attribute(index, "intentName")?.filter(|s| !s.is_empty())
                    .ok_or("missing monitored pedestrian intent name")?;
                Self::MonitoredIntent(skate2_formats::hash::sk8_hash_value_32(name.as_bytes()))
            }
            _ => return Ok(None),
        }))
    }
    /// Direct callbacks826f2968/826f29a8/826f28d8. A monitor entry is NOT proof
    /// that its packet is still in the live intent queue.
    pub(crate) fn evaluate<E>(&self, state: &PedestrianStateStorage,
        mut packet_in_queue: impl FnMut(u64) -> Result<bool, E>) -> Result<bool, E>
    {
        Ok(match *self {
            // 826f1138: entity AI state +0xc80, addi 1 / cntlzw.
            // Every vehicle ID except the native -1 sentinel is active.
            Self::BeingHonkedAt => state.honking_vehicle_id != -1,
            Self::Want(index) => state.wants[index].active,
            Self::Timer(kind) => state.timer(kind) <= 0.,
            Self::MonitoredIntent(hash) => match state.intent_monitors.get(&hash) {
                Some(&packet) => packet_in_queue(packet)?,
                None => false,
            },
        })
    }
}
