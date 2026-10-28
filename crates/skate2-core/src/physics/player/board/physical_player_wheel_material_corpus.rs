//! Original pair -> retained `SurfacePhysics` token -> four live Part pointers ->
//! actual serialization and next ground solve. Explicit diagnostic scheduling;
//! this does not establish Actor or complete native `HiLOD` chronology.
use super::PhysicalAssemblyWorld;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    simulation_assembly_backend::constraint_session::{
        contact_feedback::ControlledNoFreezing,
        physical_wheel_material::{
            PhysicalPlayerSurfaceRuntimeError, PhysicalPlayerWheelMaterialError,
        },
        skateboard_ground::SkateboardGroundFrameInput,
    },
    skateboard_processed_surface::{
        SkateboardProcessedSurface, SkateboardSelectedMaterial, SkateboardSurfaceProjection,
    },
    skeleton_processed_contact::SkeletonProcessedContact,
    static_ground_pool::StaticGroundPool,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{convert::Infallible, sync::Arc};

fn bits(value: AssemblyPartMaterial) -> [u32; 3] {
    [
        value.static_friction,
        value.dynamic_friction,
        value.restitution,
    ]
    .map(f32::to_bits)
}
#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    current: [[u32; 3]; 7],
    authored: [[u32; 3]; 7],
    published: Vec<[u32; 3]>,
    revision: u64,
}
fn snapshot(world: &PhysicalAssemblyWorld, board: &SimulationSkaterAssemblyHandle) -> Snapshot {
    world.with_board_materials_for_test(board, |session, publication, revision| Snapshot {
        current: std::array::from_fn(|part| bits(*session.current_part_material(part).unwrap())),
        authored: std::array::from_fn(|part| {
            bits(session.template().part_payloads()[part].material)
        }),
        published: publication
            .records()
            .iter()
            .map(|record| bits(record.material))
            .collect(),
        revision,
    })
}
fn assert_selected_pointers(
    world: &PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    selected: &SkateboardSelectedMaterial,
) {
    world.with_board_materials_for_test(board, |session, _, _| {
        for part in 0..4 {
            assert!(std::ptr::eq(
                session.current_part_material(part).unwrap(),
                Arc::as_ref(selected.material_arc())
            ));
        }
    });
}

pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    hook: &crate::dynamic_assembly_session::hook_pool::SimulationHookAssemblyHandle,
    ground: &StaticGroundPool,
    processed: &mut SkeletonProcessedContact,
) {
    // SurfacePhysics was constructed by the same HiLOD owner before its board.
    world
        .try_dispatch_pair_surface(board, skeleton, processed)
        .unwrap();
    let projection = processed.selected_surface().unwrap();
    let selected = projection.material.as_ref().unwrap().clone();
    let selected_type = projection.physics_type.unwrap();
    let baseline = snapshot(world, board);
    world
        .try_set_board_wheel_material(board, || Ok::<_, Infallible>(selected.clone()))
        .unwrap();
    assert_selected_pointers(world, board, &selected);
    let installed = snapshot(world, board);
    assert_eq!(installed.revision, baseline.revision + 4);
    assert_eq!(installed.authored, baseline.authored);
    assert_eq!(installed.current[4..], baseline.current[4..]);
    assert_eq!(installed.published, baseline.published);

    // Same original Vault coefficients, distinct actual SurfacePhysics owner.
    let foreign = world
        .try_with_board_force_runtime(board, |runtime| {
            let mut owner = SkateboardProcessedSurface::bind(&runtime.slide_collision_attributes);
            owner
                .try_initialize_materials(&mut runtime.slide_collision_attributes)
                .unwrap();
            let mut sink = SkateboardSurfaceProjection::new();
            owner
                .dispatch_from_published_type(selected_type, &mut sink)
                .unwrap();
            sink.material.unwrap()
        })
        .unwrap();
    assert_eq!(
        bits(**foreign.material_arc()),
        bits(**selected.material_arc())
    );
    assert!(!foreign.same_as(&selected));
    assert!(matches!(
        world.try_set_board_wheel_material(board, || Ok::<_, Infallible>(foreign)),
        Err(PhysicalPlayerWheelMaterialError::Owner(
            PhysicalPlayerSurfaceRuntimeError::ForeignMaterial
        ))
    ));
    assert_eq!(snapshot(world, board), installed);
    assert_selected_pointers(world, board, &selected);

    // Existing publication helper installs an explicit diagnostic material and
    // verifies same-frame stale records then refresh. Restore the actual token
    // through the REAL four-store setter, never copied template coefficients.
    world.check_actual_board_material_publication(board);
    world
        .try_set_board_wheel_material(board, || Ok::<_, Infallible>(selected.clone()))
        .unwrap();
    assert_selected_pointers(world, board, &selected);
    let restored = snapshot(world, board);
    assert_eq!(restored.authored, baseline.authored);
    assert_eq!(restored.current[4..], baseline.current[4..]);

    // Earlier force/steering probes intentionally leave this pair deformed.
    // Earn wheel contact through the existing source SetTransform fixture;
    // keep the actual four material pointers and the consumer assertion.
    super::contact_list_corpus::readmit_actual_wheel(world, board, hook);
    assert_selected_pointers(world, board, &selected);
    check_next_solve(world, board, ground, &selected);
    let after = snapshot(world, board);
    assert_eq!(after.authored, baseline.authored);
    assert_eq!(after.current[4..], baseline.current[4..]);
    println!(
        "actual pair SurfacePhysics token -> same four Part material owners -> refreshed original-board coefficients -> actual wheel ContactJacobian friction in next positive RobPark solve; foreign equal-value owner refused; controlled schedule, no Actor claim"
    );
}

