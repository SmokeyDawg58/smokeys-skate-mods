//! PC Object Dropper: a riderless real skateboard in the shared physics world.
//! A port addition (see `skater_pool::riderless`). Session code reaches it as
//! `frame.world.try_spawn_riderless_board(&vault, transform)` inside
//! `try_with_selected_queued_frame`, with the Vault from the local actor's
//! physical owner (`physical.owner().inputs().vault`). The pool steps it with
//! everything else; there is no per-frame call.

use super::SimulationAssemblyConstraintSession;
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyCandidate, SimulationSkaterAssemblyHandle,
        riderless::{RIDERLESS_BOARD_PARTS, RiderlessBoardError},
    },
    skateboard_body_construction::SkateboardBodyConstruction,
    skateboard_hook::SkateboardHookDefinition,
    skateboard_truck_drives::SkateboardTruckDrives,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::Arc;

/// How many times lighter a dropped board is than the player's board. The
/// player's board (~11.5 kg in the solver) is held by the hook, so an equal
/// board stopped a Skate 2-mode rider instead of being pushed; at 4x (~2.9 kg,
/// a real deck's weight) a roll-in carries it (kick 1.05 m -> ~2 m).
// Play-testing the player's own weight (1x) again now that the partly-frozen
// wake and held trucks are in; with them the Skate 2 roll-in test gives 0.32 m
// at 1x, 0.83 m at 2x, 4.9 m at 4x.
// Skate 2 physics keeps the player's own weight (1x); Skate 3 physics makes its board lighter
// on its own (skate3_world.rs `rigid_solid`, the park editor's Dropped board weight).
const RIDERLESS_LIGHTNESS: f32 = 1.0;

fn riderless_lightness() -> f32 {
    #[cfg(test)]
    if let Some(value) = std::env::var("RIDERLESS_LIGHTNESS").ok().and_then(|v| v.parse().ok()) {
        return value;
    }
    RIDERLESS_LIGHTNESS
}

/// A spawned riderless board. Clone shares the identity; it is not an owner.
#[derive(Clone, Debug)]
pub(crate) struct RiderlessBoardHandle(SimulationSkaterAssemblyHandle);

