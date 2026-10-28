//! Constructor history through real admitted source owners. Controlled input
//! samples are not a live ProcessedPhysIn/global-cvar implementation.
use super::*;
use crate::skeleton_physics_history::lane_knowledge::RecordVectorField as Field;
use crate::{
    attribute_runtime::tests::graph,
    dynamic_assembly_session::{
        hook_pool::SimulationHookAssemblyPool, skater_pool::SimulationSkaterAssemblyHandle,
        skeleton_collision_publication::tests::fixture,
    },
    simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession,
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_hook::SkateboardHookDefinition,
    skateboard_wheels::HostSkateboardWheelMassNumeric,
    skeleton_drive_attributes::{ANIMATION_DRIVES_CLASS, SkeletonDriveAttributes},
    skeleton_drives::{HostSkeletonDriveNumeric, SkeletonDriveCvars},
    skeleton_physics_history::{HostSkeletonPhysicsHistoryNumeric, SkeletonPhysicsHistory},
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{convert::Infallible, sync::Arc};

struct Cvars;
impl SkeletonDriveCvars for Cvars {
    type Error = Infallible;
    fn hard_drive_displacement(&mut self) -> Result<f32, Self::Error> {
        Ok(1.0)
    }
    fn hard_drive_strength(&mut self) -> Result<f32, Self::Error> {
        Ok(10.0)
    }
}

#[derive(Default)]
struct Input {
    events: Vec<&'static str>,
    up_reads: usize,
    fail_up: Option<usize>,
    fail_mode: bool,
    projected: bool,
    up: Option<[f32; 4]>,
}
impl SkeletonHistoryInput for Input {
    type Error = &'static str;
    type Up = [f32; 4];
    fn acquire_up_vector(&mut self) -> Result<Self::Up, Self::Error> {
        self.events.push("acquire");
        self.up_reads += 1;
        if self.fail_up == Some(self.up_reads) {
            return Err("controlled Up failure");
        }
        Ok(self.up.unwrap_or([0.0, 1.0, 0.0, -0.0]))
    }
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        self.events.push("xyz");
        if self.fail_mode {
            return Err("controlled COM mode failure");
        }
        Ok(u8::from(!self.projected))
    }
    fn read_up_vector(&mut self, up: &Self::Up) -> Result<[f32; 4], Self::Error> {
        self.events.push("read");
        Ok(*up)
    }
}

type Hooks = SimulationHookAssemblyPool<AssemblyPartMaterial, SkateboardHookDefinition>;

fn admitted(with_set: bool) -> (SkeletonNormalAssembly, SkateboardSimulationBackend, Hooks) {
    admitted_capacity(with_set, 27, 22)
}

fn admitted_capacity(
    with_set: bool,
    bodies: usize,
    joints: usize,
) -> (SkeletonNormalAssembly, SkateboardSimulationBackend, Hooks) {
    let mut backend = SkateboardSimulationBackend::try_new(bodies, joints, 46).unwrap();
    let mut manager = backend.try_manager_for_test(0).unwrap();
    let mut hooks = Hooks::try_new(&manager, &backend, 3).unwrap();
    let mut owner = fixture(None);
    owner.attach(&mut backend).unwrap();
    let mut attributes =
        SkeletonDriveAttributes::bind(&Arc::new(graph(ANIMATION_DRIVES_CLASS, vec![])));
    owner
        .try_initialize_drives(
            &mut backend,
            &mut manager,
            &mut hooks,
            &mut attributes,
            &mut HostSkeletonDriveNumeric,
            &mut HostSkateboardWheelMassNumeric,
            &mut Cvars,
        )
        .unwrap();
    if with_set {
        owner
            .try_set_retained_pose_with_hooks(&mut backend, &mut hooks)
            .unwrap();
    }
    (owner, backend, hooks)
}

