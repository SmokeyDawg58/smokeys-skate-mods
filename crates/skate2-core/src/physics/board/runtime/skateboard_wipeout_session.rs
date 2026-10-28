//! Actual `Skateboard::SetFree`82b30790, `SetWipeout`82b30830,
//! `RestoreWheelMaterial`82b29ff8 and the grind material pair.
use super::{SkateboardBodyConstruction, SkateboardBodyConstructionPhase};
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyPartMaterialError, part_contact_spy::DynamicAssemblyPartContactSpyError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_assembly::{
        SkateboardAssemblySession, SkateboardSimulationBackend,
        volume_flags::SkateboardVolumeFlagError,
    },
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum WipeoutBoardError {
    ConstructionRequired,
    DefinitionMismatch,
    Part(DynamicAssemblyPartContactSpyError),
    Material(DynamicAssemblyPartMaterialError),
    Inertia(SimulationRigidBodyPoolError),
    Cvar(InputCvarError),
    Volume(SkateboardVolumeFlagError),
    DeckCount,
}
impl std::fmt::Display for WipeoutBoardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConstructionRequired => f.write_str("wipeout board constructor incomplete"),
            Self::DefinitionMismatch => f.write_str("wipeout board definition mismatch"),
            Self::Part(e) => write!(f, "wipeout board Part: {e}"),
            Self::Material(e) => write!(f, "wipeout board material: {e}"),
            Self::Inertia(e) => write!(f, "wipeout board inertia: {e}"),
            Self::Cvar(e) => write!(f, "free board damping cvar: {e:?}"),
            Self::Volume(e) => write!(f, "wipeout board volume: {e:?}"),
            Self::DeckCount => f.write_str("wipeout board deck count exceeds source signed count"),
        }
    }
}

impl SkateboardBodyConstruction {
    fn validate_mode_session(
        &self,
        session: &SkateboardAssemblySession,
    ) -> Result<(), WipeoutBoardError> {
        if !matches!(
            self.phase,
            SkateboardBodyConstructionPhase::Complete | SkateboardBodyConstructionPhase::HandedOff
        ) {
            return Err(WipeoutBoardError::ConstructionRequired);
        }
        if !self
            .template
            .as_ref()
            .is_some_and(|template| Arc::ptr_eq(template, session.template()))
        {
            return Err(WipeoutBoardError::DefinitionMismatch);
        }
        Ok(())
    }

    /// `SkateboardBody::EnableCollisionVolumes` 0x82b241a8, called again by
    /// HoldSkateboard after SetStandard's own call. Each flag write advances
    /// the same retained publication revision, even if bit one was already set.
    pub(crate) fn try_enable_collision_volumes(
        &self,
        session: &SkateboardAssemblySession,
    ) -> Result<(), WipeoutBoardError> {
        self.try_set_collision_volumes_enabled(session, true)
    }

    /// DisableCollisionVolumes82b24128 uses the same ordered slots and
    /// re-read deck count as EnableCollisionVolumes; only bit zero is cleared.
    pub(crate) fn try_disable_collision_volumes(
        &self,
        session: &SkateboardAssemblySession,
    ) -> Result<(), WipeoutBoardError> {
        self.try_set_collision_volumes_enabled(session, false)
    }

    fn try_set_collision_volumes_enabled(
        &self,
        session: &SkateboardAssemblySession,
        enabled: bool,
    ) -> Result<(), WipeoutBoardError> {
        use WipeoutBoardError as E;
        self.validate_mode_session(session)?;
        let collision = self.collision.as_ref().ok_or(E::ConstructionRequired)?;
        for top in 0..3 {
            if enabled {
                collision.volume_flags.enable_top(top)
            } else {
                collision.volume_flags.disable_top(top)
            }
            .map_err(E::Volume)?;
        }
        let mut count = collision
            .volume_flags
            .read()
            .map_err(E::Volume)?
            .deck_count();
        i32::try_from(count).map_err(|_| E::DeckCount)?;
        let mut index = 0;
        while index < count {
            if enabled {
                collision.volume_flags.enable_deck(index)
            } else {
                collision.volume_flags.disable_deck(index)
            }
            .map_err(E::Volume)?;
            index += 1;
            count = collision
                .volume_flags
                .read()
                .map_err(E::Volume)?
                .deck_count();
        }
        Ok(())
    }

    pub(crate) fn try_set_wipeout(
        &mut self,
        session: &mut SkateboardAssemblySession,
        backend: &mut SkateboardSimulationBackend,
    ) -> Result<(), WipeoutBoardError> {
        self.try_set_wipeout_material_mode(session, backend, None)
    }

