//! Same Actor `SetUpPhysics825ff6c0` transport, after the manager's body ACS Sync.
//! The complete entry includes retained external/settings data and pointer
//! publication. This does not execute the later physical Update1 phase.
use super::{AnimOutConstructionStage, AnimOutPhysInOwner, PhysicsAnimationAttribute};
use crate::{
    actor_construction::{ActorConstruction, ActorConstructionError},
    actor_graph_construction::{ActorGraphIntentError, ActorMgAttribute, ActorSkaterMotionGraph},
    play_animation::FastString30,
};
use skate2_formats::andale::{AndaleAttributeValue, AndaleRuntimeAttribute};
use std::sync::{Arc, Mutex};

#[path = "actor_animation_physics_suffix.rs"]
mod suffix;

#[cfg(test)]
#[path = "actor_animation_physics_publication_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum AnimationPhysicsPublicationError {
    Actor(ActorConstructionError),
    Missing(&'static str),
    Poisoned,
    IncompleteDestination,
    Intent(ActorGraphIntentError),
    AttributeCapacity,
    InvalidAttributeName,
    ForeignMotion,
    ForeignAnimation,
    Reset(super::AnimOutConstructionError),
    Physical(crate::actor_construction::physical_mount::ActorPhysicalMountError),
    Processed(crate::skeleton_processed_contact::animation_input::ProcessedAnimationInputError),
    Settings(crate::players_backend_settings::PlayersSettingsError),
    World(crate::simulation_assembly_backend::constraint_session::world_services::PhysicalWorldServicesError),
}
impl std::fmt::Display for AnimationPhysicsPublicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(e) => write!(f, "animation physics Actor: {e}"),
            Self::Missing(field) => write!(f, "animation physics missing {field}"),
            Self::Intent(e) => write!(f, "animation physics intent assignment: {e:?}"),
            Self::Reset(e) => write!(f, "animation physics reset: {e:?}"),
            Self::Physical(e) => write!(f, "animation physics physical owner: {e}"),
            Self::Processed(e) => write!(f, "animation physics consumer: {e}"),
            Self::Settings(e) => write!(f, "animation physics settings: {e}"),
            Self::World(e) => write!(f, "animation physics world: {e}"),
            other => write!(f, "animation physics publication: {other:?}"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum AnimationPhysicsPrepareError<E> {
    Publication(AnimationPhysicsPublicationError),
    Physics(crate::actor_animation_construction::physics_update_data::locomotion::AnimationLocomotionError<E>),
}
impl<E> From<AnimationPhysicsPublicationError> for AnimationPhysicsPrepareError<E> {
    fn from(error: AnimationPhysicsPublicationError) -> Self {
        Self::Publication(error)
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for AnimationPhysicsPrepareError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Publication(error) => write!(f, "{error}"),
            Self::Physics(error) => write!(f, "{error}"),
        }
    }
}

impl PhysicsAnimationAttribute {
    /// MG conversion825fdcd4..825fdd20. Only four payload bytes are assigned.
    fn motion(attribute: &ActorMgAttribute) -> Self {
        let mut payload = [None; 24];
        for (target, byte) in payload.iter_mut().zip(attribute.value.to_be_bytes()) {
            *target = Some(byte);
        }
        Self {
            payload,
            begin: -1.0,
            end: -1.0,
            name: attribute.name,
            flags: 6,
            kind: 0,
            sequence: Some(-1),
        }
    }

    fn animation(
        attribute: &AndaleRuntimeAttribute,
    ) -> Result<Self, AnimationPhysicsPublicationError> {
        let mut payload = [None; 24];
        let kind = match &attribute.value {
            AndaleAttributeValue::Float(value) => {
                for (target, byte) in payload.iter_mut().zip(value.to_be_bytes()) {
                    *target = Some(byte);
                }
                0
            }
            AndaleAttributeValue::Vector(value) => {
                for (target, byte) in payload
                    .iter_mut()
                    .zip(value.iter().flat_map(|v| v.to_be_bytes()))
                {
                    *target = Some(byte);
                }
                1
            }
            AndaleAttributeValue::Bytes24(value) => {
                payload = value.map(Some);
                3
            }
            AndaleAttributeValue::Unsupported { attribute_type, .. } => {
                i8::from_ne_bytes([*attribute_type])
            }
        };
        Ok(Self {
            payload,
            begin: attribute.begin_seconds,
            end: attribute.end_seconds,
            name: FastString30::encode(&attribute.name)
                .map_err(|_| AnimationPhysicsPublicationError::InvalidAttributeName)?,
            flags: attribute.status,
            kind,
            // Existing clip/tree projection has no sequence provenance. The
            // source assignment copies it, so unknown replaces any old value.
            sequence: None,
        })
    }
}

