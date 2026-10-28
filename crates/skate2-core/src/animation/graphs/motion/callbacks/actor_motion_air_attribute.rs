//! `HasAnimAttribute`82caa1d0 over the same cached `GetTreeAttr`82d79170 owner.
use super::{ActorConstruction, ActorIdentity, MotionUpdateError};
use crate::{motion_graph_entity_authority::BoundMotionGraphContext, play_animation::FastString30};
use skate2_formats::andale::AndaleRuntimeAttribute;
use std::sync::Arc;

impl ActorConstruction {
    pub(crate) fn motion_runtime_tree_attribute(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        name: FastString30,
    ) -> Result<Option<AndaleRuntimeAttribute>, MotionUpdateError> {
        self.check_motion_runtime_context(context)?;
        self.runtime_tree_attribute_from_checked_context(name)
    }

    /// Action's HasAnimAttribute resolves the same IAnimatable/GetTreeAttr
    /// owner after its own controller-context validation.
    pub(crate) fn runtime_tree_attribute_from_checked_context(
        &self,
        name: FastString30,
    ) -> Result<Option<AndaleRuntimeAttribute>, MotionUpdateError> {
        self.runtime_tree_attribute_with_requirement(name, false)
    }

    /// `IsDoneAirDismount::ActivationFunc` 82caefe0 asserts IAnimatable exists.
    pub(crate) fn runtime_required_tree_attribute_from_checked_context(
        &self,
        name: FastString30,
    ) -> Result<Option<AndaleRuntimeAttribute>, MotionUpdateError> {
        self.runtime_tree_attribute_with_requirement(name, true)
    }

    fn runtime_tree_attribute_with_requirement(
        &self,
        name: FastString30,
        required: bool,
    ) -> Result<Option<AndaleRuntimeAttribute>, MotionUpdateError> {
        use MotionUpdateError as E;
        let actor = self.prefix().map_err(E::Actor)?;
        let Some(interface) = actor.animation_interfaces[0].as_ref() else {
            return if required {
                Err(E::MissingOwner("Animatable interface"))
            } else {
                Ok(None)
            };
        };
        let owner = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
            .ok_or(E::MissingOwner("published Animatable"))?;
        let animation = owner.lock().map_err(|_| E::Lock)?;
        if !Arc::ptr_eq(interface, animation.occurrence())
            || !animation
                .entity()
                .is_same_issuance(&self.authority.entity_handle())
        {
            return Err(E::ForeignOwner("Animatable interface"));
        }
        let base = animation.base().ok_or(E::MissingOwner("Animatable base"))?;
        find_attribute(base.tree_attributes.occupied_slots(), name)
    }
}

// Native walks cached rows in order, comparing all five packed name words.
fn find_attribute<'a>(
    rows: impl IntoIterator<Item = &'a Option<AndaleRuntimeAttribute>>,
    name: FastString30,
) -> Result<Option<AndaleRuntimeAttribute>, MotionUpdateError> {
    use crate::actor_animation_construction::frame_phase::{
        AnimationPhaseError, find_cached_tree_attribute,
    };
    find_cached_tree_attribute(rows, name).map_err(|error| match error {
        AnimationPhaseError::Missing("tree attribute name encoding") =>
            MotionUpdateError::PendingAnimationOperation("tree attribute name encoding"),
        AnimationPhaseError::Missing(owner) => MotionUpdateError::MissingOwner(owner),
        _ => unreachable!("cached lookup only reports missing row/name representation"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::andale::AndaleAttributeValue;
    #[test]
    fn cached_lookup_is_packed_ordered_and_does_not_touch_later_unknown_rows() {
        let row = AndaleRuntimeAttribute {
            sequence_id: -1,
            begin_seconds: 0.0,
            end_seconds: 1.0,
            name: "SequenceToAir".into(),
            status: 5,
            value: AndaleAttributeValue::Float(7.0),
        };
        let rows = [Some(row.clone()), None];
        assert_eq!(
            find_attribute(&rows, FastString30::encode("sequencetoair").unwrap()).unwrap(),
            Some(row)
        );
        assert!(find_attribute(&rows, FastString30::encode("other").unwrap()).is_err());
        assert!(
            find_attribute(&[], FastString30::encode("sequencetoair").unwrap())
                .unwrap()
                .is_none()
        );
    }
}
