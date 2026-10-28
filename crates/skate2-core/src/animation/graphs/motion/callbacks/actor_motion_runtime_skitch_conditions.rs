//! Child of `actor_motion_runtime_session`: Motion conditions289/293.
//! The shared runtime bundle owns the selected GlobalAttributeCollection+190.
use crate::physics_state_ids::PHYSICS_STATE_SKITCHING;
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError};
use crate::attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault};
use std::sync::Arc;

/// Retained generic Instance at GlobalAttributeCollection+190, not a cached
/// generated layout. Bind once alongside the existing shared Action globals.
pub(crate) struct SkitchMotionAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl SkitchMotionAttributes {
    /// Global ctor828b3470 allocates10;3498 resolves this class/default,
    /// 34a8 constructs generic Instance and34b4 stores the member.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            vault: Arc::clone(vault),
            instance: vault.instance(0x07d3_9b41_aace_e50f, 0xd7ed_bd36_2d7d_2152),
        }
    }

    fn entering_limit(
        &self,
        physical_vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<f32, MotionUpdateError> {
        if !Arc::ptr_eq(&self.vault, physical_vault) {
            return Err(MotionUpdateError::ForeignOwner(
                "global anim_skitching Vault",
            ));
        }
        //82ca71c4 fetches before GetPhysOut71ec, even for negative time.
        // A missing FIELD alone uses the actual DefaultDataArea4 zero.
        let value = self
            .instance
            .try_scalar_word(0xbdfd_21f2_a968_28fd, 0)
            .map_err(|error| {
                MotionUpdateError::Load(
                    crate::generated_motion_load::MotionLoadError::AttributeRuntime(error),
                )
            })?
            .unwrap_or(0);
        Ok(f32::from_bits(value))
    }
}

/// Raw7204 BO12/BI24 tests LT;7210 BO4/BI25 tests !GT, including unordered.
fn entering_interval(time: f32, limit: f32) -> bool {
    !matches!(time.partial_cmp(&0.0), Some(std::cmp::Ordering::Less))
        && !matches!(time.partial_cmp(&limit), Some(std::cmp::Ordering::Greater))
}

impl ActorConstruction {
    /// IsSkitchingWithAbsorb82ca6f10: state guard precedes the global graph read.
    pub(crate) fn motion_skitch_with_absorb(&self, context: &BoundMotionGraphContext<ActorIdentity>, attributes: Option<&SkitchMotionAttributes>) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = p.owner().phys_out().lock().map_err(|_| E::Lock)?;
        if output.try_state_output().map_err(E::PhysOut)?.physics_state != PHYSICS_STATE_SKITCHING { return Ok(false); }
        let speed = output.try_ground_output().map_err(E::PhysOut)?.skitch_contact_speed;
        let a = attributes.ok_or(E::MissingOwner("global anim_skitching"))?;
        if !Arc::ptr_eq(&a.vault, &p.owner().inputs().vault) { return Err(E::ForeignOwner("global anim_skitching Vault")); }
        Ok(super::skitching::absorb(&a.instance, speed)? > 0.0)
    }
    /// SkitchingPosition82ca7228: handedness depends on regular/switch parity.
    pub(crate) fn motion_skitch_position(&self, context: &BoundMotionGraphContext<ActorIdentity>, desired: crate::generated_motion_load::conditions::SkitchPosition) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = p.owner().phys_out().lock().map_err(|_| E::Lock)?;
        let a = output.try_animation_output().map_err(E::PhysOut)?;
        Ok(skitch_position(a.skitch_grip_hand, a.is_regular_stance, a.is_riding_switch) == desired as u32)
    }
    /// IsSkitchShimmying82ca73e0 uses ISkaterAnim::GetIsMirrored (vtable+1c),
    /// not the separate PhysOut mirror flag.
    pub(crate) fn motion_skitch_shimmying(&self, context: &BoundMotionGraphContext<ActorIdentity>, desired: Option<crate::generated_motion_load::construct::vault::ShimmyDirection>) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        let speed = p.owner().phys_out().lock().map_err(|_| E::Lock)?.try_animation_output().map_err(E::PhysOut)?.skitch_hand_speed;
        let mirrored = animation.lock().map_err(|_| E::Lock)?.try_camera_mirrored().map_err(E::AnimationPhysicsData)?;
        let speed = (if mirrored { -1.0 } else { 1.0 }) * speed;
        let direction = if speed > 0.25 { 1 } else if speed < -0.25 { 2 } else { 0 };
        Ok(desired.map_or(0, |d| d as u32) == direction)
    }
    /// IsEnteringSkitch.ActivationFunc82ca70b8. The caller borrows attributes
    /// from its installed shared runtime bundle; never bind inside this query.
    pub(crate) fn motion_runtime_is_entering_skitch(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        attributes: Option<&SkitchMotionAttributes>,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        // Physical cache/UCOM identity is established before the global read.
        let limit = attributes
            .ok_or(E::MissingOwner(
                "GlobalAttributeCollection anim_skitching+190",
            ))?
            .entering_limit(&physical.owner().inputs().vault)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        // Native has no write-latch read here. This accessor checks only that
        // the actual component is constructed, then borrows its retained value.
        let time = output
            .try_ground_output()
            .map_err(E::PhysOut)?
            .time_to_skitch;
        Ok(entering_interval(time, limit))
    }

    /// IsSkitching.ActivationFunc82ca6e08: PhysOut+1c State+10 equals104.
    pub(crate) fn motion_runtime_is_skitching(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(output.try_state_output().map_err(E::PhysOut)?.physics_state == PHYSICS_STATE_SKITCHING)
    }
}

fn skitch_position(grip: i32, regular: bool, switch: bool) -> u32 {
    let parity = regular == switch;
    match grip { 1 => u32::from(parity) + 1, 2 => u32::from(!parity) + 1, _ => 3 }
}

#[cfg(test)]
mod tests {
    use super::entering_interval;
    #[test]
    fn grip_side_flips_with_stance_parity() {
        for regular in [false, true] {
            for switch in [false, true] {
                let same = regular == switch;
                assert_eq!(super::skitch_position(1, regular, switch), if same { 2 } else { 1 });
                assert_eq!(super::skitch_position(2, regular, switch), if same { 1 } else { 2 });
                assert_eq!(super::skitch_position(0, regular, switch), 3);
            }
        }
    }

    #[test]
    fn native_condition_bits_preserve_unordered_success() {
        assert!(!entering_interval(-1.0, 2.0));
        assert!(entering_interval(-0.0, 0.0));
        assert!(entering_interval(2.0, 2.0));
        assert!(!entering_interval(2.1, 2.0));
        assert!(entering_interval(f32::NAN, 2.0));
        assert!(entering_interval(0.0, f32::NAN));
        assert!(!entering_interval(-1.0, f32::NAN));
        assert!(!entering_interval(0.0, -1.0));
        assert!(entering_interval(f32::INFINITY, f32::INFINITY));
    }
}