impl RiderlessBoardHandle {
    pub(crate) fn same_as(&self, other: &Self) -> bool {
        self.0.same_as(&other.0)
    }
}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Constructs a board from `vault` (the player's board definition), deck
    /// placed at `transform` (DMO-style row-major: X, Y, Z axis rows, then the
    /// translation row at `[12..15]`), at rest, and admits it to the pool.
    /// Fails with `Pool(Full)` when the 16-member pool (shared with AI and
    /// Party Play skaters, two members each) has no room.
    pub(crate) fn try_spawn_riderless_board(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        transform: [f32; 16],
    ) -> Result<RiderlessBoardHandle, RiderlessBoardError> {
        use RiderlessBoardError as E;
        let mut construction = Box::new(SkateboardBodyConstruction::new(vault));
        construction
            .try_construct()
            .map_err(|error| E::Construction(error.to_string()))?;
        let mut candidate = SimulationSkaterAssemblyCandidate::try_from_construction(construction)
            .map_err(|failure| E::Construction(failure.error.to_string()))?;
        candidate.try_prepare_riderless(transform)?;
        // The player's board's Insert arguments (PhysicalPlayerAssembly).
        let handle = self
            .try_insert_board(candidate, 4, f32::from_bits(0x3f7f_be77))
            .map_err(|failure| E::Pool(failure.error))?;
        let drives = (|| {
            let mut drives = SkateboardTruckDrives::new(
                self.try_calc_constructed_board_truck_transforms(&handle)?,
            );
            self.try_initialize_constructed_board_truck_drives(&handle, &mut drives)?;
            self.try_retain_constructed_board_truck_drives(&handle, &mut Some(drives))
        })();
        // A loose board is in its bail setup: `WipeoutGround.Enter`'s
        // `SetWipeout` (wipeout material on every Part, free wheels) and its
        // zero force-turn. The riding setup it is built in rolls heavy.
        let drives = drives.map_err(|error| format!("{error:?}")).and_then(|()| {
            use crate::dynamic_assembly_session::skater_pool::{
                SimulationSkaterPoolError, board_wipeout::BoardWipeoutMaterialOperation,
            };
            let pool = self
                .skaters
                .as_mut()
                .ok_or(SimulationSkaterPoolError::Uninitialized)
                .map_err(|error| error.to_string())?;
            pool.try_board_wipeout_material_operation(
                &mut self.backend,
                &handle,
                BoardWipeoutMaterialOperation::SetWipeout,
            )
            .map_err(|error| error.to_string())?;
            pool.try_wipeout_board_turn(&self.backend, &handle)
                .map_err(|error| error.to_string())?;
            let lightness = riderless_lightness();
            if lightness != 1.0 {
                pool.try_scale_riderless_lightness(&mut self.backend, &handle, lightness)
                    .map_err(|error| error.to_string())?;
                // Bodies cache their mass at a transform write.
                pool.try_place_riderless(&mut self.backend, &handle, transform)
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        });
        if let Err(error) = drives {
            // Undo the admission; the drive error is the one reported.
            let _ = self.try_remove_board_drives(&handle);
            let _ = self.try_remove_board(&handle);
            return Err(E::Construction(error));
        }
        Ok(RiderlessBoardHandle(handle))
    }

    /// Teleports the board (deck at `transform`) and zeroes every Part's motion.
    pub(crate) fn try_place_riderless_board(
        &mut self,
        board: &RiderlessBoardHandle,
        transform: [f32; 16],
    ) -> Result<(), RiderlessBoardError> {
        self.skaters
            .as_mut()
            .ok_or(RiderlessBoardError::Pool(
                crate::dynamic_assembly_session::skater_pool::SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_place_riderless(&mut self.backend, &board.0, transform)
    }

    /// Live world Part matrices in the same convention: `[0..=3]` wheels,
    /// `[4..=5]` trucks, `[6]` deck.
    pub(crate) fn riderless_board_part_transforms(
        &self,
        board: &RiderlessBoardHandle,
    ) -> Result<[[f32; 16]; RIDERLESS_BOARD_PARTS], RiderlessBoardError> {
        self.skater_pool()
            .map_err(RiderlessBoardError::Pool)?
            .try_riderless_part_transforms(&self.backend, &board.0)
    }

    /// Drives, then the pool member (order-preserving). The handle is dead after.
    pub(crate) fn try_remove_riderless_board(
        &mut self,
        board: &RiderlessBoardHandle,
    ) -> Result<(), RiderlessBoardError> {
        use RiderlessBoardError as E;
        if !self.skater_pool().map_err(E::Pool)?.is_riderless(&board.0).map_err(E::Pool)? {
            return Err(E::NotRiderless);
        }
        self.try_remove_board_drives(&board.0).map_err(E::Pool)?;
        self.try_remove_board(&board.0).map_err(E::Pool)?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn try_shove_riderless_board(
        &mut self,
        board: &RiderlessBoardHandle,
        velocity: [f32; 3],
    ) -> Result<(), RiderlessBoardError> {
        self.skaters
            .as_mut()
            .ok_or(RiderlessBoardError::Pool(
                crate::dynamic_assembly_session::skater_pool::SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_shove_riderless(&mut self.backend, &board.0, velocity)
    }

    #[cfg(test)]
    pub(crate) fn board_inverse_masses(
        &self,
        board: &crate::dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    ) -> Vec<f32> {
        self.skater_pool().map_or(Vec::new(), |pool| pool.board_inverse_masses(&self.backend, board))
    }

    #[cfg(test)]
    pub(crate) fn riderless_inverse_masses(&self, board: &RiderlessBoardHandle) -> Vec<f32> {
        self.board_inverse_masses(&board.0)
    }

    #[cfg(test)]
    pub(crate) fn probe_riderless_deck(&self, board: &RiderlessBoardHandle) -> String {
        self.skater_pool().map_or("no pool".into(), |pool| pool.probe_riderless_deck(&self.backend, &board.0))
    }

    #[cfg(test)]
    pub(crate) fn try_spin_riderless_deck(
        &mut self,
        board: &RiderlessBoardHandle,
        omega: [f32; 3],
    ) -> Result<(), RiderlessBoardError> {
        self.skaters
            .as_mut()
            .ok_or(RiderlessBoardError::Pool(
                crate::dynamic_assembly_session::skater_pool::SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_spin_riderless_deck(&mut self.backend, &board.0, omega)
    }

    #[cfg(test)]
    pub(crate) fn board_volume_enables(
        &self,
        board: &crate::dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    ) -> String {
        self.skater_pool().map_or("no pool".into(), |pool| pool.board_volume_enables(board))
    }

    #[cfg(test)]
    pub(crate) fn riderless_volume_enables(&self, board: &RiderlessBoardHandle) -> String {
        self.board_volume_enables(&board.0)
    }

    pub(crate) fn riderless_board_frozen(
        &self,
        board: &RiderlessBoardHandle,
    ) -> Result<bool, RiderlessBoardError> {
        self.skater_pool()
            .map_err(RiderlessBoardError::Pool)?
            .try_riderless_frozen(&self.backend, &board.0)
    }

    pub(crate) fn riderless_board_count(&self) -> usize {
        self.skaters.as_ref().map_or(0, |pool| pool.riderless_count())
    }
}
