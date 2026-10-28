//! Native six embedded navigation modifiers, Pedestrian82e97748.
//! Zero values below follow entity MemStore Init/FreeBlock clearing, not
//! inferred constructor writes. Init always binds MESH navigation.
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    pedestrian_intent_packet::{IntentPacket, queue::IntentQueue},
    pedestrian_navigation::PedestrianNavigation,
    simulation_assembly_backend::constraint_session::physical_island::contact_owners::SimulationContactEntityOwner,
    reckoning_numeric::{ReckoningNumeric, try_length},
};
type EntityOwner = SimulationContactEntityOwner;
#[path = "pedestrian_skater_avoidance.rs"]
pub(crate) mod skater_avoidance;
#[path = "pedestrian_general_avoidance.rs"]
pub(crate) mod general_avoidance;
#[path = "pedestrian_dmo_avoidance.rs"]
pub(crate) mod dmo_avoidance;
#[path = "pedestrian_vehicle_avoidance.rs"]
pub(crate) mod vehicle_avoidance;
#[path = "pedestrian_best_nav_ray.rs"]
pub(crate) mod best_nav_ray;
#[path = "pedestrian_navigation_intent.rs"]
pub(crate) mod intent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(crate) enum ModifierKind { ChaseGlue, General, Skater, Vehicle, DynamicObject, BigBlack }
pub(crate) const ORDER: [ModifierKind; 6] = [ModifierKind::ChaseGlue,
    ModifierKind::General, ModifierKind::Skater, ModifierKind::Vehicle,
    ModifierKind::DynamicObject, ModifierKind::BigBlack];

