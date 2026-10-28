//! Original arena callbacks feed retained runtime `GrindData` through an explicit
//! first-test registration profile. Parked exclusions are not native completion.
use crate::{
    grind_data_octree::OctreeBounds,
    grind_data_runtime::GrindDataQueryResult,
    input_cvar_runtime::InputCvarReferences,
    simulation_assembly_backend::constraint_session::grind_world::PhysicalGrindServices,
    simulation_environment_unlock_router::SimulationEnvironmentUnlockRouter,
    simulation_normal_arena_plan::{
        SimulationNormalArenaPlan, SimulationNormalOperation, SimulationRegistryAssetKind,
    },
};
use std::sync::{Arc, Mutex};

pub(super) struct RetainedGrindCorpus {
    terrain: crate::selected_terrain_startup::SelectedTerrainStartup,
    _stream: Option<super::camera_stream::RetainedCameraStream>,
}
fn all_terrain_bounds() -> OctreeBounds {
    OctreeBounds {
        minimum: [-100_000.0, -100_000.0, -100_000.0, 0.0],
        maximum: [100_000.0, 100_000.0, 100_000.0, 0.0],
    }
}
pub(super) fn mount(
    services: &PhysicalGrindServices,
    cvars: &InputCvarReferences,
) -> RetainedGrindCorpus {
    let mounts =
        Arc::new(crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus());
    let stream = super::camera_stream::load(Arc::clone(&mounts), cvars);
    let catalog = stream.catalog.clone();
    mount_inner(services, cvars, mounts, catalog, Some(stream))
}
pub(super) fn mount_from(
    services: &PhysicalGrindServices,
    cvars: &InputCvarReferences,
    mounts: &Arc<skate2_formats::mount::MountSet>,
    catalog: &crate::world_stream_catalog::WorldStreamCatalog,
) -> RetainedGrindCorpus {
    mount_inner(services, cvars, Arc::clone(mounts), catalog.clone(), None)
}
fn mount_inner(
    services: &PhysicalGrindServices,
    cvars: &InputCvarReferences,
    mounts: Arc<skate2_formats::mount::MountSet>,
    catalog: crate::world_stream_catalog::WorldStreamCatalog,
    stream: Option<super::camera_stream::RetainedCameraStream>,
) -> RetainedGrindCorpus {
    let mut resident =
        crate::selected_resident_startup::SelectedResidentStartup::new(mounts, catalog);
    resident
        .request(crate::collection_resident_streamer::CollectionId(
            0x823c_4449_fb31_63b4,
        ))
        .unwrap();
    // Deadline/yield are explicit corpus driver policy. The shared production
    // owner only exposes one poll of the existing asynchronous load machinery.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while !resident.poll().unwrap() {
        assert!(
            std::time::Instant::now() < deadline,
            "selected resident load timed out"
        );
        std::thread::yield_now();
    }
    let mesh = services
        .living_world()
        .lock()
        .unwrap()
        .try_mesh_store()
        .unwrap();
    let mut triggers = crate::trigger_camera_group::CameraTriggerGroup::construct();
    triggers
        .try_initialize(Arc::clone(resident.assets()), all_terrain_bounds())
        .unwrap();
    let mut terrain = crate::selected_terrain_startup::SelectedTerrainStartup::new(
        resident,
        SimulationEnvironmentUnlockRouter::try_new_empty(128).unwrap(),
        mesh,
        Arc::clone(services.static_data()),
        Arc::new(Mutex::new(triggers)),
        // These corpora mount terrain without LivingWorld Init: no DMO pass.
        None,
    );
    terrain.try_register(0).unwrap();
    let plans = &terrain.plans;
    let assets = &terrain.assets;
    let layers = &terrain.layers;
    for (plan, index) in terrain.adapter.selected_exclusions() {
        eprintln!(
            "selected arena exclusion: arena={:016x} operation={index} kind={}",
            plan.static_plan().asset().decoded.asset_id,
            plan.operations()[index].simulation_kind()
        );
    }
    // Check only AFTER actual normal dispatch: nonempty refusal stays ordered.
    assert_eq!(plans.len(), 21);
    assert_eq!(nav_mesh_count(plans), 0);
    assert_eq!(assets.retained_counts(), (21, 80));
    assert_eq!(
        plans
            .iter()
            .flat_map(|plan| plan.operations())
            .filter(|operation| matches!(
                operation,
                SimulationNormalOperation::RegistryAsset {
                    kind: SimulationRegistryAssetKind::WorldPainter,
                    ..
                }
            ))
            .count(),
        64
    );
    assert_eq!(layers.lock().unwrap().populated_layer_count(12), 0);
    assert!(layers.lock().unwrap().tile_count() > 0);
    eprintln!("camera mesh: same retained 21 cSim_500_900_high plans, zero kind-23 registrations");
    let mut result = GrindDataQueryResult::new();
    assert!(
        services
            .try_box_test(all_terrain_bounds(), &mut result, false, cvars)
            .unwrap()
    );
    assert_eq!(
        result.count(),
        40,
        "original terrain saturates native static cap"
    );
    assert_eq!(result.results().count(), 40);
    eprintln!(
        "original RoBsPark retained normal callbacks -> native GrindData octree -> 40-candidate cap and installed actual empty LivingWorld provider"
    );
    RetainedGrindCorpus {
        terrain,
        _stream: stream,
    }
}
fn nav_mesh_count(plans: &[Arc<SimulationNormalArenaPlan>]) -> usize {
    plans
        .iter()
        .flat_map(|plan| plan.operations())
        .filter(|operation| {
            matches!(
                operation,
                SimulationNormalOperation::RegistryAsset {
                    kind: SimulationRegistryAssetKind::NavMesh,
                    ..
                }
            )
        })
        .count()
}
impl RetainedGrindCorpus {
    pub(super) fn ground_pool(&self) -> &crate::static_ground_pool::StaticGroundPool {
        self.terrain.adapter.environment().ground_pool()
    }

    /// Selected retained capabilities, after actual full-plan admission.
    pub(super) fn camera_inputs(
        &self,
        world: &crate::physical_player_assembly::PhysicalAssemblyWorld,
        world_handle: &Arc<Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>,
        players: &crate::actor_construction::selected_startup::SelectedPlayerStartupOwners,
        bounds: &crate::world_stream_runtime::WorldBoundsPublication,
    ) -> crate::camera_session::selected_runtime::CameraSelectedInputs {
        self.terrain
            .try_camera_inputs(
                world,
                world_handle,
                players,
                bounds,
                crate::game::CommonBackEndSeed::debug_reference()
                    .camera_set
                    .to_owned(),
            )
            .unwrap()
    }

    pub(super) fn finish(mut self, services: &PhysicalGrindServices, cvars: &InputCvarReferences) {
        let retained_mesh = services
            .living_world()
            .lock()
            .unwrap()
            .try_mesh_store()
            .unwrap();
        assert!(Arc::ptr_eq(&self.terrain.mesh, &retained_mesh));
        assert_eq!(nav_mesh_count(&self.terrain.plans), 0);
        for registration in self.terrain.registrations {
            self.terrain.adapter.deregister(registration, 0).unwrap();
        }
        assert_eq!(self.terrain.layers.lock().unwrap().tile_count(), 0);
        let mut result = GrindDataQueryResult::new();
        assert!(
            !services
                .try_box_test(all_terrain_bounds(), &mut result, false, cvars)
                .unwrap()
        );
        assert_eq!(
            result.count(),
            0,
            "original DReg removes every retained static spline"
        );
    }
}
