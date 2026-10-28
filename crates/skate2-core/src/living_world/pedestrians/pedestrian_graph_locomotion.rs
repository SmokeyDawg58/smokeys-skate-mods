//! Ordinary pedestrian graph locomotion leaves over the actual Navigation.
use crate::{living_world_guard_graph::binding, pedestrian_navigation::PedestrianNavigation};

/// Wander Begin826eb108. The road/random choice and initial Seek remain in
/// the same native targeting owner used by ordinary per-frame navigation.
pub(crate) fn begin_wander<N: crate::reckoning_numeric::ReckoningNumeric,
    A: crate::pedestrian_navigation::wander::WanderAvoidVolumes>(
    entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction,
    facts: &crate::pedestrian_navigation::mesh::NavigationEntityFacts,
    world: &mut crate::living_world_pedestrian_entity::targeting::TargetingWorld<'_, A>,
    numeric: &mut N,
) -> Result<(), String> where N::Error: std::fmt::Debug {
    entity.state.as_mut().ok_or("Wander requires AI state")?.honking_vehicle_id = -1;
    entity.switch_navigation_mode(1);
    entity.navigation_mut()?.target_mode = crate::pedestrian_navigation::NavigationTargetMode::Auto;
    entity.switch_targeting_mode(2, facts, world, numeric)?;
    entity.navigation_rig_enabled = true;
    Ok(())
}

/// NoRoadWander Begin826eccb0 sets the rig flag BEFORE switching owners.
pub(crate) fn begin_no_road_wander<N: crate::reckoning_numeric::ReckoningNumeric,
    A: crate::pedestrian_navigation::wander::WanderAvoidVolumes>(
    entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction,
    facts: &crate::pedestrian_navigation::mesh::NavigationEntityFacts,
    world: &mut crate::living_world_pedestrian_entity::targeting::TargetingWorld<'_, A>,
    numeric: &mut N,
) -> Result<(), String> where N::Error: std::fmt::Debug {
    entity.navigation_rig_enabled = true;
    entity.switch_navigation_mode(1);
    entity.switch_targeting_mode(6, facts, world, numeric)
}

/// Wander Update826eb198: optional GetNavigation, then virtual168(2.0).
pub(crate) fn update_wander(entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction) {
    if let Some(navigation) = entity.navigation_if_present_mut() { navigation.seek_speed = 2.; }
}

/// CheckForTargetTimeout826ef5c8 calls Pedestrian::OnPathingFailure(4),
/// not SwitchTargetingMode(4). Raw fcmpu/bgt also falls through for NaN.
pub(crate) fn check_target_timeout<N: crate::reckoning_numeric::ReckoningNumeric,
    A: crate::pedestrian_navigation::wander::WanderAvoidVolumes>(
    entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction,
    facts: &crate::pedestrian_navigation::mesh::NavigationEntityFacts,
    world: &mut crate::living_world_pedestrian_entity::targeting::TargetingWorld<'_, A>,
    numeric: &mut N,
) -> Result<(), String> where N::Error: std::fmt::Debug {
    let timer = entity.state.as_ref().ok_or("target timeout requires AI state")?.timer(0x29);
    if timer > 0. { return Ok(()); }
    entity.on_target_pathing_failure(4, facts, world, numeric)
}

/// CheckForRoadTarget826ef650 refreshes the retained road iterator even when
/// NoRoad is active. It resets WanderRoadCheck only after the query completes.
pub(crate) fn check_for_road_target<N: crate::reckoning_numeric::ReckoningNumeric,
    A: crate::pedestrian_navigation::wander::WanderAvoidVolumes>(
    entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction,
    facts: &crate::pedestrian_navigation::mesh::NavigationEntityFacts,
    world: &mut crate::living_world_pedestrian_entity::targeting::TargetingWorld<'_, A>,
    numeric: &mut N,
) -> Result<(), String> where N::Error: std::fmt::Debug {
    if entity.state.as_ref().ok_or("road check requires AI state")?.timer(0x27) > 0. {
        return Ok(());
    }
    let tuning = world.cvars.try_pedestrian_road_tuning().map_err(|e| format!("road tuning: {e:?}"))?;
    let facing = entity.core.as_ref().ok_or("road check requires core")?.transform[2];
    let mut context = crate::pedestrian_navigation::road::RoadNavigationContext {
        rain: world.rain, mesh: &world.mesh.store, random: world.random,
        position: facts.position, facing, entity_id: entity.id,
    };
    let duration = entity.target_iterators.road.check_for_target(&mut context, tuning, numeric)?;
    let state = entity.state.as_mut().ok_or("road check requires AI state")?;
    if let Some(duration) = duration { state.set_timer(0x29, duration).map_err(|e| e.to_string())?; }
    state.set_timer(0x27, 15.).map_err(|e| e.to_string())
}