/// Same semantic recommendation flags appear at different native offsets.
#[derive(Default)]
pub(crate) struct RecommendationFlags {
    pub(crate) active: bool,
    pub(crate) target: bool,
    pub(crate) speed: bool,
    pub(crate) intent: bool,
}
#[derive(Default)]
pub(crate) struct GeneralModifier {
    pub(crate) target: [f32; 4],
    pub(crate) speed: f32,
    pub(crate) predict_time: f32,
    pub(crate) turn_mask: u32,
    pub(crate) closest_point_to_me: [f32; 4],
    pub(crate) squared_distance_to_closest: f32,
    pub(crate) target_entity: Option<EntityOwner>,
    pub(crate) recommended_packet: Option<u64>,
    pub(crate) flags: RecommendationFlags,
    pub(crate) enabled: bool,
}
#[derive(Default)]
pub(crate) struct SkaterModifier {
    pub(crate) closest_point_on_path: [f32; 4],
    pub(crate) closest_point_to_me: [f32; 4],
    pub(crate) squared_distance_to_closest: f32,
    pub(crate) time_until_closest: f32,
    pub(crate) outer_radius: f32,
    pub(crate) inner_radius: f32,
    pub(crate) forward_angle: f32,
    pub(crate) predict_time: f32,
    pub(crate) minimum_speed: f32,
    pub(crate) big_avoid_speed: f32,
    pub(crate) target_entity: Option<EntityOwner>,
    pub(crate) recommended_packet: Option<u64>,
    pub(crate) active: bool,
    pub(crate) has_intent: bool,
    pub(crate) enabled: bool,
}
#[derive(Default)]
pub(crate) struct DynamicObjectModifier {
    pub(crate) target_entity: Option<EntityOwner>,
    pub(crate) recommended_packet: Option<u64>,
    pub(crate) enabled: bool,
    pub(crate) active: bool,
    pub(crate) has_intent: bool,
    pub(crate) outer_radius: f32,
    pub(crate) inner_radius: f32,
    pub(crate) forward_angle: f32,
    pub(crate) predict_time: f32,
    pub(crate) minimum_speed: f32,
    pub(crate) big_avoid_speed: f32,
    pub(crate) minimum_annoyance_speed: f32,
    pub(crate) avoid_direction: [f32; 4],
    pub(crate) closest_corner_normal_projection: f32,
    pub(crate) entity_normal_projection: f32,
}
/// PDB geotest::tIntersectInfo. This retains results; it adds no line solver.
#[derive(Default)]
pub(crate) struct VehicleIntersection {
    pub(crate) info: u32,
    pub(crate) distance1: f32,
    pub(crate) distance2: f32,
    pub(crate) predict_time_distance1: f32,
    pub(crate) predict_time_distance2: f32,
    pub(crate) line_normal1: [f32; 4],
    pub(crate) line_normal2: [f32; 4],
    pub(crate) line_start1: [f32; 4],
    pub(crate) line_start2: [f32; 4],
    pub(crate) closest_point1: [f32; 4],
    pub(crate) closest_point2: [f32; 4],
    pub(crate) line_distance1: f32,
    pub(crate) line_distance2: f32,
    pub(crate) path_intersect: bool,
}
pub(crate) struct VehicleModifier {
    pub(crate) target: [f32; 4],
    pub(crate) speed: f32,
    pub(crate) predict_time: f32,
    pub(crate) min_inter_entity_distance: f32,
    pub(crate) stop_distance: f32,
    pub(crate) max_turn_angle_degrees: f32,
    pub(crate) dodge_distance: f32,
    pub(crate) distance: f32,
    pub(crate) quadrant: u32,
    pub(crate) target_quadrant: u32,
    pub(crate) intersection: VehicleIntersection,
    pub(crate) packet: u64,
    pub(crate) turn_mask: u32,
    pub(crate) target_entity: Option<EntityOwner>,
    pub(crate) flags: RecommendationFlags,
    pub(crate) enabled: bool,
}
pub(crate) struct BigBlackModifier {
    pub(crate) target: [f32; 4],
    pub(crate) speed: f32,
    pub(crate) max_turn_angle_degrees: f32,
    pub(crate) influence_radius: f32,
    pub(crate) medium_avoid_radius: f32,
    pub(crate) high_avoid_radius: f32,
    pub(crate) avoidance_location: [f32; 4],
    pub(crate) distance: f32,
    pub(crate) packet: u64,
    pub(crate) target_entity: Option<EntityOwner>,
    pub(crate) big_black: Option<EntityOwner>,
    pub(crate) flags: RecommendationFlags,
    pub(crate) enabled: bool,
    pub(crate) last_deviation_was_left: bool,
}
pub(crate) struct ChaseGlueModifier {
    pub(crate) target: [f32; 4],
    pub(crate) explicit_target: [f32; 4],
    pub(crate) speed: f32,
    pub(crate) max_turn_angle_degrees: f32,
    pub(crate) optimal_distance: f32,
    pub(crate) activation_range: f32,
    pub(crate) max_speed: f32,
    pub(crate) glue_timeout: f32,
    pub(crate) glue_timer: f32,
    pub(crate) dt: f32,
    pub(crate) packet: u64,
    pub(crate) target_entity: Option<EntityOwner>,
    pub(crate) flags: RecommendationFlags,
    pub(crate) glue_active: bool,
    pub(crate) enabled: bool,
    pub(crate) has_explicit_target: bool,
}
pub(crate) struct NavigationModifiers {
    pub(crate) chase_glue: ChaseGlueModifier,
    pub(crate) general: GeneralModifier,
    pub(crate) skater: SkaterModifier,
    pub(crate) vehicle: VehicleModifier,
    pub(crate) dynamic_object: DynamicObjectModifier,
    pub(crate) big_black: BigBlackModifier,
    pub(crate) initialized_with_mesh: bool,
}
impl NavigationModifiers {
    /// Three original unmanaged embedded packets share the SAME IntentQueue
    /// arena as suggestions; IDs preserve identity across queue removal.
    /// Caller retains the queue on failure, preserving preceding allocations.
    pub(crate) fn new(queue: &mut IntentQueue) -> Result<Self, String> {
        let vehicle_packet = queue.allocate(IntentPacket::new(false))?;
        let big_black_packet = queue.allocate(IntentPacket::new(false))?;
        let chase_packet = queue.allocate(IntentPacket::new(false))?;
        Ok(Self {
            chase_glue: ChaseGlueModifier { target: [0.;4], explicit_target: [0.;4],
                speed: 0., max_turn_angle_degrees: 0., optimal_distance: 0.,
                activation_range: 0., max_speed: 0., glue_timeout: 0., glue_timer: 0.,
                dt: 0., packet: chase_packet, target_entity: None,
                flags: RecommendationFlags::default(), glue_active: false,
                enabled: true, has_explicit_target: false },
            general: GeneralModifier { turn_mask: 119, enabled: true, ..Default::default() },
            skater: SkaterModifier { enabled: true, ..Default::default() },
            vehicle: VehicleModifier { target: [0.;4], speed: 0., predict_time: 0.,
                min_inter_entity_distance: 0., stop_distance: 0., max_turn_angle_degrees: 0.,
                dodge_distance: 0., distance: 0., quadrant: 0, target_quadrant: 0,
                intersection: VehicleIntersection { info: 2, ..Default::default() }, packet: vehicle_packet,
                turn_mask: 119, target_entity: None, flags: RecommendationFlags::default(), enabled: true },
            dynamic_object: DynamicObjectModifier { enabled: true, ..Default::default() },
            big_black: BigBlackModifier { target: [0.;4], speed: 0., max_turn_angle_degrees: 0.,
                influence_radius: 0., medium_avoid_radius: 0., high_avoid_radius: 0.,
                avoidance_location: [0.;4], distance: 0., packet: big_black_packet,
                target_entity: None, big_black: None, flags: RecommendationFlags::default(),
                enabled: true, last_deviation_was_left: false },
            initialized_with_mesh: false,
        })
    }
    /// All six native Init methods bind mMeshNavigation, independently of
    /// active Navigation. Rust borrows it at use sites instead of self-pointers.
    pub(crate) fn initialize_with_mesh(&mut self, _mesh: &PedestrianNavigation) {
        self.initialized_with_mesh = true;
    }
    /// ResetNavModifiersToDefault82e6d460 preserves writes if a later read fails.
    pub(crate) fn reset_enabled_to_attributes(&mut self, active_navigation: &AttributeRuntimeInstance)
        -> Result<(), String> {
        for (kind, offset) in [(ModifierKind::ChaseGlue,4), (ModifierKind::General,2),
            (ModifierKind::Skater,1), (ModifierKind::DynamicObject,3),
            (ModifierKind::Vehicle,0), (ModifierKind::BigBlack,5)] {
            let value = active_navigation.try_cached_layout_field(offset,1).map_err(|e| e.to_string())?
                .ok_or("ResetNavModifiersToDefault navigation layout absent")?;
            self.set_enabled(kind, value.bytes()[0] != 0);
        }
        Ok(())
    }
    pub(crate) fn set_enabled(&mut self, kind: ModifierKind, enabled: bool) {
        match kind {
            ModifierKind::ChaseGlue => self.chase_glue.enabled = enabled,
            ModifierKind::General => self.general.enabled = enabled,
            ModifierKind::Skater => self.skater.enabled = enabled,
            ModifierKind::Vehicle => self.vehicle.enabled = enabled,
            ModifierKind::DynamicObject => self.dynamic_object.enabled = enabled,
            ModifierKind::BigBlack => self.big_black.enabled = enabled,
        }
    }
    pub(crate) fn enabled(&self, kind: ModifierKind) -> bool {
        match kind {
            ModifierKind::ChaseGlue => self.chase_glue.enabled,
            ModifierKind::General => self.general.enabled,
            ModifierKind::Skater => self.skater.enabled,
            ModifierKind::Vehicle => self.vehicle.enabled,
            ModifierKind::DynamicObject => self.dynamic_object.enabled,
            ModifierKind::BigBlack => self.big_black.enabled,
        }
    }
    /// EvaluateNavModifiers resets each owner before any candidate traversal.
    /// General preserves turn mask; Vehicle specifically restores119.
    pub(crate) fn reset_evaluation(&mut self) {
        self.chase_glue.reset();
        self.general.flags = RecommendationFlags::default();
        self.skater.active = false; self.skater.has_intent = false;
        self.vehicle.flags = RecommendationFlags::default(); self.vehicle.turn_mask = 119;
        self.dynamic_object.active = false; self.dynamic_object.has_intent = false;
        self.big_black.flags = RecommendationFlags::default();
    }
    pub(crate) fn active(&self, kind: ModifierKind) -> bool {
        match kind {
            ModifierKind::ChaseGlue => self.chase_glue.flags.active,
            ModifierKind::General => self.general.flags.active,
            ModifierKind::Skater => self.skater.active,
            ModifierKind::Vehicle => self.vehicle.flags.active,
            ModifierKind::DynamicObject => self.dynamic_object.active,
            ModifierKind::BigBlack => self.big_black.flags.active,
        }
    }
    pub(crate) fn turn_mask(&self, kind: ModifierKind) -> u32 {
        match kind { ModifierKind::General => self.general.turn_mask,
            ModifierKind::Vehicle => self.vehicle.turn_mask, _ => 119 }
    }
    pub(crate) fn suggested_speed(&self, kind: ModifierKind) -> Option<f32> {
        match kind {
            ModifierKind::ChaseGlue => self.chase_glue.flags.speed.then_some(self.chase_glue.speed),
            ModifierKind::General => self.general.flags.speed.then_some(self.general.speed),
            ModifierKind::Vehicle => self.vehicle.flags.speed.then_some(self.vehicle.speed),
            ModifierKind::BigBlack => self.big_black.flags.speed.then_some(self.big_black.speed),
            _ => None,
        }
    }
    /// Returns native target and max rotation together. General uses constant
    /// ZERO from shared body82e6c588, not a guessed normal rotation limit.
    pub(crate) fn suggested_target(&self, kind: ModifierKind) -> Option<([f32;4],f32)> {
        match kind {
            ModifierKind::ChaseGlue => self.chase_glue.flags.target.then_some((self.chase_glue.target,self.chase_glue.max_turn_angle_degrees)),
            ModifierKind::General => self.general.flags.target.then_some((self.general.target,0.)),
            ModifierKind::Vehicle => self.vehicle.flags.target.then_some((self.vehicle.target,self.vehicle.max_turn_angle_degrees)),
            ModifierKind::BigBlack => self.big_black.flags.target.then_some((self.big_black.target,self.big_black.max_turn_angle_degrees)),
            _ => None,
        }
    }
    pub(crate) fn suggested_intent(&self, kind: ModifierKind) -> Result<Option<u64>,String> {
        let (has,packet) = match kind {
            ModifierKind::ChaseGlue => (false,None),
            ModifierKind::General => (self.general.flags.intent,self.general.recommended_packet),
            ModifierKind::Skater => (self.skater.has_intent,self.skater.recommended_packet),
            ModifierKind::Vehicle => (self.vehicle.flags.intent,Some(self.vehicle.packet)),
            ModifierKind::DynamicObject => (self.dynamic_object.has_intent,self.dynamic_object.recommended_packet),
            ModifierKind::BigBlack => (self.big_black.flags.intent,Some(self.big_black.packet)),
        };
        if !has { return Ok(None); }
        packet.map(Some).ok_or("modifier GetSuggestedIntent native null-packet assertion".into())
    }
}
impl ChaseGlueModifier {
    /// Reset82e6c4a8 deliberately preserves target, tuning, packet, and enabled.
    pub(crate) fn reset(&mut self) {
        self.speed = 0.; self.max_turn_angle_degrees = 4.25;
        self.target_entity = None; self.flags = RecommendationFlags::default();
        self.has_explicit_target = false; self.glue_timer = 0.; self.glue_active = false;
    }
    pub(crate) fn set_explicit_target(&mut self, target: [f32;4]) {
        self.explicit_target = target; self.has_explicit_target = true;
    }
    /// RecommendTarget82e77360. Caller performs the native IsEnabled gate.
    /// Position/speed are read from the bound MESH navigation, not the entity's
    /// currently selected navigation. Distance includes all three XYZ lanes.
    pub(crate) fn recommend<N: ReckoningNumeric>(&mut self, mesh: &PedestrianNavigation,
        target_entity: EntityOwner, target_position: [f32;4], numeric: &mut N)
        -> Result<(),N::Error> {
        self.target_entity = Some(target_entity);
        self.flags.speed = false; self.flags.active = false; self.flags.target = false;
        if self.has_explicit_target {
            self.flags.active = true; self.flags.target = true; self.target = self.explicit_target;
            return Ok(());
        }
        let delta: [f32;4] = std::array::from_fn(|i| target_position[i] - mesh.position[i]);
        let distance = try_length(delta,numeric)?;
        let speed = mesh.speed();
        if self.activation_range > distance {
            if distance > self.optimal_distance {
                self.flags.speed = true; self.flags.active = true; self.speed = self.max_speed;
            }
            if self.optimal_distance > distance {
                if !self.glue_active { self.glue_active = true; self.glue_timer = self.glue_timeout; }
                if self.glue_timer > 0. {
                    self.flags.speed = true; self.flags.active = true; self.speed = speed * 0.75;
                }
            }
        }
        if self.glue_active { self.glue_timer -= self.dt; }
        Ok(())
    }
}
