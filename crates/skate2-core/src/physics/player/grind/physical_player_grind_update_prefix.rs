//! Actual `GrindManager.Update`82de1798..1828; Materials1830 follows.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GRIND, PHYSICS_STATE_GRIND_5_O,
    PHYSICS_STATE_GRIND_BACKSLASH, PHYSICS_STATE_GRIND_BOARDSLIDE, PHYSICS_STATE_GRIND_TIPSLIDE,
    PHYSICS_STATE_NONSPECIFIC,
};
use super::{GrindManager, types::Investigation};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    grind_data_octree::OctreeBounds,
    grind_data_runtime::GrindDataQueryResult,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_state_changer::PhysicalPlayerStateChangerOwner,
    skeleton_processed_contact::SkeletonProcessedContact,
};
#[derive(Debug)]
pub(crate) enum GrindPrefixError<Q> {
    Missing(&'static str),
    Poisoned,
    Cvar(InputCvarError),
    Query(Q),
    Overlay(DebugOverlayError),
    UnknownFlags { offset: u32, mask: u32 },
}
impl<Q: std::fmt::Debug> std::fmt::Display for GrindPrefixError<Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "grind prefix missing {field}"),
            Self::Poisoned => f.write_str("grind prefix owner lock poisoned"),
            Self::Cvar(error) => write!(f, "grind prefix cvar: {error:?}"),
            Self::Query(error) => write!(f, "grind prefix query: {error:?}"),
            Self::Overlay(error) => error.fmt(f),
            Self::UnknownFlags { offset, mask } => {
                write!(f, "grind prefix unknown flags {offset:x}/{mask:x}")
            }
        }
    }
}
pub(super) fn need<T, Q>(value: Option<T>, field: &'static str) -> Result<T, GrindPrefixError<Q>> {
    value.ok_or(GrindPrefixError::Missing(field))
}
impl GrindManager {
    /// Executes the consecutive native prefix and retains its partial writes.
    pub(crate) fn try_update_prefix<Q>(
        &mut self,
        refs: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        query: &mut impl FnMut(OctreeBounds, &mut GrindDataQueryResult) -> Result<bool, Q>,
    ) -> Result<(), GrindPrefixError<Q>> {
        use GrindPrefixError as E;
        self.investigation = Investigation::reset(); //17b0
        self.try_monitor_flickering(refs)?; //17b8
        self.try_update_permission(refs)?; //17bc
        let processed = self.bindings.processed.lock().map_err(|_| E::Poisoned)?;
        let fields = processed.physics_state_fields();
        let category = need(fields.category, "category8ec")?;
        let active = category == PHYSICS_CATEGORY_GRIND || need(fields.current, "state8e4")? == PHYSICS_STATE_NONSPECIFIC;
        self.time_in_grind = if active {
            need(fields.time_delta, "dt93c")? + self.time_in_grind
        } else {
            0.0
        }; //180c
        drop(processed);
        if refs.try_enable_usual_grinds().map_err(E::Cvar)? {
            self.collision
                .as_ref()
                .ok_or(E::Missing("GrindCollision"))?
                .try_find_nearby(refs, overlay, query)?;
        }
        Ok(()) // next source call is Materials.Update1830
    }
    fn try_monitor_flickering<Q>(
        &mut self,
        refs: &InputCvarReferences,
    ) -> Result<(), GrindPrefixError<Q>> {
        use GrindPrefixError as E;
        let current = need(
            self.bindings
                .processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .physics_state_fields()
                .current,
            "state8e4",
        )?;
        if current != self.flicker_state {
            let increment = match current {
                PHYSICS_STATE_GRIND_BOARDSLIDE | PHYSICS_STATE_GRIND_TIPSLIDE | PHYSICS_STATE_GRIND_BACKSLASH => Some(refs.try_deck_flicker_score().map_err(E::Cvar)?),
                PHYSICS_STATE_GRIND_5_O => Some(refs.try_truck_flicker_score().map_err(E::Cvar)?),
                _ => None,
            };
            if let Some(increment) = increment {
                self.flicker_score = self.flicker_score.wrapping_add(increment);
            }
            self.flicker_state = current;
        }
        self.flicker_score = self.flicker_score.wrapping_sub(1).max(0);
        self.too_much_flickering =
            self.flicker_score > refs.try_too_much_flicker_threshold().map_err(E::Cvar)?;
        Ok(())
    }
    fn try_update_permission<Q>(
        &mut self,
        refs: &InputCvarReferences,
    ) -> Result<(), GrindPrefixError<Q>> {
        use GrindPrefixError as E;
        self.flicker_avoid_frames = if self.too_much_flickering {
            refs.try_state_flicker_avoid_frames().map_err(E::Cvar)?
        } else {
            self.flicker_avoid_frames.wrapping_sub(1).max(0)
        }; //75c4
        let processed = self.bindings.processed.lock().map_err(|_| E::Poisoned)?;
        let (word, known) = processed.known_flags_8c4();
        for mask in [0x1000_0000, 0x0800_0000] {
            if known & mask == 0 {
                return Err(E::UnknownFlags {
                    offset: 0x8c4,
                    mask,
                });
            }
            if word & mask != 0 {
                self.grind_forbidden = true;
                return Ok(());
            }
        }
        let word8c8 = need(processed.known_flags_8c8(), "flags8c8")?; //75e4 capture
        if word8c8 & 0x20 != 0
            || word8c8 & 0x1_0000 != 0
            || need(processed.physics_state_fields().flags_8cc, "flags8cc")? & 0x0100_0000 != 0
            || self.flicker_avoid_frames > 0
        {
            self.grind_forbidden = true;
            return Ok(());
        }
        drop(processed);
        let owner = self
            .bindings
            .owner
            .upgrade()
            .ok_or(E::Missing("physical owner"))?;
        let changer = owner.state_changer().lock().map_err(|_| E::Poisoned)?;
        let processed = self.bindings.processed.lock().map_err(|_| E::Poisoned)?;
        if changer.try_would_prevent_grind(&processed, refs)? {
            self.grind_forbidden = true;
            return Ok(());
        }
        // Native r5 remains the original captured8c8 across WouldPreventGrind.
        self.grind_forbidden = word8c8 & 0x10 != 0
            && need(processed.physics_state_fields().category, "category8ec")? != PHYSICS_CATEGORY_GRIND;
        Ok(())
    }
}
impl PhysicalPlayerStateChangerOwner {
    /// `WouldPreventGrind`82dd3a68: read SAME processed, then actual late counter.
    pub(crate) fn try_would_prevent_grind<Q>(
        &self,
        processed: &SkeletonProcessedContact,
        refs: &InputCvarReferences,
    ) -> Result<bool, GrindPrefixError<Q>> {
        use GrindPrefixError as E;
        if self.owner.is_none() {
            return Err(E::Missing("StateChanger constructor"));
        }
        let fields = processed.physics_state_fields();
        if need(fields.category, "category8ec")? != PHYSICS_CATEGORY_AIR {
            return Ok(false);
        }
        let time = need(fields.state_timer, "state timer974")?;
        let minimum = refs.try_state_changer_min_air_time().map_err(E::Cvar)?;
        // raw3a90=40990024: NOT_GT includes unordered.
        if time.partial_cmp(&minimum) != Some(std::cmp::Ordering::Greater) {
            return Ok(true);
        }
        let count = self
            .fields
            .as_ref()
            .ok_or(E::Missing("StateChanger fields"))?
            .post_grind_jump_counter;
        Ok(count <= refs.try_grind_jump_air_frames().map_err(E::Cvar)?)
    }
}

