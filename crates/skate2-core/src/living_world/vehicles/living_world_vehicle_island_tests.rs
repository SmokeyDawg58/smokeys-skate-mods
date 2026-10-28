//! Structural admission tests, UNRUN; constructed unit box, not a live vehicle.
use super::*;
use crate::{
    collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchConfig},
    living_world_grind_services::character_models::collision_assembly::tests::{
        create, definition,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};

fn world() -> PhysicalAssemblyWorld {
    let mut world = PhysicalAssemblyWorld::try_new(6, 0, 0, 0).unwrap();
    let batches = || {
        let capacity = CollisionQueryBatchCapacity {
            batches: 1,
            tests: 1,
        };
        CollisionQueryBatchConfig {
            line: capacity,
            boxes: capacity,
            trajectory: capacity,
            step_selector: capacity,
        }
    };
    world
        .try_initialize_world_line_queries(
            SimulationWorldLineQueryConfig {
                vehicle_update_in_parallel: false,
                ground: batches(),
                dmo: batches(),
                vehicle: batches(),
            },
            false,
        )
        .unwrap();
    world
}

#[test]
fn spawn_and_readmission_after_velocity_writes_wait_for_real_collision_refresh() {
    use crate::serialised_assembly_bbox::HostSerialisedAssemblyBBoxNumeric;
    for angular in [false, true] {
        let mut world = world();
        let def = definition();
        let existing = create(&def, &mut world.backend, 2.).unwrap();
        let existing = world
            .try_admit_vehicle_collision(
                CharacterAssemblyCandidate::new(existing, 71),
                &[FRESH_VOLUME_TRANSFORM],
                4,
            )
            .unwrap();
        let body = world
            .world_line_queries
            .as_ref()
            .unwrap()
            .vehicle
            .character_publication(&existing)
            .unwrap()
            .rigid_bodies()[0];
        let previous_frame = world
            .backend
            .collision_body_pool()
            .frame_identity()
            .unwrap();
        // This is the same pool-wide invalidation produced by skater/vehicle
        // velocity writes before Census attempts another car spawn.
        if angular {
            world
                .backend
                .rigid_bodies
                .try_set_part_angular_velocity(body, [0., 1., 0.])
                .unwrap();
        } else {
            world
                .backend
                .rigid_bodies
                .try_set_part_linear_velocity(body, [1., 0., 0.])
                .unwrap();
        }
        assert!(
            world
                .backend
                .collision_body_pool()
                .frame_identity()
                .is_none()
        );
        let candidate = create(&def, &mut world.backend, 2.).unwrap();
        let mut position = FRESH_VOLUME_TRANSFORM;
        position[12] = 12.;
        let spawned = world
            .try_admit_vehicle_collision(
                CharacterAssemblyCandidate::new(candidate, 72),
                &[position],
                4,
            )
            .unwrap();
        assert!(
            world
                .backend
                .collision_body_pool()
                .frame_identity()
                .is_none()
        );
        let pool = &world.world_line_queries.as_ref().unwrap().vehicle;
        let publication = pool.character_publication(&spawned).unwrap();
        assert!(!publication.has_bounds_for_frame(previous_frame));
        assert_eq!(
            pool.world_part_transforms(&world.backend, &spawned)
                .unwrap()[0][12],
            12.
        );
        assert!(
            world
                .physical_bodies
                .dynamic_order()
                .iter()
                .any(|b| b.same_vehicle(&spawned))
        );
        let candidate = pool.character_candidate(&spawned).unwrap();
        assert!(matches!(publication.try_stage_bounds(&candidate.assembly, &world.backend),
            Err(crate::dynamic_assembly_session::character_publication::CharacterPublicationError::StaleVolumes)));

        world.backend.begin_dynamic_update_frame(1. / 60.);
        let frame = world
            .backend
            .collision_body_pool()
            .frame_identity()
            .unwrap();
        assert_ne!(frame, previous_frame);
        let pool = &mut world.world_line_queries.as_mut().unwrap().vehicle;
        assert!(
            !pool
                .character_publication(&spawned)
                .unwrap()
                .has_bounds_for_frame(frame)
        );
        pool.try_update_serialized(&world.backend, &mut HostSerialisedAssemblyBBoxNumeric)
            .unwrap();
        for handle in [&existing, &spawned] {
            assert!(
                pool.character_publication(handle)
                    .unwrap()
                    .has_bounds_for_frame(frame)
            );
        }

        // The same rule must hold when an existing entity removes/re-adds its
        // collision body; preserve Assembly identity and native registration.
        let detached = world.try_remove_vehicle_collision(&spawned).unwrap();
        world
            .backend
            .rigid_bodies
            .try_set_part_linear_velocity(body, [2., 0., 0.])
            .unwrap();
        let readmitted = world
            .try_admit_vehicle_collision(detached, &[position], 4)
            .unwrap();
        assert!(readmitted.same_as(&spawned));
        assert!(
            world
                .backend
                .collision_body_pool()
                .frame_identity()
                .is_none()
        );
        assert!(
            !world
                .world_line_queries
                .as_ref()
                .unwrap()
                .vehicle
                .character_publication(&readmitted)
                .unwrap()
                .has_bounds_for_frame(frame)
        );
        world.backend.begin_dynamic_update_frame(1. / 60.);
        let frame = world
            .backend
            .collision_body_pool()
            .frame_identity()
            .unwrap();
        let pool = &mut world.world_line_queries.as_mut().unwrap().vehicle;
        pool.try_update_serialized(&world.backend, &mut HostSerialisedAssemblyBBoxNumeric)
            .unwrap();
        assert!(
            pool.character_publication(&readmitted)
                .unwrap()
                .has_bounds_for_frame(frame)
        );
        world.try_remove_vehicle_collision(&readmitted).unwrap();
        world.try_remove_vehicle_collision(&existing).unwrap();
    }
}