    /// `Skateboard::SetFree` 0x82b30790. The linked damping value is read
    /// after the free-wheel bit store and multiplied by exact 0x426fffff.
    pub(crate) fn try_set_free(
        &mut self,
        session: &mut SkateboardAssemblySession,
        backend: &mut SkateboardSimulationBackend,
        refs: &InputCvarReferences,
    ) -> Result<(), WipeoutBoardError> {
        self.try_set_wipeout_material_mode(session, backend, Some(refs))
    }

    fn try_set_wipeout_material_mode(
        &mut self,
        session: &mut SkateboardAssemblySession,
        backend: &mut SkateboardSimulationBackend,
        free_cvars: Option<&InputCvarReferences>,
    ) -> Result<(), WipeoutBoardError> {
        use WipeoutBoardError as E;
        self.validate_mode_session(session)?;
        // Native captures this SAME embedded material before the first store.
        let material = Arc::clone(&self.materials.wipeout);
        self.post_fields
            .as_mut()
            .ok_or(E::ConstructionRequired)?
            .free_wheel_enabled = true;
        let angular_drag = if let Some(refs) = free_cvars {
            refs.try_free_skateboard_angular_damping()
                .map_err(E::Cvar)?
                * f32::from_bits(0x426f_ffff)
        } else {
            0.0
        };
        let deck = session
            .try_part_contact_spy_observation(backend, 6)
            .map_err(E::Part)?;
        let inertia = backend.try_capture_part_inertia(deck).map_err(E::Inertia)?;
        backend
            .try_set_captured_angular_drag(deck, &inertia, angular_drag)
            .map_err(E::Inertia)?;
        for part in [6, 0, 1, 2, 3, 4, 5] {
            // Exclusive same-session borrow preserves each native fresh Assembly
            // capture; the actual Part material is resolved/written at this stage.
            session
                .try_set_part_material(backend, part, Arc::clone(&material))
                .map_err(E::Material)?;
        }
        self.try_enable_collision_volumes(session)
    }

    /// Actual old-Ground Exit's selected wheel restoration: only Parts0..3.
    pub(crate) fn try_restore_wheel_material(
        &mut self,
        session: &mut SkateboardAssemblySession,
        backend: &mut SkateboardSimulationBackend,
    ) -> Result<(), WipeoutBoardError> {
        self.validate_mode_session(session)?;
        let material = Arc::clone(&self.materials.wheel);
        for part in 0..4 {
            session
                .try_set_part_material(backend, part, Arc::clone(&material))
                .map_err(WipeoutBoardError::Material)?;
        }
        Ok(())
    }

    /// `Skateboard::SetMaterialsForGrind` 0x82b29ee8 (deck, wheels, trucks)
    /// and `RestoreMaterialsFromGrind` 0x82b29f50 with the ordinary materials.
    pub(crate) fn try_set_grind_materials(
        &mut self,
        session: &mut SkateboardAssemblySession,
        backend: &mut SkateboardSimulationBackend,
        grind: bool,
    ) -> Result<(), WipeoutBoardError> {
        self.validate_mode_session(session)?;
        let (deck, wheel, truck) = if grind {
            (
                Arc::clone(&self.materials.deck_grind),
                Arc::clone(&self.materials.wheel_grind),
                Arc::clone(&self.materials.truck_grind),
            )
        } else {
            (
                Arc::clone(&self.materials.deck),
                Arc::clone(&self.materials.wheel),
                Arc::clone(&self.materials.truck),
            )
        };
        session
            .try_set_part_material(backend, 6, deck)
            .map_err(WipeoutBoardError::Material)?;
        for part in 0..4 {
            session
                .try_set_part_material(backend, part, Arc::clone(&wheel))
                .map_err(WipeoutBoardError::Material)?;
        }
        for part in 4..6 {
            session
                .try_set_part_material(backend, part, Arc::clone(&truck))
                .map_err(WipeoutBoardError::Material)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn wipeout_material_owner_for_test(
        &self,
    ) -> Arc<skate2_formats::collision::AssemblyPartMaterial> {
        Arc::clone(&self.materials.wipeout)
    }
    #[cfg(test)]
    pub(crate) fn wheel_material_owner_for_test(
        &self,
    ) -> Arc<skate2_formats::collision::AssemblyPartMaterial> {
        Arc::clone(&self.materials.wheel)
    }
}