fn live_transforms(
    owner: &SkeletonNormalAssembly,
    backend: &SkateboardSimulationBackend,
) -> [[f32; 16]; 24] {
    std::array::from_fn(|part| {
        owner
            .session()
            .try_live_part_transform_host(backend, part)
            .unwrap()
    })
}

pub(crate) fn history_words(history: &SkeletonPhysicsHistory) -> Vec<u32> {
    let record = history.record();
    history
        .part_masses()
        .iter()
        .chain(history.fractional_part_masses())
        .chain([history.total_mass(), history.time_delta()].iter())
        .chain(record.transforms().iter().flatten())
        .chain(record.positions().iter().flatten())
        .chain(record.velocities().iter().flatten())
        .chain(record.velocity_deltas().iter().flatten())
        .chain(record.centre_of_mass().iter())
        .chain(record.centre_of_mass_velocity().iter())
        .chain(record.proxy_volume_position().iter())
        .map(|value| value.to_bits())
        .collect()
}

fn check_history(owner: &SkeletonNormalAssembly, live: &[[f32; 16]; 24], updates: usize) {
    let history = owner.physics_history();
    let masses = std::array::from_fn::<_, 24, _>(|part| {
        owner.session().template().collision().body().parts[part].unscaled_part_mass
    });
    assert_eq!(
        history.part_masses().map(f32::to_bits),
        masses.map(f32::to_bits)
    );
    let total = masses.into_iter().fold(0.0_f32, |sum, mass| mass + sum);
    assert_eq!(history.total_mass().to_bits(), total.to_bits());
    let reciprocal = 1.0 / total;
    let fractions = masses.map(|mass| mass * reciprocal);
    assert_eq!(
        history.fractional_part_masses().map(f32::to_bits),
        fractions.map(f32::to_bits)
    );
    assert_eq!(history.time_delta().to_bits(), 0x3c88_8889);
    let record = history.record();
    let mut com = [0.0_f32; 4];
    let mut com_velocity = [0.0_f32; 4];
    for part in 0..24 {
        assert_eq!(
            record.transforms()[part].map(f32::to_bits),
            live[part].map(f32::to_bits)
        );
        for lane in 0..3 {
            let position = live[part][12 + lane];
            let first = (position - 0.0) * f32::from_bits(0x426f_ffff);
            // These controlled live matrices are finite and unchanged between
            // calls: the second subtraction is exactly positive zero.
            let velocity = if updates == 1 { first } else { 0.0 };
            let delta = if updates == 1 {
                first - 0.0
            } else {
                velocity - first
            };
            assert_eq!(record.positions()[part][lane].to_bits(), position.to_bits());
            assert_eq!(
                record.velocities()[part][lane].to_bits(),
                velocity.to_bits()
            );
            assert_eq!(
                record.velocity_deltas()[part][lane].to_bits(),
                delta.to_bits()
            );
            com[lane] = position.mul_add(fractions[part], com[lane]);
            com_velocity[lane] = velocity.mul_add(fractions[part], com_velocity[lane]);
        }
    }
    assert_eq!(
        record.centre_of_mass().map(f32::to_bits)[..3],
        com.map(f32::to_bits)[..3]
    );
    assert_eq!(
        record.centre_of_mass_velocity().map(f32::to_bits)[..3],
        com_velocity.map(f32::to_bits)[..3]
    );
    assert_eq!(
        record.proxy_volume_position().map(f32::to_bits)[..3],
        [
            live[0][12].to_bits(),
            live[0][13].to_bits(),
            live[0][14].to_bits()
        ]
    );

    for part in 0..24 {
        assert!(
            record
                .transform_typed(part)
                .unwrap()
                .columns()
                .iter()
                .all(|column| column.w.is_none())
        );
        for field in [
            Field::Position(part),
            Field::Velocity(part),
            Field::VelocityDelta(part),
        ] {
            assert!(record.vector_typed(field).w.is_none());
        }
    }
    for field in [
        Field::CentreOfMass,
        Field::CentreOfMassVelocity,
        Field::ProxyVolumePosition,
    ] {
        assert!(record.vector_typed(field).w.is_none());
    }
}

