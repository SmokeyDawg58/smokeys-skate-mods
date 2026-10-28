//! Synthetic discriminators, not original corpus or native runtime traces.
//! Fixtures use the existing packed source-plan test builder and real checked
//! AddInertia/image/Create APIs. Explicit record corruption below is test-only.

use super::super::{
    SerialisedAssemblyParamsState, try_prepare_serialised_assembly_create_from_xenon,
};
use super::*;
use crate::{
    serialised_assembly_bbox::{
        HostSerialisedAssemblyBBoxNumeric, SerialisedAssemblyBBoxMinMaxNumeric,
    },
    serialised_assembly_template::tests::source_plan,
    simulation_rigid_body_pool::SimulationRigidBodyHandle,
    simulation_xenon_address_image::{
        SimulationArenaXenonAddressBinding, SimulationRigidBodyAddInertiaInput,
        SimulationXenonAddressImage,
    },
};
use skate2_formats::{collision::AssemblyPartMaterial, pegasus::ObjectId};
use std::sync::Arc;

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

#[derive(Default)]
struct CountingNumeric {
    calls: usize,
    fail_at: Option<usize>,
}

impl CountingNumeric {
    fn visit(&mut self) -> Result<(), usize> {
        let call = self.calls;
        self.calls += 1;
        if self.fail_at == Some(call) {
            Err(call)
        } else {
            Ok(())
        }
    }
}

impl SerialisedAssemblyBBoxMinMaxNumeric for CountingNumeric {
    type Error = usize;
    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        self.visit()?;
        Ok(HostSerialisedAssemblyBBoxNumeric
            .minimum(left, right)
            .unwrap())
    }
    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        self.visit()?;
        Ok(HostSerialisedAssemblyBBoxNumeric
            .maximum(left, right)
            .unwrap())
    }
}

impl SerialisedAssemblyPartBBoxNumeric for CountingNumeric {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        self.visit()?;
        Ok(HostSerialisedAssemblyBBoxNumeric
            .sum_three_products(left, right)
            .unwrap())
    }
    fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.visit()?;
        Ok(HostSerialisedAssemblyBBoxNumeric
            .reciprocal_square_root_estimate(value)
            .unwrap())
    }
}

struct Fixture {
    template: SerialisedAssemblyTemplate,
    pool: SimulationRigidBodyPool,
    inertia: SimulationRigidBodyAddInertiaOwner,
    image: SimulationPackedBodyWordImageDomain,
    handles: Vec<SimulationRigidBodyHandle>,
    parts: Vec<[SerialisedAssemblyCreatePartInput; 1]>,
    assemblies: Vec<PreparedSerialisedAssembly>,
}

