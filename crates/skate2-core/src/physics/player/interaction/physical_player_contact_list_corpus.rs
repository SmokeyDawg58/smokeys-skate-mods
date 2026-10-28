//! Actual registered board constructor -> resident original Rob's Park ground
//! -> solved spies -> Island contact vectors. This is the board-ready subpath
//! of the real paired factory, not a full pair/Actor or flags3 Tick. Spawn,
//! serial scheduling, capacity and current cvar services are controlled here.
use super::*;
use crate::{
    simulation_assembly_backend::constraint_session::{
        contact_feedback::ControlledNoFreezing, skateboard_ground::SkateboardGroundFrameInput,
    },
    simulation_contact_jacobian::{
        contact_spy::{SimulationContactSpy, SimulationContactSpyFrame},
        ground_preparation::corpus::mounted_robs_park_ground_for_board_test,
    },
    skateboard_attributes::SkateboardAttributes,
    static_ground_pool::StaticGroundPool,
};

const TERRAIN_POINT: [f32; 3] = [588.8617, -54.431, 996.5048];
const TIMESTEP: f32 = f32::from_bits(0x3c88_8889);

struct Services;
impl SkateboardSpawnServices for Services {
    fn current_orthogonal_tolerance(&mut self) -> f32 {
        0.01
    }
    fn orthogonality_assertion_failed(&mut self) {
        panic!("controlled terrain placement must pass the source rotation guard");
    }
}
impl PhysicalPlayerAssemblyServices for Services {
    fn current_entity_flags(&mut self) -> u32 {
        1
    }
}

/// Reuses the already-verified original input owners. No `SkeletonData`/body is
/// made before the actual board constructor and no contact is fabricated.
pub(crate) fn check(original: &Arc<PhysicalPlayerAssemblyOwner>) {
    check_frame_lifetime(original, false);
    check_frame_lifetime(original, true);
}

