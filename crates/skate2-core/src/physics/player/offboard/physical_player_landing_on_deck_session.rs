//! Actual selected-world LandingOnDeck manager binding for OffBoard Air/landing.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_remaining_constructor_owners::landing_runtime::{LandingError, LandingWorld},
    reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::{convert::Infallible, sync::Arc};

#[derive(Debug)]
pub(crate) enum LandingSessionError<N> {
    Runtime(LandingError<N>),
    Pool(SimulationSkaterPoolError),
    Poisoned,
    Missing(&'static str),
    ForeignOwner,
}
impl<N: std::fmt::Debug> std::fmt::Display for LandingSessionError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "landing manager session: {self:?}")
    }
}

impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_arguments,
        reason = "real owner and source StartLandOnBoardQuery inputs"
    )]
    pub(crate) fn try_start_pair_land_on_board_query<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
        position: V,
        velocity: V,
        maximum_error: f32,
        ground: &StaticGroundPool,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), LandingSessionError<N::Error>> {
        use LandingSessionError as E;
        self.skeleton(skeleton).map_err(E::Pool)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let landing = remaining
            .landing
            .as_mut()
            .ok_or(E::Missing("LandingOnDeck"))?;
        if !landing.skeleton.matches(owner, &processed, skeleton) || !landing.world.matches(owner) {
            return Err(E::ForeignOwner);
        }
        let queries = self
            .world_line_queries
            .as_mut()
            .ok_or(E::Missing("world trajectory queries"))?;
        if !queries.belongs_to(owner.manager()) {
            return Err(E::ForeignOwner);
        }
        let dmos = self
            .dmos
            .as_ref()
            .ok_or(E::Missing("world DMO collection"))?;
        landing
            .start_land_on_board_query(
                position,
                velocity,
                maximum_error,
                cvars,
                &mut LandingWorld {
                    queries,
                    ground,
                    dmos,
                },
                numeric,
                &self.debug_overlay,
            )
            .map_err(E::Runtime)
    }

    pub(crate) fn try_update_pair_land_on_board_postphysics(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        cvars: &InputCvarReferences,
    ) -> Result<(), LandingSessionError<Infallible>> {
        use LandingSessionError as E;
        let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let landing = remaining
            .landing
            .as_mut()
            .ok_or(E::Missing("LandingOnDeck"))?;
        if !landing.world.matches(owner) {
            return Err(E::ForeignOwner);
        }
        // Native no-batch branch neither touches the query owner nor cvars.
        if landing.trajectory_batch.is_none() {
            return Ok(());
        }
        let queries = self
            .world_line_queries
            .as_mut()
            .ok_or(E::Missing("world trajectory queries"))?;
        landing
            .update_post_physics(queries, cvars, &self.debug_overlay)
            .map_err(E::Runtime)
    }

    pub(crate) fn try_fill_pair_land_on_board_output(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        cvars: &InputCvarReferences,
    ) -> Result<(), LandingSessionError<Infallible>> {
        use LandingSessionError as E;
        let remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let landing = remaining
            .landing
            .as_ref()
            .ok_or(E::Missing("LandingOnDeck"))?;
        if !landing.world.matches(owner) || owner.manager() != self.physical_manager_authority() {
            return Err(E::ForeignOwner);
        }
        let mut output = owner.phys_out().lock().map_err(|_| E::Poisoned)?;
        landing
            .fill_phys_out(&mut output, cvars, &self.debug_overlay)
            .map_err(E::Runtime)
    }
}

impl PhysicalAssemblyWorld {
    /// AccurateIK82dceab0 through the retained physical/skeleton binding.
    pub(crate) fn try_calculate_pair_land_on_board_ik<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
        time_to_land: f32,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), LandingSessionError<N::Error>> {
        use LandingSessionError as E;
        let assembly = self.skeleton(skeleton).map_err(E::Pool)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let landing = remaining
            .landing
            .as_mut()
            .ok_or(E::Missing("LandingOnDeck"))?;
        if !landing.skeleton.matches(owner, &processed, skeleton) || !landing.world.matches(owner) {
            return Err(E::ForeignOwner);
        }
        landing
            .calculate_accurate_ik_offset(
                assembly,
                time_to_land,
                cvars,
                numeric,
                &self.debug_overlay,
            )
            .map_err(E::Runtime)
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_landing_manager<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<
        crate::physical_player_remaining_constructor_owners::landing_runtime::LandingResult,
        LandingSessionError<N::Error>,
    > {
        use LandingSessionError as E;
        self.skeleton(skeleton).map_err(E::Pool)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let landing = remaining
            .landing
            .as_mut()
            .ok_or(E::Missing("LandingOnDeck"))?;
        if !landing.skeleton.matches(owner, &processed, skeleton) || !landing.world.matches(owner) {
            return Err(E::ForeignOwner);
        }
        let queries = self
            .world_line_queries
            .as_mut()
            .ok_or(E::Missing("world trajectory queries"))?;
        if !queries.belongs_to(owner.manager()) {
            return Err(E::ForeignOwner);
        }
        let dmos = self
            .dmos
            .as_ref()
            .ok_or(E::Missing("world DMO collection"))?;
        // Scratch values are overwritten by Update before success publication.
        let mut result =
            crate::physical_player_remaining_constructor_owners::landing_runtime::LandingResult {
                position: V::known([0.; 4]),
                velocity: V::known([0.; 4]),
                can_land: false,
                time_to_land: 0.,
            };
        landing
            .update(
                &mut result,
                cvars,
                &mut LandingWorld {
                    queries,
                    ground,
                    dmos,
                },
                numeric,
                &self.debug_overlay,
            )
            .map_err(E::Runtime)?;
        Ok(result)
    }
    pub(crate) fn try_start_pair_landing_state_wobble(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
        landing: bool,
    ) -> Result<(), crate::dynamic_assembly_session::skater_pool::SkeletonLandingWobbleError> {
        use crate::dynamic_assembly_session::skater_pool::SkeletonLandingWobbleError as E;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::MissingConstruction)?
                .processed(),
        );
        let mirrored = {
            let p = processed.lock().map_err(|_| E::MissingConstruction)?;
            let (flags, known) = p.known_flags_8c4();
            if known & 0x100000 == 0 {
                return Err(E::MissingConstruction);
            }
            let other = p
                .physics_state_fields()
                .flags_8cc
                .ok_or(E::MissingConstruction)?;
            ((flags >> 20) ^ (other >> 3)) & 1 != 0
        };
        self.skaters
            .as_mut()
            .ok_or(E::MissingConstruction)?
            .try_start_deck_state_wobble(&self.backend, skeleton, owner, landing, mirrored)
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_enable_pair_landing_ik(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), crate::physical_player_assembly::landing_on_deck_state::LandingStateError> {
        use crate::physical_player_assembly::landing_on_deck_state::{
            LandingStateError as E, service,
        };
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, actual| {
                if !Arc::ptr_eq(owner, actual) {
                    return Err(E::ForeignOwner);
                }
                normal
                    .try_enable_air_entry_ik()
                    .map_err(|e| service("EnableIK", e))
            })
            .map_err(|e| service("pair", e))?
    }
}