impl Fixture {
    fn new(states: &[u32], start_frame: bool) -> Self {
        let owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let template =
            SerialisedAssemblyTemplate::try_new(Arc::clone(&owner), ObjectId(3)).unwrap();
        let binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(owner, [0x2000_0000, 0, 0, 0, 0]).unwrap(),
        );
        let count = u32::try_from(states.len()).unwrap();
        let mut pool = SimulationRigidBodyPool::try_new(states.len()).unwrap();
        if start_frame {
            pool.begin_dynamic_update_frame(1.0 / 60.0);
        }
        let mut inertia = SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding).unwrap();
        let runtime_inertia = template.runtime_inertia(0).unwrap();
        let handles = states
            .iter()
            .map(|state| {
                inertia
                    .try_add(
                        &mut pool,
                        IDENTITY,
                        SimulationRigidBodyAddInertiaInput::Definition(&runtime_inertia),
                        *state,
                    )
                    .unwrap()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let parts = handles
            .iter()
            .map(|handle| {
                [SerialisedAssemblyCreatePartInput {
                    body: handle.observation(),
                    part_transform: IDENTITY,
                    material: AssemblyPartMaterial {
                        static_friction: 0.8,
                        dynamic_friction: 0.7,
                        restitution: 0.0,
                    },
                    user_data: 0,
                }]
            })
            .collect::<Vec<_>>();
        let image = SimulationPackedBodyWordImageDomain::try_new(
            &pool,
            SimulationXenonAddressImage::try_new(0x1000_0080, count, 0, 0).unwrap(),
        )
        .unwrap();
        let assemblies = parts
            .iter()
            .map(|parts| {
                try_prepare_serialised_assembly_create_from_xenon(
                    &template,
                    &pool,
                    &inertia,
                    &image,
                    parts,
                    SerialisedAssemblyLiveAssemblyInput {
                        group: 7,
                        culling_table: template.culling_table(),
                    },
                    SerialisedAssemblyParamsState::native_default(),
                    &mut HostSerialisedAssemblyBBoxNumeric,
                )
                .unwrap()
            })
            .collect();
        Self {
            template,
            pool,
            inertia,
            image,
            handles,
            parts,
            assemblies,
        }
    }

    fn refresh(
        &mut self,
        flags: &[u32],
        numeric: &mut CountingNumeric,
    ) -> Result<Vec<usize>, SerialisedAssemblyPoolUpdateError<usize>> {
        let mut inputs = self
            .assemblies
            .iter_mut()
            .zip(&self.parts)
            .map(|(assembly, parts)| SerialisedAssemblyPoolUpdateInput {
                assembly,
                template: &self.template,
                inertia_owner: &self.inertia,
                parts,
                volume_flags: flags,
                live: SerialisedAssemblyLiveAssemblyInput {
                    group: 11,
                    culling_table: self.template.culling_table(),
                },
            })
            .collect::<Vec<_>>();
        try_update_serialised_assembly_pool(&mut inputs, &self.pool, &self.image, numeric)
    }

    fn stage(
        &self,
        index: usize,
        numeric: &mut CountingNumeric,
    ) -> Result<AabbWords, SerialisedAssemblyPoolUpdateError<usize>> {
        try_stage_assembly_bounds(
            &self.assemblies[index],
            index,
            &self.pool,
            &self.image,
            numeric,
        )
    }
}

#[test]
fn new_frame_requires_refresh_and_standalone_update_invalidates_bounds() {
    let mut fixture = Fixture::new(&[4], true);
    let old_frame = fixture.pool.frame_identity().unwrap();
    assert!(fixture.assemblies[0].has_bounds_for_frame(old_frame));
    fixture.pool.begin_dynamic_update_frame(1.0 / 60.0);
    let next_frame = fixture.pool.frame_identity().unwrap();
    assert_ne!(old_frame, next_frame);
    assert!(!fixture.assemblies[0].has_bounds_for_frame(next_frame));
    let mut numeric = CountingNumeric::default();
    assert_eq!(
        fixture.stage(0, &mut numeric),
        Err(SerialisedAssemblyPoolUpdateError::StaleVolumes { assembly: 0 })
    );
    assert_eq!(numeric.calls, 0);
    assert_eq!(fixture.refresh(&[1], &mut numeric), Ok(vec![0]));
    assert!(fixture.assemblies[0].has_bounds_for_frame(next_frame));
    fixture.assemblies[0]
        .try_update_from_xenon(
            &fixture.template,
            &fixture.pool,
            &fixture.inertia,
            &fixture.image,
            &fixture.parts[0],
            &[1],
            SerialisedAssemblyLiveAssemblyInput {
                group: 12,
                culling_table: fixture.template.culling_table(),
            },
        )
        .unwrap();
    assert_eq!(fixture.assemblies[0].volume_frame, Some(next_frame));
    assert!(!fixture.assemblies[0].has_bounds_for_frame(next_frame));
}