#[expect(
    clippy::too_many_lines,
    reason = "one actual mounted construction, solve/list lifetime and explicit cleanup proof"
)]
fn check_frame_lifetime(original: &Arc<PhysicalPlayerAssemblyOwner>, empty_surface_only: bool) {
    let source = original.inputs();
    let layout = source
        .vault
        .physics(SkateboardAttributes::DEFAULT_COLLECTION)
        .try_physics_layout()
        .unwrap();
    let gravity: [f32; 3] = std::array::from_fn(|axis| {
        f32::from_be_bytes(layout.raw_bytes[axis * 4..axis * 4 + 4].try_into().unwrap())
    });
    assert_eq!(gravity.map(f32::to_bits), [0, 0xc11c_cccd, 0]);
    let mut inputs = PhysicalPlayerAssemblyInputs {
        cvars: source.cvars.clone(),
        vault: Arc::clone(&source.vault),
        hierarchy: Arc::clone(&source.hierarchy),
        parameters: Arc::clone(&source.parameters),
        pose_hierarchy: Arc::clone(&source.pose_hierarchy),
        pose_matrices: Arc::clone(&source.pose_matrices),
        spawn: FRESH_VOLUME_TRANSFORM,
        hat_type: source.hat_type,
        bouncy_ragdoll: source.bouncy_ragdoll,
        is_human: source.is_human,
    };
    inputs.spawn[12..15].copy_from_slice(&TERRAIN_POINT);
    let ground = mounted_robs_park_ground_for_board_test();
    let mut selected = PhysicalAssemblyWorld::try_new(9, 6, 3, 256).unwrap();
    selected.backend_mut().set_gravity(gravity);
    selected.backend_mut().set_cooldown(2);
    selected
        .backend_mut()
        .set_minimum_energy(f32::from_bits(0x3727_c5ac));
    selected.try_initialize_skater_pool(3, false).unwrap();
    let mut factory = PhysicalPlayerAssemblyConstruction::new(&mut selected, inputs);
    factory.try_construct_board(&mut Services).unwrap();
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::BoardReady);
    assert!(factory.state.skeleton_construction.is_none());
    let board = factory.board().unwrap().clone();
    let hook = factory.state.hook.as_ref().unwrap().clone();
    let owner = Arc::clone(factory.owner());
    // Test-only access to the SAME exclusively borrowed manager at BoardReady;
    // production's completed_world_mut gate remains unchanged.
    let world = &mut *factory.world;
    assert!(Arc::ptr_eq(
        world.physical_body_member(&board).unwrap().owner(),
        &owner
    ));
    assert_eq!(world.physical_body_registry().target_count(), 7);
    if empty_surface_only {
        // Independent real construction: this solve must not perturb the other
        // fixture's pristine wheel-placement premise.
        step(world, &ground, 0, false);
        super::board_surface_corpus::empty_completed(world, &board);
        factory.try_teardown().unwrap();
        assert_eq!(factory.world().physical_contact_list_entry_count(), 0);
        return;
    }
    assert!(
        world
            .try_get_physical_collisions(&board)
            .unwrap()
            .is_empty()
    );
    assert_eq!(world.physical_contact_list_entry_count(), 1);
    assert!(world.physical_contact_list_has_entry(&board));
    place_actual_wheel(world, &board, &hook);
    step(world, &ground, 4, true);
    let retained = check_current_list(world, &board, &owner);
    assert!(!retained.records().is_empty());
    let old_records = retained.records().to_vec();
    // A diagnostic epoch change cannot promote the known-positive old list to
    // current output. The next real frame performs its own StartFrame as usual.
    world.backend_mut().begin_dynamic_update_frame(TIMESTEP);
    assert!(world.try_get_physical_collisions(&board).is_err());
    let stale = world.physical_contact_list_prefix(&board).unwrap();
    assert!(!stale.is_empty());
    assert!(
        stale
            .iter()
            .all(|contact| Arc::ptr_eq(contact.spy_frame(), &retained))
    );
    // The disabled export immediately follows the GUARANTEED nonempty prefix.
    // No optional empty physics frame can make this clear check vacuous.
    step(world, &ground, 0, false);
    assert!(world.contact_spies().unwrap().unwrap().records().is_empty());
    assert!(
        world
            .try_get_physical_collisions(&board)
            .unwrap()
            .is_empty(),
        "empty export clears the prior frame rather than retaining/appending entries"
    );
    assert_eq!(
        world.physical_contact_list_entry_count(),
        1,
        "source Clear retains the Body vector entry"
    );
    assert_eq!(
        retained.records(),
        old_records,
        "retained immutable old data is not current-frame authority"
    );
    for _ in 0..2 {
        step(world, &ground, 4, false);
        // Contacts need not persist under every physical response. If selected,
        // validate new entries; an empty solved export must give an empty list.
        if world.contact_spies().unwrap().unwrap().records().is_empty() {
            assert!(
                world
                    .try_get_physical_collisions(&board)
                    .unwrap()
                    .is_empty()
            );
        } else {
            let current = check_current_list(world, &board, &owner);
            assert!(
                !Arc::ptr_eq(&retained, &current),
                "new actual export, not appended old contacts"
            );
        }
    }
    factory.try_teardown().unwrap();
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Removed);
    assert_eq!(factory.world().physical_contact_list_entry_count(), 0);
    assert!(!factory.world().physical_contact_list_has_entry(&board));
    assert!(factory.world.try_get_physical_collisions(&board).is_err());
    assert_eq!(factory.world().physical_body_registry().target_count(), 0);
    assert_eq!(
        factory
            .world()
            .backend()
            .rigid_body_snapshot()
            .counts
            .static_bodies,
        1
    );
    assert!(
        factory
            .world()
            .backend()
            .drive_snapshot()
            .active_order
            .is_empty()
    );
    println!(
        "mounted real registered board -> original RobPark terrain -> solver/export/RF reset/actual Island list; positive retained Spy references, side/tag/normal/position/live relative velocity, next-frame replacement/empty clear and explicit cleanup; controlled board-ready schedule, not full player feedback"
    );
}

/// Explicit component-fixture admission after other tests have deformed the
/// same board. Restore the native `SetTransform` base expected by the existing
/// proven wheel placement, retaining every effect of the existing native setter.
pub(super) fn readmit_actual_wheel(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    hook: &SimulationHookAssemblyHandle,
) {
    let mut base = FRESH_VOLUME_TRANSFORM;
    base[12..15].copy_from_slice(&TERRAIN_POINT);
    world
        .try_set_registered_board_transform(board, hook, base, &mut Services)
        .unwrap();
    place_actual_wheel(world, board, hook);
}

fn place_actual_wheel(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    hook: &SimulationHookAssemblyHandle,
) {
    let live = world
        .board(board)
        .unwrap()
        .try_live_part_transform_host(world.backend(), 0)
        .unwrap();
    let radius = world
        .board(board)
        .unwrap()
        .template()
        .collision()
        .wheels
        .parts()[0]
        .volume()
        .radius;
    let target = [
        TERRAIN_POINT[0],
        TERRAIN_POINT[1] + radius * 0.5,
        TERRAIN_POINT[2],
    ];
    let mut requested = FRESH_VOLUME_TRANSFORM;
    requested[12..15].copy_from_slice(&TERRAIN_POINT);
    for axis in 0..3 {
        requested[12 + axis] += target[axis] - live[12 + axis];
    }
    world
        .try_set_registered_board_transform(board, hook, requested, &mut Services)
        .unwrap();
    let placed = world
        .board(board)
        .unwrap()
        .try_live_part_transform_host(world.backend(), 0)
        .unwrap();
    for axis in 0..3 {
        assert!((placed[12 + axis] - target[axis]).abs() < 0.000_25);
    }
}