#[cfg(test)]
mod tests {
    use super::{GrindPrefixError, PhysicalPlayerStateChangerOwner, SkeletonProcessedContact};
    use crate::input_cvar_runtime::{InputCvarError, InputCvarManager, InputCvarReferences};
    use crate::physics_state_ids::{PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GROUND};
    #[test]
    fn actual_state_changer_non_air_skips_cvars_and_nan_air_prevents_grind() {
        let (_world, owner) =
            crate::physical_player_assembly::tests::issued_owner_for_grind_tests();
        let mut changer = PhysicalPlayerStateChangerOwner::new();
        changer.try_construct(&owner, true).unwrap();
        let mut processed = SkeletonProcessedContact::new();
        processed.physics_state_fields_mut().category = Some(PHYSICS_CATEGORY_GROUND);
        let refs = InputCvarReferences::new_static_initialized();
        assert!(
            !changer
                .try_would_prevent_grind::<()>(&processed, &refs)
                .unwrap()
        );
        processed.physics_state_fields_mut().category = Some(PHYSICS_CATEGORY_AIR);
        processed.physics_state_fields_mut().state_timer = Some(f32::NAN);
        assert!(matches!(
            changer.try_would_prevent_grind::<()>(&processed, &refs),
            Err(GrindPrefixError::Cvar(InputCvarError::Unlinked(
                "kStateChangerMinTimeInAir"
            )))
        ));
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        // Source NOT_GT exits before accessing the actual late counter.
        changer.fields = None;
        assert!(
            changer
                .try_would_prevent_grind::<()>(&processed, &refs)
                .unwrap()
        );
        processed.physics_state_fields_mut().state_timer = Some(1.0);
        assert!(matches!(
            changer.try_would_prevent_grind::<()>(&processed, &refs),
            Err(GrindPrefixError::Missing("StateChanger fields"))
        ));
    }
}