#[test]
fn actual_owner_initializes_two_distinct_samples_without_mutating_any_live_constraint_or_body() {
    let (mut owner, mut backend, mut hooks) = admitted(true);
    let live = live_transforms(&owner, &backend);
    assert!(
        live.iter()
            .any(|matrix| matrix[12..15].iter().any(|&value| value != 0.0))
    );
    let bodies = backend.rigid_body_snapshot();
    let joints = backend.joint_snapshot();
    let drives = backend.drive_snapshot();
    let mut input = Input::default();
    owner
        .try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric,
        )
        .unwrap();
    assert_eq!(
        owner.physics_history_initialization(),
        SkeletonHistoryInitialization::Complete
    );
    assert_eq!(input.events, ["acquire", "xyz", "acquire", "xyz"]);
    check_history(&owner, &live, 2);
    assert!(
        owner
            .physics_history()
            .record()
            .velocity_deltas()
            .iter()
            .flatten()
            .any(|&v| v < 0.0)
    );
    assert_eq!(backend.rigid_body_snapshot(), bodies);
    assert_eq!(backend.joint_snapshot(), joints);
    assert_eq!(backend.drive_snapshot(), drives);
    let saved = history_words(owner.physics_history());
    input.events.clear();
    assert!(matches!(
        owner.try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonHistoryInitializationError::AlreadyAttempted)
    ));
    assert!(input.events.is_empty());
    assert_eq!(history_words(owner.physics_history()), saved);
    owner.try_release_drives(&mut backend, &mut hooks).unwrap();
    assert_eq!(history_words(owner.physics_history()), saved);
    owner.detach(&mut backend).unwrap();
}

#[test]
fn failed_second_acquisition_retains_the_entire_first_record_and_refuses_retry() {
    let (mut owner, mut backend, mut hooks) = admitted(true);
    let live = live_transforms(&owner, &backend);
    let mut input = Input {
        fail_up: Some(2),
        ..Input::default()
    };
    assert!(matches!(
        owner.try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonHistoryInitializationError::Input(
            "controlled Up failure"
        ))
    ));
    assert_eq!(input.events, ["acquire", "xyz", "acquire"]);
    assert_eq!(
        owner.physics_history_initialization(),
        SkeletonHistoryInitialization::Updating {
            completed_updates: 1
        }
    );
    check_history(&owner, &live, 1);
    let saved = history_words(owner.physics_history());
    input.events.clear();
    input.fail_up = None;
    assert!(matches!(
        owner.try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonHistoryInitializationError::AlreadyAttempted)
    ));
    assert!(input.events.is_empty());
    assert_eq!(history_words(owner.physics_history()), saved);
    owner.try_release_drives(&mut backend, &mut hooks).unwrap();
    owner.detach(&mut backend).unwrap();
}