impl AnimOutPhysInOwner {
    /// Append using Attribute.operator=825f7658. Reusing vector storage keeps
    /// payload bytes which that type does not assign, including unknown bytes.
    fn append_attribute(
        &mut self,
        mut attribute: PhysicsAnimationAttribute,
    ) -> Result<(), AnimationPhysicsPublicationError> {
        if self.attribute_count == self.attributes.len() {
            return Err(AnimationPhysicsPublicationError::AttributeCapacity);
        }
        let copied = match attribute.kind {
            0 => 4,
            1 => 16,
            3 => 24,
            _ => 0,
        };
        if let Some(previous) = &self.attributes[self.attribute_count] {
            attribute.payload[copied..].copy_from_slice(&previous.payload[copied..]);
        }
        self.attributes[self.attribute_count] = Some(attribute);
        self.attribute_count += 1;
        Ok(())
    }

    pub(crate) fn attribute(&self, index: usize) -> Option<&PhysicsAnimationAttribute> {
        (index < self.attribute_count)
            .then(|| self.attributes[index].as_ref())
            .flatten()
    }
}

fn publish_motion<K>(
    motion: &mut ActorSkaterMotionGraph<K>,
    destination: &mut AnimOutPhysInOwner,
) -> Result<(), AnimationPhysicsPublicationError> {
    use AnimationPhysicsPublicationError as E;
    if destination.stage != AnimOutConstructionStage::Returned {
        return Err(E::IncompleteDestination);
    }
    motion
        .state
        .as_mut()
        .ok_or(E::Missing("Motion state"))?
        .transfer_physics_payloads(destination);
    let source = motion
        .intents
        .as_ref()
        .ok_or(E::Missing("Motion intents"))?;
    let target = destination
        .intents
        .as_mut()
        .ok_or(E::Missing("destination intents"))?;
    // fixed_hash_map.operator=825fd730 frees then reinserts in source traversal
    // order. Rebuilding collision chains differs from cloning their links.
    target.clear();
    for entry in source.iter() {
        target
            .insert_unique(entry.name, entry.value)
            .map_err(E::Intent)?;
    }
    destination.attribute_count = 0;
    for attribute in motion
        .state
        .as_ref()
        .expect("state already read")
        .attributes()
    {
        destination.append_attribute(PhysicsAnimationAttribute::motion(attribute))?;
    }
    Ok(())
}