#[test]
fn admission_places_then_registers_real_parts_in_shared_island_lists() {
    let mut world = world();
    let def = definition();
    let mut position = FRESH_VOLUME_TRANSFORM;
    position[12] = 12.;
    let active = create(&def, &mut world.backend, 2.).unwrap();
    let active = CharacterAssemblyCandidate::new(active, 71);
    let mut owners =
        crate::living_world_grind_services::collideable_objects::CollideableObjectMan::new();
    owners.register(active.handle(), 71).unwrap();
    assert!(matches!(owners.register(active.handle(), 99), Err(crate::living_world_grind_services::collideable_objects::AssemblyOwnerError::DuplicateAssembly)));
    let active = world
        .try_admit_vehicle_collision(active, &[position], 4)
        .unwrap();
    let fixed = create(&def, &mut world.backend, 2.).unwrap();
    let fixed = world
        .try_admit_vehicle_collision(
            CharacterAssemblyCandidate::new(fixed, 72),
            &[FRESH_VOLUME_TRANSFORM],
            1,
        )
        .unwrap();
    let pool = &world.world_line_queries.as_ref().unwrap().vehicle;
    assert_eq!(
        pool.character_candidate(&active).unwrap().body_id(),
        Some(1)
    );
    assert_eq!(pool.character_candidate(&fixed).unwrap().body_id(), Some(0));
    assert_eq!(world.physical_bodies.next_body_id(), 2);
    assert!(world.physical_bodies.dynamic_order()[0].same_vehicle(&active));
    assert!(world.physical_bodies.static_order()[0].same_vehicle(&fixed));
    assert_eq!(
        pool.world_part_transforms(&world.backend, &active).unwrap()[0][12],
        12.
    );
    for handle in [&active, &fixed] {
        let member = &world.physical_bodies.living_world_members[world
            .physical_bodies
            .living_world_index(LivingWorldBodyKind::Vehicle, handle)
            .unwrap()];
        assert!(member.registration_complete);
        assert_eq!(member.registered_parts, 1);
        let observation = pool
            .character_candidate(handle)
            .unwrap()
            .assembly
            .parts
            .try_part_contact_spy_observation(&world.backend, 0)
            .unwrap();
        assert_ne!(
            world
                .backend
                .collision_body_pool()
                .try_raw_state(observation)
                .unwrap()
                & 8,
            0
        );
        assert!(
            matches!(world.physical_bodies.targets.get(&observation), Some(IslandSpyTarget::LivingWorld { kind: LivingWorldBodyKind::Vehicle, member, part: 0 }) if member.same_as(handle))
        );
    }
    let island_handle = SimulationIslandBodyHandle::Vehicle(active.clone());
    assert_eq!(
        world
            .island_body_group_for_contacts(&island_handle)
            .unwrap(),
        10
    );
    assert!(matches!(
        world
            .island_body_owner_for_contacts(&island_handle)
            .unwrap(),
        super::super::contact_owners::SimulationContactEntityOwner::LivingWorld(71)
    ));
    assert!(
        world
            .try_get_island_collisions(&island_handle)
            .unwrap()
            .is_empty()
    );
    world.try_register_vehicle_body_targets(&active).unwrap();
    assert_eq!(world.physical_bodies.target_count(), 2);
    assert!(matches!(
        world.try_add_vehicle_body_membership(&active, 4),
        Err(VehicleIslandError::AlreadyMember)
    ));
    assert_eq!(world.physical_bodies.next_body_id(), 2);
    let detached = world.try_remove_vehicle_collision(&active).unwrap();
    assert!(!detached.assembly.parts.is_attached());
    assert!(detached.handle().same_as(&active));
    assert_eq!(owners.owner_of(detached.handle()), 71);
    let readmitted = world
        .try_admit_vehicle_collision(detached, &[position], 4)
        .unwrap();
    assert!(readmitted.same_as(&active));
    assert_eq!(owners.owner_of(&readmitted), 71);
    assert_eq!(
        world
            .world_line_queries
            .as_ref()
            .unwrap()
            .vehicle
            .island_body(&readmitted)
            .unwrap()
            .body_id,
        Some(2)
    );
    owners.remove(&readmitted); // DeInit removes this before Island cleanup.
    assert_eq!(owners.owner_of(&readmitted), 0);
    world.try_remove_vehicle_collision(&readmitted).unwrap();
    world.try_remove_vehicle_collision(&fixed).unwrap();
    assert_eq!(world.physical_bodies.target_count(), 0);
    assert!(world.physical_bodies.static_order().is_empty());
    assert!(world.physical_bodies.dynamic_order().is_empty());
    assert_eq!(world.physical_bodies.next_body_id(), 3);
    assert_eq!(world.world_line_queries.as_ref().unwrap().vehicle.len(), 0);
}

#[test]
fn too_few_target_transforms_leave_candidate_detached_and_ids_unissued() {
    let mut world = world();
    let candidate = create(&definition(), &mut world.backend, 2.).unwrap();
    let before = world.backend.rigid_body_snapshot().counts;
    let failure = world
        .try_admit_vehicle_collision(CharacterAssemblyCandidate::new(candidate, 71), &[], 4)
        .unwrap_err();
    let VehicleAdmissionFailure::Placement {
        candidate,
        error:
            VehiclePlacementError::PartTransforms {
                parts: 1,
                transforms: 0,
            },
    } = failure
    else {
        panic!("wrong refusal");
    };
    assert!(!candidate.assembly.parts.is_attached());
    assert_eq!(candidate.world_index(), 0xffff);
    assert_eq!(world.physical_bodies.next_body_id(), 1);
    assert_eq!(world.backend.rigid_body_snapshot().counts, before);
}