#[test]
fn detached_foreign_and_only_part_placement_refuse_before_sampling_or_initialization() {
    let mut owner = fixture(None);
    let mut backend = SkateboardSimulationBackend::try_new(24, 22, 0).unwrap();
    let saved = history_words(owner.physics_history());
    let mut input = Input::default();
    assert!(matches!(
        owner.try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonHistoryInitializationError::Detached)
    ));
    owner.attach(&mut backend).unwrap();
    assert!(matches!(
        owner.try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonHistoryInitializationError::DrivesNotReady)
    ));
    assert_eq!(history_words(owner.physics_history()), saved);
    owner.detach(&mut backend).unwrap();
    let (mut owner, mut backend, mut hooks) = admitted(false);
    let foreign = SkateboardSimulationBackend::try_new(24, 22, 0).unwrap();
    assert!(matches!(
        owner.try_initialize_physics_history(
            &foreign,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonHistoryInitializationError::ForeignBackend)
    ));
    assert!(matches!(
        owner.try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonHistoryInitializationError::PoseNotSet)
    ));
    owner
        .try_place_parts_from_retained_physics_pose(&mut backend)
        .unwrap();
    assert!(matches!(
        owner.try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonHistoryInitializationError::PoseNotSet)
    ));
    assert_eq!(
        owner.physics_history_initialization(),
        SkeletonHistoryInitialization::NotStarted
    );
    assert!(input.events.is_empty());
    owner
        .try_set_retained_pose_with_hooks(&mut backend, &mut hooks)
        .unwrap();
    // A later successful Part-only prefix must invalidate the full-Set proof.
    owner
        .try_place_parts_from_retained_physics_pose(&mut backend)
        .unwrap();
    assert!(matches!(
        owner.try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonHistoryInitializationError::PoseNotSet)
    ));
    assert!(input.events.is_empty());
    owner
        .try_set_retained_pose_with_hooks(&mut backend, &mut hooks)
        .unwrap();
    owner
        .try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric,
        )
        .unwrap();
    owner.try_release_drives(&mut backend, &mut hooks).unwrap();
    owner.detach(&mut backend).unwrap();
}

#[test]
fn stale_actual_root_is_read_before_input_and_retains_mass_prefix_without_record_writes() {
    let (mut owner, mut backend, mut hooks) = admitted_capacity(true, 51, 44);
    let mut other = fixture(None);
    other.attach(&mut backend).unwrap();
    let stale = other.session.part_attachment.parts[0].body;
    other.detach(&mut backend).unwrap();
    // Private fixture corruption uses an actually issued, now removed body.
    // No production capability or mutable session escape is fabricated.
    let original = owner.session.part_attachment.parts[0].body;
    owner.session.part_attachment.parts[0].body = stale;
    let initial_record = *owner.physics_history().record().transforms();
    let mut input = Input::default();
    assert!(matches!(
        owner.try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonHistoryInitializationError::LiveRoot(_))
    ));
    owner.session.part_attachment.parts[0].body = original;
    assert!(input.events.is_empty());
    assert_eq!(
        owner.physics_history_initialization(),
        SkeletonHistoryInitialization::Updating {
            completed_updates: 0
        }
    );
    assert!(owner.physics_history().total_mass() > 0.0);
    assert_eq!(owner.physics_history().time_delta().to_bits(), 0);
    assert_eq!(
        owner
            .physics_history()
            .record()
            .transforms()
            .map(|matrix| matrix.map(f32::to_bits)),
        initial_record.map(|matrix| matrix.map(f32::to_bits))
    );
    assert_eq!(
        owner
            .physics_history()
            .record()
            .positions()
            .map(|position| position.map(f32::to_bits)),
        [[0; 4]; 24]
    );
    owner.try_release_drives(&mut backend, &mut hooks).unwrap();
    owner.detach(&mut backend).unwrap();
}

#[test]
fn projected_mode_loads_each_retained_up_only_after_its_cvar_and_does_not_normalize_it() {
    let (mut owner, mut backend, mut hooks) = admitted(true);
    let live = live_transforms(&owner, &backend);
    let up = [0.0_f32, 2.0, 0.0, 3.0];
    let mut input = Input {
        projected: true,
        up: Some(up),
        ..Input::default()
    };
    owner
        .try_initialize_physics_history(
            &backend,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric,
        )
        .unwrap();
    assert_eq!(
        input.events,
        ["acquire", "xyz", "read", "acquire", "xyz", "read"]
    );
    let history = owner.physics_history();
    let mut weighted = [0.0_f32; 4];
    for (matrix, fraction) in live.iter().zip(history.fractional_part_masses()) {
        for lane in 0..4 {
            weighted[lane] = matrix[12 + lane].mul_add(*fraction, weighted[lane]);
        }
    }
    let displacement: [f32; 4] = std::array::from_fn(|lane| weighted[lane] - live[0][12 + lane]);
    let projection = (displacement[0] * up[0] + displacement[1] * up[1]) + displacement[2] * up[2];
    let expected: [u32; 4] =
        std::array::from_fn(|lane| up[lane].mul_add(projection, live[0][12 + lane]).to_bits());
    assert_eq!(
        history.record().centre_of_mass().map(f32::to_bits)[..3],
        expected[..3]
    );
    assert_eq!(
        history.record().centre_of_mass_velocity().map(f32::to_bits),
        [0; 4]
    );
    owner.try_release_drives(&mut backend, &mut hooks).unwrap();
    owner.detach(&mut backend).unwrap();
}

