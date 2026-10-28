//! PhysState_OffBoard::UpdateLocomotionViability82d86f48 with actual query owners.
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    input_cvar_runtime::InputCvarError,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{OwnerMember, StateKind, StatePayload},
    },
    physical_player_remaining_constructor_owners::location_runtime::{
        LocationError, LocationInput, LocationResult, LocationWorld,
    },
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum LocationSessionError {
    Runtime(LocationError<ReckoningHostNumericError, InputCvarError>),
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    UnknownFlags(u32),
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_offboard_location_viability(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<(), LocationSessionError> {
        use LocationSessionError as E;
        let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
        let Some(StatePayload::OffBoard(state)) = states
            .states
            .get_mut(&StateKind::OffBoard)
            .filter(|s| s.published)
            .and_then(|s| s.value.as_mut())
        else {
            return Err(E::Missing("OffBoard state"));
        };
        state.auto_turn = Some(0.0);
        let link = state
            .offboard_location_test
            .as_ref()
            .and_then(Option::as_ref)
            .ok_or(E::Missing("OffboardLocationTest link"))?;
        if link.member != OwnerMember::OffboardLocationTest
            || !link.owner.upgrade().is_some_and(|o| Arc::ptr_eq(&o, owner))
        {
            return Err(E::ForeignOwner);
        }
        drop(states);
        let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let location = remaining
            .location
            .as_mut()
            .ok_or(E::Missing("OffboardLocationTest"))?;
        if !location.world.matches(owner) {
            return Err(E::ForeignOwner);
        }
        let mut tuning = refs.clone();
        let result = if location.sync_allowed {
            let queries = self
                .world_line_queries
                .as_mut()
                .ok_or(E::Missing("world queries"))?;
            if !queries.belongs_to(owner.manager()) {
                return Err(E::ForeignOwner);
            }
            let mut output = LocationResult::constructed();
            location
                .get_results(&mut output, queries, &mut tuning, numeric)
                .map_err(E::Runtime)?;
            Some(output)
        } else {
            None
        };
        drop(remaining);
        if let Some(result) = result {
            let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::OffBoard(state)) = states
                .states
                .get_mut(&StateKind::OffBoard)
                .filter(|s| s.published)
                .and_then(|s| s.value.as_mut())
            else {
                return Err(E::Missing("OffBoard state"));
            };
            state.auto_turn = Some(result.turn);
            state.is_locomotion_viable = Some(!result.blocked);
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let processed = processed.lock().map_err(|_| E::Poisoned)?;
        let (flags, known) = processed.physics_state_fields().flags_8d4;
        if known & 0x80000000 == 0 {
            return Err(E::UnknownFlags(0x80000000));
        }
        if flags & 0x80000000 == 0 {
            drop(processed);
            let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::OffBoard(state)) = states
                .states
                .get_mut(&StateKind::OffBoard)
                .filter(|s| s.published)
                .and_then(|s| s.value.as_mut())
            else {
                return Err(E::Missing("OffBoard state"));
            };
            state.is_locomotion_viable = Some(false);
            return Ok(());
        }
        let effective = processed
            .ground_position_fields()
            .effective_animation
            .as_ref()
            .ok_or(E::Missing("Processed effective animation+c0"))?;
        let ground_position = effective.columns()[3]; // +f0 then +e0 source loads
        let look = effective.columns()[2];
        if known & 0x08000000 == 0 {
            return Err(E::UnknownFlags(0x08000000));
        }
        let input = LocationInput {
            ground: ground_position,
            look,
            jump_velocity: V::known([0.0; 4]),
            jumping: false,
            world_only: flags & 0x08000000 != 0,
        };
        drop(processed);
        let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
        let location = remaining
            .location
            .as_mut()
            .ok_or(E::Missing("OffboardLocationTest"))?;
        if !location.world.matches(owner) {
            return Err(E::ForeignOwner);
        }
        // World::Initialize827b0f3c captures GhostEnabled before choosing
        // its simulation island: flags1 when visual enabled, otherwise flags3.
        // Reuse that retained constructor choice; never resample live backend.
        let visual_island_enabled = self.island_flags() == 1;
        let queries = self
            .world_line_queries
            .as_mut()
            .ok_or(E::Missing("world queries"))?;
        if !queries.belongs_to(owner.manager()) {
            return Err(E::ForeignOwner);
        }
        let dmos = self.dmos.as_ref().ok_or(E::Missing("world DMO owner"))?;
        location
            .start_query(
                input,
                &mut LocationWorld {
                    visual_island_enabled,
                    queries,
                    ground,
                    dmos,
                },
                &mut tuning,
                &mut HostCollisionLineNumeric,
            )
            .map_err(E::Runtime)
    }
}
