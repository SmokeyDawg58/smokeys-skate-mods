//! Persistent selected `GrindData` and `LivingWorld` query owners. Startup stages
//! remain explicit: this does not execute the complete Simulation initialization.
use crate::{
    grind_data_octree::OctreeBounds,
    grind_data_runtime::{
        DynamicGrindSplineProvider, GrindDataError, GrindDataQueryError, GrindDataQueryResult,
        GrindDataRuntime, HostGrindDataNumeric,
    },
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{
        LivingWorldGrindServices, LivingWorldInitError,
        provider::{LivingWorldGrindError, LivingWorldGrindProvider},
    },
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

pub(crate) struct PhysicalGrindServices {
    static_data: Arc<Mutex<GrindDataRuntime>>,
    living_world: Arc<Mutex<LivingWorldGrindServices>>,
    dynamic_installed: bool,
}

#[derive(Debug)]
pub(crate) enum PhysicalGrindError {
    Poisoned,
    Static(GrindDataError<Infallible>),
    Entity(LivingWorldInitError),
    Query(GrindDataQueryError<Infallible, DynamicProviderError>),
    AlreadyInstalled,
}

#[derive(Debug)]
pub(crate) enum DynamicProviderError {
    Poisoned,
    LivingWorld(LivingWorldGrindError),
}
impl std::fmt::Display for PhysicalGrindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("static grind owner lock poisoned"),
            Self::Static(error) | Self::Query(GrindDataQueryError::Static(error)) => error.fmt(f),
            Self::Entity(error) => error.fmt(f),
            Self::Query(GrindDataQueryError::Dynamic(error)) => error.fmt(f),
            Self::AlreadyInstalled => {
                f.write_str("selected dynamic grind provider already installed")
            }
        }
    }
}
impl std::fmt::Display for DynamicProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("dynamic grind owner lock poisoned"),
            Self::LivingWorld(error) => error.fmt(f),
        }
    }
}
struct RetainedDynamicProvider<'a> {
    owner: &'a Arc<Mutex<LivingWorldGrindServices>>,
    cvars: &'a InputCvarReferences,
}
impl DynamicGrindSplineProvider for RetainedDynamicProvider<'_> {
    type Error = DynamicProviderError;
    fn query(
        &mut self,
        bounds: OctreeBounds,
        result: &mut GrindDataQueryResult,
    ) -> Result<(), Self::Error> {
        // Acquire the dynamic owner only at native BoxTest99cc, after the
        // extent gate and all static result writes, including the 40-hit cap.
        let living_world = self
            .owner
            .lock()
            .map_err(|_| DynamicProviderError::Poisoned)?;
        LivingWorldGrindProvider {
            living_world: &living_world,
            cvars: self.cvars,
        }
        .query(bounds, result)
        .map_err(DynamicProviderError::LivingWorld)
    }
}

impl PhysicalGrindServices {
    pub(crate) fn new() -> Self {
        Self {
            static_data: Arc::new(Mutex::new(GrindDataRuntime::new())),
            living_world: Arc::new(Mutex::new(LivingWorldGrindServices::new())),
            dynamic_installed: false,
        }
    }
    /// Selected World.Initialize827b12a4 GrindData.Init call.
    pub(crate) fn try_initialize_static(&self) -> Result<(), PhysicalGrindError> {
        self.static_data
            .lock()
            .map_err(|_| PhysicalGrindError::Poisoned)?
            .try_initialize()
            .map_err(PhysicalGrindError::Static)
    }
    /// Selected Manager.Init8271f6f0 Vehicle/DMO entity-store subsequence.
    pub(crate) fn try_initialize_entities(
        &self,
        vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
    ) -> Result<(), PhysicalGrindError> {
        let mut living = self
            .living_world
            .lock()
            .map_err(|_| PhysicalGrindError::Poisoned)?;
        living
            .try_initialize_selected_entities_with_execution(vault, cvars)
            .map_err(PhysicalGrindError::Entity)
    }
    /// Simulation8289f0fc/828a6f3c installs its retained Manager+4 provider.
    /// The host replay guard does not model replacing a different native Manager.
    pub(crate) fn try_install_dynamic_provider(&mut self) -> Result<(), PhysicalGrindError> {
        if self.dynamic_installed {
            return Err(PhysicalGrindError::AlreadyInstalled);
        }
        self.dynamic_installed = true;
        Ok(())
    }
    /// Registration and queries share this exact static implementation.
    pub(crate) fn static_data(&self) -> &Arc<Mutex<GrindDataRuntime>> {
        &self.static_data
    }
    /// Future complete `LivingWorld` construction must retain this same owner.
    pub(crate) fn living_world(&self) -> &Arc<Mutex<LivingWorldGrindServices>> {
        &self.living_world
    }
    /// The world's grind splines in the box (`try_box_test_world`), then the PC park editor's
    /// placed pieces' (their splines at each placement, as a dropped prop's are queried) and
    /// dropped grind boards' (at their decks).
    pub(crate) fn try_box_test(
        &self,
        bounds: OctreeBounds,
        result: &mut GrindDataQueryResult,
        ignore_vertical: bool,
        cvars: &InputCvarReferences,
    ) -> Result<bool, PhysicalGrindError> {
        let found = self.try_box_test_world(bounds, result, ignore_vertical, cvars)?;
        let difference: [f32; 3] = std::array::from_fn(|lane| bounds.minimum[lane] - bounds.maximum[lane]);
        // The world query's own extent gate: a degenerate box finds nothing.
        if difference.iter().map(|d| d * d).sum::<f32>() <= f32::from_bits(0x3780_0000) {
            return Ok(found);
        }
        for placement in crate::ui::object_dropper::mission_placements().iter() {
            let Some((low, high)) = placement.shape.grind_bounds else {
                continue;
            };
            let (minimum, maximum) = crate::world::transform_collision_aabb(low, high, placement.transform);
            if (0..3).any(|axis| bounds.minimum[axis] > maximum[axis] || minimum[axis] > bounds.maximum[axis]) {
                continue;
            }
            // No reach gate: a piece's bounds already were (a mega ramp is far bigger than a
            // DMO's `kMaxDynamicObjectSize`).
            let room = crate::living_world_grind_services::provider::query_entity_grind_splines(
                crate::living_world_grind_services::provider::EntityKind::Dmo,
                u32::try_from(placement.id & 0xffff_ffff).unwrap_or(u32::MAX),
                &placement.transform,
                placement.shape.grind_splines.iter(),
                &bounds,
                [placement.transform[12], placement.transform[13], placement.transform[14], 1.0],
                f32::INFINITY,
                result,
            )
            .map_err(|e| PhysicalGrindError::Query(GrindDataQueryError::Dynamic(DynamicProviderError::LivingWorld(e))))?;
            if !room {
                return Ok(true);
            }
        }
        // Then the dropped grind boards' deck edges, at their decks.
        for &(id, deck) in crate::ui::object_dropper::grind_boards().iter() {
            let room = crate::living_world_grind_services::provider::query_entity_grind_splines(
                crate::living_world_grind_services::provider::EntityKind::Dmo,
                id,
                &deck,
                crate::ui::object_dropper::grind_board_splines().iter(),
                &bounds,
                [deck[12], deck[13], deck[14], 1.0],
                f32::INFINITY,
                result,
            )
            .map_err(|e| PhysicalGrindError::Query(GrindDataQueryError::Dynamic(DynamicProviderError::LivingWorld(e))))?;
            if !room {
                break;
            }
        }
        Ok(result.count() != 0)
    }