#[test]
fn initial_epoch_allows_refresh_and_later_update_refusal_invalidates_every_stamp() {
    let mut fixture = Fixture::new(&[4, 4], false);
    // Pool construction already establishes host epoch 1. A valid stamp is
    // deliberately not proof that native BeginFrame has executed.
    let initial_frame = fixture.pool.frame_identity().unwrap();
    let mut numeric = CountingNumeric::default();
    assert_eq!(fixture.refresh(&[1], &mut numeric), Ok(vec![0, 1]));
    assert!(numeric.calls > 0);
    assert!(
        fixture
            .assemblies
            .iter()
            .all(|assembly| assembly.has_bounds_for_frame(initial_frame))
    );
    fixture.pool.begin_dynamic_update_frame(1.0 / 60.0);
    let frame = fixture.pool.frame_identity().unwrap();
    assert_ne!(initial_frame, frame);
    assert!(
        fixture
            .assemblies
            .iter()
            .all(|assembly| !assembly.has_bounds_for_frame(frame))
    );
    fixture.refresh(&[1], &mut numeric).unwrap();
    let old = fixture
        .assemblies
        .iter()
        .map(|assembly| assembly.header.aabb)
        .collect::<Vec<_>>();
    assert!(
        fixture
            .assemblies
            .iter()
            .all(|assembly| assembly.has_bounds_for_frame(frame))
    );
    assert!(matches!(
        fixture.refresh(&[], &mut numeric),
        Err(SerialisedAssemblyPoolUpdateError::Update { assembly: 0, .. })
    ));
    assert!(
        fixture
            .assemblies
            .iter()
            .all(|assembly| !assembly.has_bounds_for_frame(frame))
    );
    assert_eq!(
        fixture
            .assemblies
            .iter()
            .map(|assembly| assembly.header.aabb)
            .collect::<Vec<_>>(),
        old
    );
}

#[test]
fn disabled_frozen_and_all_static_still_recompute_their_bounds() {
    for state in [4, 2, 1] {
        let mut fixture = Fixture::new(&[state], true);
        let sentinel = AabbWords::from_f32([-999.0; 4], [999.0; 4]);
        fixture.assemblies[0].header.aabb = sentinel;
        let mut numeric = CountingNumeric::default();
        let active = fixture.refresh(&[0], &mut numeric).unwrap();
        assert!(
            numeric.calls > 0,
            "bounds cannot filter disabled/frozen/static records"
        );
        assert_ne!(fixture.assemblies[0].header.aabb, sentinel);
        assert!(!fixture.assemblies[0].volume_data.records()[0].enabled);
        assert_eq!(
            fixture.assemblies[0].volume_data.records()[0].frozen,
            state & 2 != 0
        );
        assert_eq!(
            fixture.assemblies[0].header.num_static,
            u32::from(state & 1 != 0)
        );
        assert_eq!(active, if state & 2 == 0 { vec![0] } else { vec![] });
    }
}

#[test]
fn aggregate_checks_body_table_index_but_never_dereferences_selected_body() {
    let mut fixture = Fixture::new(&[4], true);
    // Deliberately corrupt only test records. Neither aggregate numeric fields
    // nor a removed body's observation may be read by this bounds-job branch.
    let record = &mut fixture.assemblies[0].volume_data.records_mut_for_test()[0];
    record.volume_type = VolumeType::Aggregate;
    record.rigid_body_index = 1;
    record.volume.radius = f32::NAN;
    record.transform = [f32::NAN; 16];
    fixture
        .pool
        .try_remove_rigid_body(fixture.handles[0])
        .unwrap();
    let mut numeric = CountingNumeric::default();
    assert_eq!(
        fixture.stage(0, &mut numeric),
        Err(SerialisedAssemblyPoolUpdateError::BodyIndex {
            assembly: 0,
            volume: 0,
            index: 1
        })
    );
    fixture.assemblies[0].volume_data.records_mut_for_test()[0].rigid_body_index = 0;
    assert_eq!(
        fixture.stage(0, &mut numeric),
        Ok(AabbWords::from_f32([0.0; 4], [0.0; 4]))
    );
    assert_eq!(numeric.calls, 0);
    fixture.assemblies[0].volume_data.records_mut_for_test()[0].volume_type = VolumeType::Sphere;
    assert!(matches!(
        fixture.stage(0, &mut numeric),
        Err(SerialisedAssemblyPoolUpdateError::Body { assembly: 0, .. })
    ));
    assert_eq!(numeric.calls, 0);
}