impl ActorConstruction {
    /// `Actor::SetUpPhysics825ff6c0`. Concrete `SkaterAnim` vtable82302d28+88
    /// resolves call825ff710 to shared blr825187c0, so it has no extra body Sync.
    /// The earlier manager synchronization remains the caller's obligation.
    pub(crate) fn try_set_up_physics<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self,
        dt: f32,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut N,
        settings: &crate::players_backend_settings::PlayersBackendSettings,
    ) -> Result<(), AnimationPhysicsPrepareError<N::Error>> {
        self.try_prepare_animation_physics_outputs(dt, cvars, numeric)?;
        self.try_finish_animation_physics_outputs(settings)
            .map_err(Into::into)
    }

    /// Actual `ProcessAnimOut82e00030` on the cached physical pair, consuming
    /// its retained input pointer. This selected call is not Update1 execution.
    pub(crate) fn try_process_bound_animation_physics_input(
        &self,
    ) -> Result<(), AnimationPhysicsPublicationError> {
        use AnimationPhysicsPublicationError as E;
        let expected = self.published_animation_physics_input()?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let input = physical
            .owner()
            .animation_physics_input
            .lock()
            .map_err(|_| E::Poisoned)?
            .as_ref()
            .cloned()
            .ok_or(E::Missing("physical AnimOutPhysIn pointer"))?;
        if !Arc::ptr_eq(&input, &expected) {
            return Err(E::Missing("same Actor AnimOutPhysIn pointer"));
        }
        let processed = Arc::clone(
            physical
                .owner()
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let input = input.lock().map_err(|_| E::Poisoned)?;
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_process_animation_input(&input)
            .map_err(E::Processed)
    }

    /// Selected `SetPhysUpdateData` call825ff978, native setter82dfe688.
    /// The omitted Actor settings/external-input suffix must run before this
    /// in a full frame. Retain the actual pointer, never a payload snapshot.
    pub(crate) fn try_bind_animation_physics_input(
        &self,
    ) -> Result<(), AnimationPhysicsPublicationError> {
        use AnimationPhysicsPublicationError as E;
        let input = self.published_animation_physics_input()?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        *physical
            .owner()
            .animation_physics_input
            .lock()
            .map_err(|_| E::Poisoned)? = Some(input);
        Ok(())
    }

    /// SetUpPhysics825ff740..825ff7f0, with the caller's primitive numeric
    /// service. The physical interface call is a separate continuation.
    pub(crate) fn try_prepare_animation_physics_outputs<
        N: crate::reckoning_numeric::ReckoningNumeric,
    >(
        &self,
        dt: f32,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), AnimationPhysicsPrepareError<N::Error>> {
        use AnimationPhysicsPublicationError as E;
        let output = self.published_animation_physics_input()?;
        let animation = {
            let prefix = self.prefix().map_err(E::Actor)?;
            if !prefix.animation_published {
                return Err(E::Missing("published SkaterAnim").into());
            }
            let animation = Arc::clone(prefix.animation.as_ref().ok_or(E::Missing("SkaterAnim"))?);
            {
                let owner = animation.lock().map_err(|_| E::Poisoned)?;
                if !prefix.animation_interfaces[2]
                    .as_ref()
                    .is_some_and(|registered| Arc::ptr_eq(registered, owner.occurrence()))
                {
                    return Err(E::ForeignAnimation.into());
                }
            }
            animation
        };
        {
            let mut output = output.lock().map_err(|_| E::Poisoned)?;
            output.try_reset().map_err(E::Reset)?;
            animation
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_get_physics_update(&mut output, dt, cvars, numeric)
                .map_err(AnimationPhysicsPrepareError::Physics)?;
        }
        self.try_publish_animation_physics_outputs()
            .map_err(Into::into)
    }

    pub(crate) fn published_animation_physics_input(
        &self,
    ) -> Result<Arc<Mutex<AnimOutPhysInOwner>>, AnimationPhysicsPublicationError> {
        use AnimationPhysicsPublicationError as E;
        let prefix = self.prefix().map_err(E::Actor)?;
        if !prefix.animation_physics_input_published {
            return Err(E::Missing("published AnimOutPhysIn"));
        }
        Ok(Arc::clone(
            prefix
                .animation_physics_input
                .as_ref()
                .ok_or(E::Missing("AnimOutPhysIn"))?,
        ))
    }

    /// Selected native transport continuation. All data comes from this Actor's
    /// retained owners. A later tree-attribute failure retains the Motion drain,
    /// copied map and attributes already appended. No physical tick is implied.
    pub(crate) fn try_publish_animation_physics_outputs(
        &self,
    ) -> Result<(), AnimationPhysicsPublicationError> {
        use AnimationPhysicsPublicationError as E;
        let mut prefix = self.prefix().map_err(E::Actor)?;
        if !prefix.animation_physics_input_published {
            return Err(E::Missing("published AnimOutPhysIn"));
        }
        let output = Arc::clone(
            prefix
                .animation_physics_input
                .as_ref()
                .ok_or(E::Missing("AnimOutPhysIn"))?,
        );
        if !prefix.animation_published {
            return Err(E::Missing("published SkaterAnim"));
        }
        let animation = Arc::clone(prefix.animation.as_ref().ok_or(E::Missing("SkaterAnim"))?);
        if !prefix.motion_graph_published {
            return Err(E::Missing("published Motion graph"));
        }
        let motion = prefix
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.motion())
            .filter(|motion| motion.is_complete())
            .ok_or(E::Missing("constructed Motion graph"))?;
        if !prefix.graph_interfaces[1]
            .as_ref()
            .is_some_and(|registered| Arc::ptr_eq(registered, motion.occurrence()))
            || !prefix.graph_interfaces[2]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, motion.occurrence()))
        {
            return Err(E::ForeignMotion);
        }
        {
            let animation = animation.lock().map_err(|_| E::Poisoned)?;
            if !prefix.animation_interfaces[2]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, animation.occurrence()))
            {
                return Err(E::ForeignAnimation);
            }
        }
        let motion = prefix
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .filter(|motion| motion.is_complete())
            .ok_or(E::Missing("constructed Motion graph"))?;
        let mut output = output.lock().map_err(|_| E::Poisoned)?;
        publish_motion(motion, &mut output)?;
        // This is an outer Actor entry, never an animation callback. No ACS,
        // startup service or graph callback runs while these locks are held.
        // Release the prefix before borrowing animation again, matching body
        // dispatch's checked-owner lookup followed by animation-only work.
        drop(prefix);
        let animation = animation.lock().map_err(|_| E::Poisoned)?;
        let base = animation.base().ok_or(E::Missing("Animatable base"))?;
        for slot in base.tree_attributes.occupied_slots() {
            let attribute = slot.as_ref().ok_or(E::Missing("occupied tree attribute"))?;
            output.append_attribute(PhysicsAnimationAttribute::animation(attribute)?)?;
        }
        Ok(())
    }
}