/// UseCrossWalk826ecbe8 speeds up to4 only when an existing iterator's
/// CanProceedToTarget returns false. A null iterator retains the default2.
pub(crate) fn use_crosswalk<A: crate::pedestrian_navigation::wander::WanderAvoidVolumes>(
    entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction,
    facts: &crate::pedestrian_navigation::mesh::NavigationEntityFacts,
    world: &mut crate::living_world_pedestrian_entity::targeting::TargetingWorld<'_, A>,
) -> Result<(), String> {
    if entity.navigation_if_present_mut().is_none() { return Ok(()); }
    let mut speed = 2.;
    if entity.current_target_iterator() == Some(crate::living_world_pedestrian_entity::targeting::TargetIteratorKind::Road) {
        let tuning = world.cvars.try_pedestrian_road_tuning().map_err(|e| format!("road tuning: {e:?}"))?;
        let facing = entity.core.as_ref().ok_or("crosswalk requires core")?.transform[2];
        let mut context = crate::pedestrian_navigation::road::RoadNavigationContext {
            rain: world.rain, mesh: &world.mesh.store, random: world.random,
            position: facts.position, facing, entity_id: entity.id,
        };
        if !entity.target_iterators.road.can_proceed(&mut context, tuning)? { speed = 4.; }
    }
    // All seven other concrete native iterator vtables use the true-returning
    // folded function826c6d80 at slot20, so they retain speed2.
    entity.navigation_mut()?.seek_speed = speed;
    Ok(())
}

/// WanderOnRoad82722f50 uses the entity's retained RAIN road, all its segment
/// quads in source order, and the first containing triangle (including edges).
pub(crate) fn wander_on_road<N: crate::reckoning_numeric::ReckoningNumeric,
    A: crate::pedestrian_navigation::wander::WanderAvoidVolumes>(
    entity: &mut crate::living_world_pedestrian_entity::PedestrianConstruction,
    facts: &crate::pedestrian_navigation::mesh::NavigationEntityFacts,
    world: &mut crate::living_world_pedestrian_entity::targeting::TargetingWorld<'_, A>,
    numeric: &mut N,
) -> Result<(), String> where N::Error: std::fmt::Debug {
    let item = entity.rain_data.ok_or("WanderOnRoad requires entity RAIN data")?;
    if item.kind != crate::living_world_rain::spatial::RainItemKind::Road { return Ok(()); }
    let road = world.rain.roads.get(&item.id).ok_or("WanderOnRoad retained road absent")?;
    let mut corners = Vec::with_capacity(road.segments.len());
    for segment in &road.segments {
        let mut quad = [[0.; 4]; 4];
        for (index, corner) in quad.iter_mut().enumerate() {
            *corner = segment.record.vector(80 + index * 16)
                .map_err(|e| format!("WanderOnRoad segment: {e:?}"))?;
        }
        corners.push(quad);
    }
    let point = facts.position;
    for quad in corners {
        let contains = [[0, 1, 2], [0, 2, 3]].into_iter().any(|indices| {
            crate::living_world_mesh_store::asset::point_in_triangle_xz(
                [point[0], point[1], point[2]],
                indices.map(|i| [quad[i][0], quad[i][1], quad[i][2]]))
        });
        if !contains { continue; }
        let first = std::array::from_fn(|i| point[i] - quad[1][i]);
        let second = std::array::from_fn(|i| point[i] - quad[3][i]);
        let first_distance = numeric.dot3(first, first).map_err(|e| format!("road distance: {e:?}"))?;
        let second_distance = numeric.dot3(second, second).map_err(|e| format!("road distance: {e:?}"))?;
        let target = if first_distance > second_distance { quad[3] } else { quad[1] };
        entity.target_explicit(target, crate::pedestrian_navigation::NavigationTargetMode::ExplicitNoSeekNoAstar,
            facts, world, numeric)?;
        entity.navigation_mut()?.seek_speed = 4.;
        break;
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) struct SuggestVelocity {
    linear: f32,
    override_animation: bool,
    override_ramp: bool,
}
impl SuggestVelocity {
    pub(crate) fn from_attributes(fields:&[crate::generated_action_load::Attribute])->Result<Self,String>{
        let get=|key|fields.iter().find(|a|a.key==key).map(|a|a.value);
        let flag=|key|match get(key).unwrap_or("false"){"true"|"1"=>Ok(true),"false"|"0"=>Ok(false),_=>Err("SuggestVelocity boolean attribute".to_string())};
        Ok(Self{linear:get("linear").ok_or("SuggestVelocity linear required")?.parse::<f32>().map_err(|e|e.to_string())?,
            override_animation:flag("overrideAnimation")?,override_ramp:flag("overrideRamp")?})
    }
    /// Native ctor826eb2e0: required GetFloat(linear), optional booleans false.
    pub(crate) fn bind(occurrence: u16) -> Result<Self, String> {
        Ok(Self {
            linear: binding::float(occurrence, "linear", None)?,
            override_animation: binding::boolean(occurrence, "overrideAnimation", false)?,
            override_ramp: binding::boolean(occurrence, "overrideRamp", false)?,
        })
    }
    /// Update826eb3a8 uses Navigation virtual slots168/48/172, in that order.
    /// SetDebugTuningVelocityOverride82e6bbd8 sets both value and valid flag;
    /// SetRampedSpeed82e6bb40 assigns directly. No clamp or dt scaling here.
    pub(crate) fn update(self, navigation: &mut PedestrianNavigation) {
        navigation.seek_speed = self.linear;
        if self.override_animation { navigation.velocity_override = Some(self.linear); }
        if self.override_ramp { navigation.ramped_seek_speed = self.linear; }
    }

    /// End826eb470 unconditionally calls Navigation virtual52, verified as
    /// ClearDebugTuningVelocityOverride82e6bbe8. This also clears a previous
    /// override when this occurrence did not itself set overrideAnimation.
    /// Seek speed and ramped speed are deliberately retained by the original.
    pub(crate) fn end(self, navigation: &mut PedestrianNavigation) {
        navigation.velocity_override = None;
    }
}