fn step(
    world: &mut PhysicalAssemblyWorld,
    ground: &StaticGroundPool,
    spy_flags: u32,
    require_positive: bool,
) {
    let (counts, report) = world
        .try_step_authored_ground(
            SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                ground_pool: ground,
                ground_group: 0,
                pipeline_culling: None,
                allow_multiple_writes: true,
                spy_flags,
            },
            TIMESTEP,
            50,
            &mut ControlledNoFreezing,
        )
        .unwrap();
    assert_eq!((counts.joints, counts.drives), (6, 3));
    assert_eq!(report.processed_assemblies, 1);
    if require_positive {
        assert!(
            counts.contacts > 0,
            "actual source wheel penetration must produce contacts: {report:?}"
        );
        assert!(
            !world.contact_spies().unwrap().unwrap().records().is_empty(),
            "actual registered board contacts must have positive solved impulses"
        );
    }
    assert_eq!(world.physical_body_registry().target_count(), 7);
    assert_eq!(world.backend().rigid_body_snapshot().counts.active, 7);
}

struct Expected {
    spy_index: usize,
    spy: SimulationContactSpy,
    part: u32,
    is_a: bool,
    normal: [f32; 3],
    velocity: [f32; 3],
    surface: u32,
}

fn expected_list(
    world: &PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
) -> Vec<Expected> {
    let bodies = world.backend().rigid_body_snapshot();
    let mut expected = Vec::new();
    for (spy_index, spy) in world
        .contact_spies()
        .unwrap()
        .unwrap()
        .records()
        .iter()
        .enumerate()
    {
        // Source cap applies before each side's velocity reads. Neither the
        // unregistered real StaticWorld endpoint nor hook gets a Body entry.
        for (is_a, this, other, sign, surface) in [
            (true, spy.body_a(), spy.body_b(), 1.0, spy.tag() & 0xffff),
            (false, spy.body_b(), spy.body_a(), -1.0, spy.tag() >> 16),
        ] {
            if expected.len() == 16 {
                continue;
            }
            let Some(target) = world.physical_body_target(this).unwrap() else {
                continue;
            };
            assert!(target.member().same_as(board));
            assert!(
                world.physical_body_target(other).unwrap().is_none(),
                "isolated actual terrain endpoint has no registered Body owner"
            );
            let this_velocity = bodies.slots[this.identity().id().index() as usize].velocity;
            let other_velocity = bodies.slots[other.identity().id().index() as usize].velocity;
            expected.push(Expected {
                spy_index,
                spy: spy.clone(),
                part: target.part(),
                is_a,
                normal: spy.frame()[0].map(|value| value * sign),
                velocity: std::array::from_fn(|lane| this_velocity[lane] - other_velocity[lane]),
                surface,
            });
        }
    }
    expected
}

fn check_current_list(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Arc<SimulationContactSpyFrame> {
    let expected = expected_list(world, board);
    assert!(
        !expected.is_empty(),
        "positive real registered board spies must reach its vector"
    );
    assert!(Arc::ptr_eq(
        world.physical_body_member(board).unwrap().owner(),
        owner
    ));
    let actual = world.try_get_physical_collisions(board).unwrap();
    assert_eq!(actual.len(), expected.len());
    let retained = Arc::clone(actual[0].spy_frame());
    for (contact, expected) in actual.iter().zip(&expected) {
        assert!(Arc::ptr_eq(contact.spy_frame(), &retained));
        assert_eq!(contact.spy_index(), expected.spy_index);
        assert!(std::ptr::eq(
            contact.spy(),
            &raw const retained.records()[expected.spy_index]
        ));
        assert_eq!(contact.spy(), &expected.spy);
        assert!(contact.this_body().same_as(board));
        assert!(contact.other_body().is_none());
        assert!(!contact.self_colliding());
        assert_eq!(contact.is_body_a(), expected.is_a);
        assert_eq!(
            (contact.part(), contact.surface()),
            (expected.part, expected.surface)
        );
        assert_eq!(
            contact.normal().map(f32::to_bits),
            expected.normal.map(f32::to_bits)
        );
        assert_eq!(
            contact.position().map(f32::to_bits),
            expected.spy.frame()[3].map(f32::to_bits)
        );
        assert_eq!(
            contact.relative_velocity().map(f32::to_bits),
            expected.velocity.map(f32::to_bits)
        );
    }
    assert!(
        std::ptr::eq(
            Arc::as_ref(&retained),
            world.contact_spies().unwrap().unwrap()
        ),
        "ContactInfo retains the exact real completed exporter frame, not copied fake spies"
    );
    retained
}