fn check_next_solve(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    ground: &StaticGroundPool,
    selected: &SkateboardSelectedMaterial,
) {
    // Read-only capture before the material consumer's isolated solve. Earlier
    // component diagnostics deliberately mutate this same pair's dynamics.
    let before_bodies = world.backend().rigid_body_snapshot();
    let (counts, report) = world
        .try_step_authored_ground(
            SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                ground_pool: ground,
                ground_group: 0,
                pipeline_culling: None,
                allow_multiple_writes: true,
                spy_flags: 4,
            },
            f32::from_bits(0x3d08_8889),
            50,
            &mut ControlledNoFreezing,
        )
        .unwrap();
    assert!(counts.contacts > 0);
    assert_eq!(report.processed_assemblies, 2);
    let wheel_bodies = world.with_board_materials_for_test(board, |_, publication, _| {
        publication.rigid_bodies()[..4].to_vec()
    });
    let actual = **selected.material_arc();
    let expected_friction = [actual.static_friction, actual.dynamic_friction]
        .map(|value| if value > 0.0 { value.to_bits() } else { 0 });
    let wheel_contacts: Vec<_> = world
        .contact_materials_for_test()
        .iter()
        .filter(|contact| {
            contact
                .bodies
                .iter()
                .any(|body| wheel_bodies.contains(body))
        })
        .collect();
    if wheel_contacts.is_empty() {
        eprintln!(
            "wheel material diagnostic: counts={counts:?} report={report:?} wheel_body_ids={wheel_bodies:?}"
        );
        let after_bodies = world.backend().rigid_body_snapshot();
        for (part, body) in wheel_bodies.iter().enumerate() {
            let id = body.identity().id();
            let before = before_bodies.slots.iter().find(|slot| slot.id == id);
            let after = after_bodies.slots.iter().find(|slot| slot.id == id);
            // Snapshot includes COM, basis/quaternion, velocity, force/torque,
            // live inertia/drag, pool identity and membership; no inferred pose.
            eprintln!("wheel material diagnostic: Part{part} before={before:?} after={after:?}");
        }
        for (index, contact) in world.contact_materials_for_test().iter().enumerate() {
            let endpoints = contact.bodies.map(|body| {
                world
                    .physical_body_target(body)
                    .unwrap()
                    .map(|target| (target.member().same_as(board), target.part()))
            });
            eprintln!(
                "wheel material diagnostic: contact[{index}] body_ids={:?} endpoints(board,part)={endpoints:?} friction={:08x?} spy={:08x} normal_impulse_bits={:08x}",
                contact.bodies,
                contact.friction_bits,
                contact.spy_word,
                contact.normal_impulse_bits
            );
        }
    }
    assert!(
        !wheel_contacts.is_empty(),
        "actual wheel contact required for material consumer proof"
    );
    for contact in wheel_contacts {
        assert_eq!(contact.friction_bits, expected_friction);
    }
    assert_selected_pointers(world, board, selected);
    world.with_board_materials_for_test(board, |session, publication, _| {
        assert!(
            publication.has_bounds_for_frame(
                world
                    .backend()
                    .collision_body_pool()
                    .frame_identity()
                    .unwrap()
            )
        );
        for record in publication.records() {
            let part = usize::from(record.rigid_body_index);
            assert_eq!(
                bits(record.material),
                bits(*session.current_part_material(part).unwrap())
            );
            if part < 4 {
                assert_eq!(bits(record.material), bits(**selected.material_arc()));
            }
        }
    });
}