    fn try_box_test_world(
        &self,
        bounds: OctreeBounds,
        result: &mut GrindDataQueryResult,
        ignore_vertical: bool,
        cvars: &InputCvarReferences,
    ) -> Result<bool, PhysicalGrindError> {
        let data = self
            .static_data
            .lock()
            .map_err(|_| PhysicalGrindError::Poisoned)?;
        // Do not require the dynamic owner when the actual provider is null.
        if !self.dynamic_installed {
            return data
                .try_box_test::<_, RetainedDynamicProvider<'_>>(
                    bounds,
                    result,
                    ignore_vertical,
                    &mut HostGrindDataNumeric,
                    None,
                )
                .map_err(PhysicalGrindError::Query);
        }
        let mut provider = RetainedDynamicProvider {
            owner: &self.living_world,
            cvars,
        };
        data.try_box_test(
            bounds,
            result,
            ignore_vertical,
            &mut HostGrindDataNumeric,
            Some(&mut provider),
        )
        .map_err(PhysicalGrindError::Query)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicProviderError, GrindDataQueryError, GrindDataQueryResult, HostGrindDataNumeric,
        InputCvarReferences, OctreeBounds, PhysicalGrindError, PhysicalGrindServices,
    };
    use std::sync::Arc;

    fn bounds() -> OctreeBounds {
        OctreeBounds {
            minimum: [-8.0; 4],
            maximum: [8.0; 4],
        }
    }

    fn populated() -> PhysicalGrindServices {
        let services = PhysicalGrindServices::new();
        services.try_initialize_static().unwrap();
        services
            .static_data()
            .lock()
            .unwrap()
            .try_add(
                crate::grind_data_runtime::tests::source(0x1234, 1),
                &mut HostGrindDataNumeric,
            )
            .unwrap();
        services
    }

    fn poison_dynamic(services: &PhysicalGrindServices) {
        let owner = Arc::clone(services.living_world());
        assert!(
            std::thread::spawn(move || {
                let _guard = owner.lock().unwrap();
                panic!("intentional dynamic-owner poison");
            })
            .join()
            .is_err()
        );
    }

    #[test]
    fn installed_poisoned_dynamic_owner_is_only_read_after_static_query() {
        let mut services = populated();
        let refs = InputCvarReferences::new_static_initialized();
        let mut result = GrindDataQueryResult::new();
        assert!(
            services
                .try_box_test(bounds(), &mut result, false, &refs)
                .unwrap()
        );
        assert_eq!(result.count(), 1);
        services.try_install_dynamic_provider().unwrap();
        poison_dynamic(&services);
        let degenerate = OctreeBounds {
            minimum: [2.0; 4],
            maximum: [2.0; 4],
        };
        assert!(
            !services
                .try_box_test(degenerate, &mut result, false, &refs)
                .unwrap()
        );
        assert_eq!(result.count(), 0);
        assert!(matches!(
            services.try_box_test(bounds(), &mut result, false, &refs),
            Err(PhysicalGrindError::Query(GrindDataQueryError::Dynamic(
                DynamicProviderError::Poisoned
            )))
        ));
        assert_eq!(
            result.count(),
            1,
            "static result must precede dynamic failure"
        );
    }

    #[test]
    fn native_null_provider_skips_even_poisoned_dynamic_owner_and_unlinked_cvars() {
        let services = populated();
        poison_dynamic(&services);
        let refs = InputCvarReferences::new_static_initialized();
        let mut result = GrindDataQueryResult::new();
        assert!(
            services
                .try_box_test(bounds(), &mut result, false, &refs)
                .unwrap()
        );
        assert_eq!(result.count(), 1);
    }
}
