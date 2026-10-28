//! Selected physical-service projection of `LivingWorld::Manager`82721dc0.
//! The same workspace occurrence is copied into Actor's physical initialization.
//! Parked subsystem constructors and spatial queries remain unrecovered.
use crate::attribute_runtime::{
    AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use crate::physical_player_assembly::hilod_tail::IntersectionCapsule;
use std::sync::Arc;

// Same shared Random storage; constructor/reset call order remains unchanged.
use crate::native_skater_random::NativeSkaterRandom as ManagerRandom;

/// Absence of a recovered submanager is not a native NULL or empty collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParkedManagerState {
    Unrecovered,
}

/// These effects must use the session's actual random and resource owners.
pub(crate) trait LivingWorldConstructorEffects {
    type Error;
    /// Actual `rw::math::Random` call82722018, after constructor scalar stores.
    fn try_random_seed(&mut self) -> Result<u32, Self::Error>;
    /// Actual resource category `LivingWorld`, capsule radius1/halfheight1.
    /// None represents the real allocation result, not a skipped allocation.
    fn try_allocate_intersection_capsule(
        &mut self,
    ) -> Result<Option<Box<IntersectionCapsule>>, Self::Error>;
}

/// Meaningful retained Manager fields; this does not certify the full constructor.
pub(crate) struct LivingWorldPhysicalServices {
    pub(crate) parked_submanagers: ParkedManagerState,
    pub(crate) random: Option<ManagerRandom>,
    pub(crate) debug_attributes: Option<AttributeRuntimeInstance>,
    pub(crate) debug_layout: Option<AttributeRuntimeData>,
    pub(crate) debug_layout_uses_default: Option<bool>,
    pub(crate) spawn_message_registered: Option<bool>,
    pub(crate) unspawn_message_registered: Option<bool>,
    pub(crate) big_black_present: Option<bool>,
    pub(crate) actor_tracker_present: Option<bool>,
    pub(crate) can_trigger_alerts_chases: Option<bool>,
    pub(crate) valid_threat_local_skater: Option<bool>,
    pub(crate) valid_threat_ai_skaters: Option<bool>,
    pub(crate) valid_threat_pedestrians: Option<bool>,
    pub(crate) pedestrian_bind_pose_present: Option<bool>,
    pub(crate) pedestrian_hierarchy_present: Option<bool>,
    pub(crate) pedestrian_motion_graph_pool_present: Option<bool>,
    pub(crate) behaviour_evaluate_frame_timer: Option<u32>,
    pub(crate) intersection_capsule: Option<Box<IntersectionCapsule>>,
    attempted: bool,
    selected_prefix_complete: bool,
}
impl LivingWorldPhysicalServices {
    pub(crate) fn before_constructor() -> Self {
        Self {
            parked_submanagers: ParkedManagerState::Unrecovered,
            random: None,
            debug_attributes: None,
            debug_layout: None,
            debug_layout_uses_default: None,
            spawn_message_registered: None,
            unspawn_message_registered: None,
            big_black_present: None,
            actor_tracker_present: None,
            can_trigger_alerts_chases: None,
            valid_threat_local_skater: None,
            valid_threat_ai_skaters: None,
            valid_threat_pedestrians: None,
            pedestrian_bind_pose_present: None,
            pedestrian_hierarchy_present: None,
            pedestrian_motion_graph_pool_present: None,
            behaviour_evaluate_frame_timer: None,
            intersection_capsule: None,
            attempted: false,
            selected_prefix_complete: false,
        }
    }
    pub(crate) fn selected_prefix_complete(&self) -> bool {
        self.selected_prefix_complete
    }
    pub(crate) fn try_construct_selected_prefix<E: LivingWorldConstructorEffects>(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        effects: &mut E,
    ) -> Result<(), LivingWorldConstructorError<E::Error>> {
        use LivingWorldConstructorError as Error;
        if self.attempted {
            return Err(Error::AlreadyAttempted);
        }
        self.attempted = true;
        // Embedded parked managers are deliberately not presented as constructed.
        self.random = Some(ManagerRandom::reset()); // 82721e40
        self.debug_attributes = Some(vault.instance(0xff19_68a5_c467_bcf9, 0xf97b_9086_caa5_0c53));
        self.debug_layout = self
            .debug_attributes
            .as_ref()
            .expect("stored attributes")
            .try_cached_layout(0x84)
            .map_err(Error::Attribute)?;
        self.debug_layout_uses_default = Some(self.debug_layout.is_none());
        // Native fixed hash maps are empty after construction (capacities256/128).
        self.spawn_message_registered = Some(false);
        self.unspawn_message_registered = Some(false);
        self.can_trigger_alerts_chases = Some(false);
        self.big_black_present = Some(false);
        self.actor_tracker_present = Some(false);
        self.valid_threat_local_skater = Some(true);
        self.valid_threat_ai_skaters = Some(true);
        self.valid_threat_pedestrians = Some(true);
        self.pedestrian_bind_pose_present = Some(false);
        self.pedestrian_hierarchy_present = Some(false);
        self.pedestrian_motion_graph_pool_present = Some(false);
        self.behaviour_evaluate_frame_timer = Some(0);
        let seed = effects.try_random_seed().map_err(Error::Effect)?;
        self.random.as_mut().expect("reset random").set_seed(seed);
        self.intersection_capsule = effects
            .try_allocate_intersection_capsule()
            .map_err(Error::Effect)?;
        self.selected_prefix_complete = true;
        Ok(())
    }
}
#[derive(Debug)]
pub(crate) enum LivingWorldConstructorError<E> {
    AlreadyAttempted,
    Attribute(AttributeRuntimeError),
    Effect(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for LivingWorldConstructorError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => {
                f.write_str("LivingWorld selected constructor already attempted")
            }
            Self::Attribute(error) => write!(f, "LivingWorld debug attributes: {error}"),
            Self::Effect(error) => write!(f, "LivingWorld constructor effect: {error:?}"),
        }
    }
}