type MountedWorld =
    SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition>;

pub(crate) fn initialize_mounted_history(
    world: &mut MountedWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Vec<u32> {
    let owner = world.skeleton(skeleton).unwrap();
    let live = live_transforms(owner, world.backend());
    let bodies = world.backend().rigid_body_snapshot();
    let joints = world.backend().joint_snapshot();
    let drives = world.backend().drive_snapshot();
    let frame = world
        .backend()
        .rigid_body_pool_for_test()
        .frame_identity()
        .unwrap();
    let fresh = world
        .skeleton_publication(skeleton)
        .unwrap()
        .has_bounds_for_frame(frame);
    let mut input = Input::default();
    world
        .try_initialize_physics_history(
            skeleton,
            &mut input,
            &mut HostSkeletonPhysicsHistoryNumeric,
        )
        .unwrap();
    assert_eq!(input.events, ["acquire", "xyz", "acquire", "xyz"]);
    let owner = world.skeleton(skeleton).unwrap();
    assert_eq!(
        owner.physics_history_initialization(),
        SkeletonHistoryInitialization::Complete
    );
    check_history(owner, &live, 2);
    assert!(
        owner
            .physics_history()
            .record()
            .velocity_deltas()
            .iter()
            .flatten()
            .any(|&v| v != 0.0)
    );
    assert_eq!(world.backend().rigid_body_snapshot(), bodies);
    assert_eq!(world.backend().joint_snapshot(), joints);
    assert_eq!(world.backend().drive_snapshot(), drives);
    assert_eq!(
        world
            .skeleton_publication(skeleton)
            .unwrap()
            .has_bounds_for_frame(frame),
        fresh
    );
    history_words(owner.physics_history())
}

#[test]
fn constructor_samples_do_not_claim_a_completed_post_solve_history_frame() {
    let (mut owner, mut backend, mut hooks) = admitted(true);
    assert_eq!(
        owner.physics_history_frame_state().progress(),
        SkeletonHistoryFrameProgress::NotStarted
    );
    assert!(owner.physics_history_frame_state().frame().is_none());
    owner
        .try_initialize_physics_history(
            &backend,
            &mut Input::default(),
            &mut HostSkeletonPhysicsHistoryNumeric,
        )
        .unwrap();
    assert_eq!(
        owner.physics_history_initialization(),
        SkeletonHistoryInitialization::Complete
    );
    assert_eq!(
        owner.physics_history_frame_state().progress(),
        SkeletonHistoryFrameProgress::NotStarted
    );
    assert!(owner.physics_history_frame_state().frame().is_none());
    owner.try_release_drives(&mut backend, &mut hooks).unwrap();
    owner.detach(&mut backend).unwrap();
}

/// Oracle assembled from actual retained bodies BEFORE calling the ongoing
/// writer. It uses prior positions/velocities, not solver velocity or a freshly
/// zeroed shadow history. All source four-lane arithmetic remains explicit.
struct OngoingExpected {
    matrices: [[f32; 16]; 24],
    positions: [[f32; 4]; 24],
    velocities: [[f32; 4]; 24],
    deltas: [[f32; 4]; 24],
    proxy: [f32; 4],
    com: [f32; 4],
    com_velocity: [f32; 4],
    previous_com: [f32; 4],
    previous_com_velocity: [f32; 4],
    mass_words: Vec<u32>,
}

fn ongoing_expected(
    world: &MountedWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    board: &SimulationSkaterAssemblyHandle,
) -> OngoingExpected {
    let owner = world.skeleton(skeleton).unwrap();
    let old = owner.physics_history();
    let mut matrices = live_transforms(owner, world.backend());
    let proxy: [f32; 4] = std::array::from_fn(|lane| matrices[0][12 + lane]);
    // cc4c: actual paired deck6, not constructor's actual skeleton0 root.
    matrices[0] = world
        .board(board)
        .unwrap()
        .try_live_part_transform_host(world.backend(), 6)
        .unwrap();
    let positions = matrices.map(|matrix| std::array::from_fn(|lane| matrix[12 + lane]));
    assert_ne!(
        positions[0][..3]
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        proxy[..3].iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        "actual deck6 and skeleton0 must discriminate the ongoing source from the constructor"
    );
    let velocities: [[f32; 4]; 24] = std::array::from_fn(|part| {
        std::array::from_fn(|lane| {
            (positions[part][lane] - old.record().positions()[part][lane])
                * f32::from_bits(0x426f_ffff)
        })
    });
    let deltas = std::array::from_fn(|part| {
        std::array::from_fn(|lane| velocities[part][lane] - old.record().velocities()[part][lane])
    });
    assert!(
        positions
            .iter()
            .zip(&velocities)
            .any(|(position, velocity)| {
                (0..3).any(|lane| {
                    (position[lane] * f32::from_bits(0x426f_ffff)).to_bits()
                        != velocity[lane].to_bits()
                })
            }),
        "zeroing prior history must change at least one expected XYZ velocity"
    );
    let mut com = [0.0_f32; 4];
    let mut com_velocity = [0.0_f32; 4];
    for part in 0..24 {
        for lane in 0..4 {
            com[lane] =
                positions[part][lane].mul_add(old.fractional_part_masses()[part], com[lane]);
            com_velocity[lane] = velocities[part][lane]
                .mul_add(old.fractional_part_masses()[part], com_velocity[lane]);
        }
    }
    OngoingExpected {
        matrices,
        positions,
        velocities,
        deltas,
        proxy,
        com,
        com_velocity,
        previous_com: old.record().centre_of_mass(),
        previous_com_velocity: old.record().centre_of_mass_velocity(),
        mass_words: mass_words(old),
    }
}

fn mass_words(history: &SkeletonPhysicsHistory) -> Vec<u32> {
    history
        .part_masses()
        .iter()
        .chain(history.fractional_part_masses())
        .chain([history.total_mass()].iter())
        .map(|value| value.to_bits())
        .collect()
}

fn check_ongoing_words(
    history: &SkeletonPhysicsHistory,
    expected: &OngoingExpected,
    completed_com: bool,
) {
    let record = history.record();
    assert_eq!(
        mass_words(history),
        expected.mass_words,
        "ongoing call must not reinitialize or replace original PartMass/fractions"
    );
    assert_eq!(
        history.time_delta().to_bits(),
        0x3c88_8889,
        "fixed native history time, not manager dt"
    );
    assert_eq!(
        record.transforms().map(|row| row.map(f32::to_bits)),
        expected.matrices.map(|row| row.map(f32::to_bits))
    );
    // Native XYZ arithmetic remains bit-exact; packed attached-body W values
    // are unavailable and must stay unknown through each derived vector.
    let check_vector = |field, expected: [f32; 4]| {
        let actual = record.vector_typed(field);
        assert_eq!(
            actual.xyz.map(f32::to_bits),
            std::array::from_fn(|lane| expected[lane].to_bits())
        );
        assert_eq!(actual.w, None);
    };
    for part in 0..24 {
        assert!(
            record
                .transform_typed(part)
                .unwrap()
                .columns()
                .iter()
                .all(|column| column.w.is_none())
        );
        check_vector(Field::Position(part), expected.positions[part]);
        check_vector(Field::Velocity(part), expected.velocities[part]);
        check_vector(Field::VelocityDelta(part), expected.deltas[part]);
    }
    check_vector(Field::ProxyVolumePosition, expected.proxy);
    let (com, velocity) = if completed_com {
        (expected.com, expected.com_velocity)
    } else {
        (expected.previous_com, expected.previous_com_velocity)
    };
    check_vector(Field::CentreOfMass, com);
    check_vector(Field::CentreOfMassVelocity, velocity);
}

/// Invoked by the existing actual paired original-data corpus AFTER a real
/// ground solve/export/list stage. It never fabricates a frame or board owner.
pub(crate) fn check_mounted_ongoing_history(
    world: &mut MountedWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    board: &SimulationSkaterAssemblyHandle,
) {
    let expected = ongoing_expected(world, skeleton, board);
    let bodies = world.backend().rigid_body_snapshot();
    let joints = world.backend().joint_snapshot();
    let drives = world.backend().drive_snapshot();
    let mut input = Input::default();
    world
        .try_update_skeleton_physics_history(skeleton, board, &mut input)
        .unwrap();
    assert_eq!(
        input.events,
        ["acquire", "xyz"],
        "one source update, not constructor's two samples"
    );
    let owner = world.skeleton(skeleton).unwrap();
    check_ongoing_words(owner.physics_history(), &expected, true);
    assert_eq!(
        owner.physics_history_frame_state().progress(),
        SkeletonHistoryFrameProgress::Complete
    );
    assert!(std::ptr::eq(
        Arc::as_ref(owner.physics_history_frame_state().frame().unwrap()),
        world.contact_spies().unwrap().unwrap()
    ));
    let saved = history_words(owner.physics_history());
    input.events.clear();
    assert!(
        world
            .try_update_skeleton_physics_history(skeleton, board, &mut input)
            .is_err()
    );
    assert!(
        input.events.is_empty(),
        "same actual frame cannot replay history or input sampling"
    );
    assert_eq!(
        history_words(world.skeleton(skeleton).unwrap().physics_history()),
        saved
    );
    assert_eq!(world.backend().rigid_body_snapshot(), bodies);
    assert_eq!(world.backend().joint_snapshot(), joints);
    assert_eq!(world.backend().drive_snapshot(), drives);
}

/// Invoke on a NEW actual solved frame. COM-mode failure occurs after all24
/// live matrix/history rows and proxy were written, but before either COM store.
/// The checked manager must retain that prefix and reject replay on this frame.
pub(crate) fn check_mounted_ongoing_history_late_failure(
    world: &mut MountedWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    board: &SimulationSkaterAssemblyHandle,
) {
    let expected = ongoing_expected(world, skeleton, board);
    let bodies = world.backend().rigid_body_snapshot();
    let mut input = Input {
        fail_mode: true,
        ..Input::default()
    };
    assert!(
        world
            .try_update_skeleton_physics_history(skeleton, board, &mut input)
            .is_err()
    );
    assert_eq!(input.events, ["acquire", "xyz"]);
    let owner = world.skeleton(skeleton).unwrap();
    check_ongoing_words(owner.physics_history(), &expected, false);
    assert_eq!(
        owner.physics_history_frame_state().progress(),
        SkeletonHistoryFrameProgress::Updating
    );
    assert!(std::ptr::eq(
        Arc::as_ref(owner.physics_history_frame_state().frame().unwrap()),
        world.contact_spies().unwrap().unwrap()
    ));
    let saved = history_words(owner.physics_history());
    input.events.clear();
    input.fail_mode = false;
    assert!(
        world
            .try_update_skeleton_physics_history(skeleton, board, &mut input)
            .is_err()
    );
    assert!(input.events.is_empty());
    assert_eq!(
        history_words(world.skeleton(skeleton).unwrap().physics_history()),
        saved
    );
    assert_eq!(world.backend().rigid_body_snapshot(), bodies);
}