#[test]
fn empty_and_all_aggregate_union_return_positive_zero_without_numeric_calls() {
    let mut fixture = Fixture::new(&[4], true);
    let mut numeric = CountingNumeric {
        calls: 0,
        fail_at: Some(0),
    };
    let mut aggregate = fixture.assemblies[0].volume_data.records()[0].clone();
    aggregate.volume_type = VolumeType::Aggregate;
    aggregate.transform = [f32::NAN; 16];
    *fixture.assemblies[0].volume_data.records_mut_for_test() = vec![aggregate.clone(), aggregate];
    for empty in [false, true] {
        if empty {
            fixture.assemblies[0]
                .volume_data
                .records_mut_for_test()
                .clear();
        }
        let bounds = fixture.stage(0, &mut numeric).unwrap();
        assert_eq!(bounds.min_f32().map(f32::to_bits), [0; 4]);
        assert_eq!(bounds.max_f32().map(f32::to_bits), [0; 4]);
    }
    assert_eq!(numeric.calls, 0);
}

#[test]
fn later_assembly_numeric_failure_keeps_every_old_header_bbox_unpublished() {
    let mut fixture = Fixture::new(&[4, 4], true);
    let mut measurement = CountingNumeric::default();
    fixture.stage(0, &mut measurement).unwrap();
    assert!(measurement.calls > 0);
    let old = [
        AabbWords::from_f32([-13.0; 4], [17.0; 4]),
        AabbWords::from_f32([-23.0; 4], [29.0; 4]),
    ];
    for (assembly, sentinel) in fixture.assemblies.iter_mut().zip(old) {
        assembly.header.aabb = sentinel;
    }
    let frame = fixture.pool.frame_identity().unwrap();
    let mut numeric = CountingNumeric {
        calls: 0,
        fail_at: Some(measurement.calls),
    };
    assert_eq!(
        fixture.refresh(&[0], &mut numeric),
        Err(SerialisedAssemblyPoolUpdateError::Numeric(
            measurement.calls
        ))
    );
    assert_eq!(
        numeric.calls,
        measurement.calls + 1,
        "first bbox completed before second failed"
    );
    for (assembly, sentinel) in fixture.assemblies.iter().zip(old) {
        assert_eq!(assembly.header.aabb, sentinel);
        assert_eq!(
            assembly.header.group, 11,
            "earlier VolumeData/metadata Update is intentionally visible"
        );
        assert!(!assembly.volume_data.records()[0].enabled);
        assert_eq!(assembly.volume_frame, Some(frame));
        assert!(!assembly.has_bounds_for_frame(frame));
    }
}

#[test]
fn active_ordinals_follow_input_order_not_body_ring_or_static_filter() {
    // Add selects Frozen only for exactly 2; other nonstatic values such as
    // 6 enter Moving and cannot be used as retained frozen-state fixtures.
    let mut fixture = Fixture::new(&[4, 2, 1, 2, 4], true);
    // Input order becomes frozen,dynamic,dynamic,static,frozen: deliberately
    // distinct from both original Add order and body-state ring grouping.
    fixture.assemblies.swap(0, 1);
    fixture.parts.swap(0, 1);
    fixture.assemblies.swap(2, 4);
    fixture.parts.swap(2, 4);
    fixture.assemblies.swap(3, 4);
    fixture.parts.swap(3, 4);
    assert_eq!(
        fixture
            .parts
            .iter()
            .map(|parts| fixture.pool.try_raw_state(parts[0].body).unwrap())
            .collect::<Vec<_>>(),
        vec![2, 4, 4, 1, 2],
        "the selector consumes retained live body states, not Add request flags"
    );
    assert_eq!(
        fixture.refresh(&[0], &mut CountingNumeric::default()),
        Ok(vec![1, 2, 3])
    );
}
