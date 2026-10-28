//! Actual `CheckAllowGrindLogic`82e00130..26c; independent of investigation.
use super::SkeletonProcessedContact;
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::hilod_prefix::PhysicalHiLodPrefix,
};
use std::fmt;
#[derive(Debug)]
pub(crate) enum PermissionLatchError {
    Missing(&'static str),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl fmt::Display for PermissionLatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "permission latch missing {v}"),
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
        }
    }
}
impl SkeletonProcessedContact {
    pub(crate) fn publish_world_only_collision(&mut self, world_only: bool) {
        let flags = &mut self.physics_state_fields_mut().flags_8d4;
        flags.0 = (flags.0 & !0x0800_0000) | (u32::from(world_only) << 27);
        flags.1 |= 0x0800_0000;
    }
    pub(crate) fn try_update_permission_latches(
        &mut self,
        prefix: &mut PhysicalHiLodPrefix,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), PermissionLatchError> {
        use PermissionLatchError as E;
        let (word, known) = self.known_flags_8c4();
        if known & 0x0020_0000 == 0 {
            return Err(E::Missing("prepare8c4"));
        }
        if word & 0x0020_0000 != 0 {
            prefix.grind_illegal_count = 0;
            prefix.non_collision_flags |= 0x0200_0000;
        }
        let flags = self.flags_8c8.ok_or(E::Missing("flags8c8"))?;
        let started = if flags & 0x80 != 0 {
            true
        } else {
            let (word, known) = self.known_flags_8c4();
            if known & 0x0040_0000 == 0 {
                return Err(E::Missing("jump8c4"));
            }
            word & 0x0040_0000 != 0
        };
        if started {
            prefix.grind_illegal_count = 0;
            prefix.non_collision_flags |= 0x0600_0000;
        }
        let captured = prefix.non_collision_flags;
        if captured & 0x0200_0000 != 0 {
            let flags = self.flags_8c8.ok_or(E::Missing("flags8c8"))?;
            if flags & 0x40 != 0 || (captured & 0x0800_0000 != 0 && flags & 0x10000 == 0) {
                prefix.non_collision_flags = captured & !0x0600_0000;
            }
            // Entered-branch decision is retained even if the preceding store cleared it.
            let count = prefix.grind_illegal_count;
            let maximum = cvars.try_max_grind_illegal_frames().map_err(E::Cvar)?;
            if count > maximum && prefix.non_collision_flags & 0x0800_0000 == 0 {
                overlay.require().map_err(E::Overlay)?;
                prefix.non_collision_flags &= !0x0600_0000;
            }
            prefix.grind_illegal_count = prefix.grind_illegal_count.wrapping_add(1);
        }
        let flags = self.flags_8c8.ok_or(E::Missing("flags8c8"))?;
        prefix.non_collision_flags =
            (prefix.non_collision_flags & !0x1000_0000) | ((flags << 22) & 0x1000_0000);
        let flags = self.flags_8c8.ok_or(E::Missing("flags8c8"))?;
        prefix.non_collision_flags =
            (prefix.non_collision_flags & !0x0800_0000) | ((flags << 11) & 0x0800_0000);
        let flags = self.flags_8c8.ok_or(E::Missing("flags8c8"))?;
        self.flags_8c8 = Some((flags & !0x20) | ((prefix.non_collision_flags >> 21) & 0x20));
        let flags = self.flags_8c8.ok_or(E::Missing("flags8c8"))?;
        self.flags_8c8 = Some((flags & !0x10) | ((prefix.non_collision_flags >> 21) & 0x10));
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::{
        DebugOverlaySlot, InputCvarReferences, PermissionLatchError, PhysicalHiLodPrefix,
        SkeletonProcessedContact,
    };
    use crate::input_cvar_runtime::InputCvarManager;
    #[test]
    fn clearing_latch_does_not_skip_late_cvar_and_failed_read_preserves_prefix() {
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        processed.flags_8c8 = Some(0x40);
        let mut prefix = PhysicalHiLodPrefix::construct_in_sim_allocation();
        prefix.non_collision_flags = 0x0600_0000;
        prefix.grind_illegal_count = 9;
        let refs = InputCvarReferences::new_static_initialized();
        assert!(matches!(
            processed.try_update_permission_latches(
                &mut prefix,
                &refs,
                &DebugOverlaySlot::default()
            ),
            Err(PermissionLatchError::Cvar(_))
        ));
        assert_eq!(prefix.non_collision_flags, 0);
        assert_eq!(prefix.grind_illegal_count, 9);
    }
    #[test]
    fn actual_latches_publish_with_no_investigation_or_overlay_for_idle() {
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        processed.flags_8c8 = Some(0x10040);
        let mut prefix = PhysicalHiLodPrefix::construct_in_sim_allocation();
        prefix.non_collision_flags = 0;
        processed
            .try_update_permission_latches(
                &mut prefix,
                &InputCvarReferences::new_static_initialized(),
                &DebugOverlaySlot::default(),
            )
            .unwrap();
        assert_eq!(prefix.non_collision_flags, 0x1800_0000);
        assert_eq!(processed.flags_8c8, Some(0x10040));
    }
    #[test]
    fn timeout_requires_actual_overlay_before_counter_increment() {
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        let mut prefix = PhysicalHiLodPrefix::construct_in_sim_allocation();
        prefix.non_collision_flags = 0x0600_0000;
        prefix.grind_illegal_count = 21;
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let overlay = DebugOverlaySlot::default();
        assert!(matches!(
            processed.try_update_permission_latches(&mut prefix, &refs, &overlay),
            Err(PermissionLatchError::Overlay(_))
        ));
        assert_eq!(prefix.grind_illegal_count, 21);
        assert_eq!(prefix.non_collision_flags, 0x0600_0000);
        overlay.try_construct().unwrap();
        processed
            .try_update_permission_latches(&mut prefix, &refs, &overlay)
            .unwrap();
        assert_eq!(prefix.grind_illegal_count, 22);
        assert_eq!(prefix.non_collision_flags, 0);
    }
}
