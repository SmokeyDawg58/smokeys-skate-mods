use super::super::grind_runtime::*;
use super::*;
use crate::{
    grind_data_octree::OctreeBounds,
    grind_data_runtime::{
        DynamicGrindSplineProvider, GrindDataError, GrindDataQueryResult, GrindDataRuntime,
        HostGrindDataNumeric,
    },
};

fn spline_bytes(count: usize) -> Vec<u8> {
    let mut bytes = vec![0; 16 + count * 144];
    put_u32(&mut bytes, 4, u32::try_from(count).unwrap());
    put_u32(&mut bytes, 8, 16);
    put_u32(&mut bytes, 12, 16);
    for index in 0..count {
        let offset = 16 + index * 144;
        put_u32(&mut bytes, offset, 1.0_f32.to_bits());
        for lane in 0..4 {
            put_u32(&mut bytes, offset + 80 + lane * 4, (-4.0_f32).to_bits());
            put_u32(&mut bytes, offset + 96 + lane * 4, 4.0_f32.to_bits());
        }
    }
    bytes
}
fn grind_plan(second_count: usize) -> Arc<SimulationNormalArenaPlan> {
    let mut fixture = ArenaFixture::new();
    let painter = fixture.push(WORLD_PAINTER_DATA_TYPE_ID_0F, &[0]);
    let first_bytes = spline_bytes(1);
    let second_bytes = spline_bytes(second_count);
    let first = fixture.push(SPLINE_DATA_TYPE_ID, &first_bytes);
    let rain = fixture.push(RAIN_DATA_TYPE_ID, &[0]);
    let second = fixture.push(SPLINE_DATA_TYPE_ID, &second_bytes);
    let ai = fixture.push(AI_PATH_DATA_TYPE_ID, &[0]);
    fixture.push(
        TABLE_OF_CONTENTS_TYPE_ID,
        &toc(&[
            (0x11, WORLD_PAINTER_DATA_TYPE_ID_0F, painter),
            (0x1234_5678_0000_0020, SPLINE_DATA_TYPE_ID, first),
            (0x33, RAIN_DATA_TYPE_ID, rain),
            (0x1234_5678_0000_0020, SPLINE_DATA_TYPE_ID, second),
            (0x55, AI_PATH_DATA_TYPE_ID, ai),
        ]),
    );
    fixture.finish(
        vec![
            SimulationStaticOperation::Grind {
                source_toc_index: 1,
                source_dictionary_index: first.entry_index(),
                grind_asset_index: 0,
            },
            SimulationStaticOperation::Grind {
                source_toc_index: 3,
                source_dictionary_index: second.entry_index(),
                grind_asset_index: 1,
            },
        ],
        vec![
            GrindAsset::from_spline_data(
                0x1234_5678_0000_0020,
                first.entry_index(),
                &SplineData::parse(&first_bytes).unwrap(),
            ),
            GrindAsset::from_spline_data(
                0x1234_5678_0000_0020,
                second.entry_index(),
                &SplineData::parse(&second_bytes).unwrap(),
            ),
        ],
    )
}
fn runtime() -> Arc<Mutex<GrindDataRuntime>> {
    let mut runtime = GrindDataRuntime::new();
    runtime.try_initialize().unwrap();
    Arc::new(Mutex::new(runtime))
}
struct UncalledDynamic;
impl DynamicGrindSplineProvider for UncalledDynamic {
    type Error = Infallible;
    fn query(&mut self, _: OctreeBounds, _: &mut GrindDataQueryResult) -> Result<(), Self::Error> {
        panic!("native absent provider must not be called")
    }
}
fn ids(runtime: &Arc<Mutex<GrindDataRuntime>>) -> Vec<u64> {
    let mut result = GrindDataQueryResult::new();
    runtime
        .lock()
        .unwrap()
        .try_box_test::<_, UncalledDynamic>(
            OctreeBounds {
                minimum: [-8.0; 4],
                maximum: [8.0; 4],
            },
            &mut result,
            false,
            &mut HostGrindDataNumeric,
            None,
        )
        .unwrap();
    result
        .results()
        .map(|primitive| primitive.source.asset_id())
        .collect()
}
fn expected_non_grind(add: bool, suffix: bool) -> Vec<Event> {
    let mut events = vec![
        Event::Registry(SimulationRegistryAssetKind::WorldPainter, 0x11, add, None),
        Event::Registry(SimulationRegistryAssetKind::Rain, 0x33, add, None),
    ];
    if suffix {
        events.push(Event::Registry(
            SimulationRegistryAssetKind::AiPath,
            0x55,
            add,
            None,
        ));
    }
    events
}

#[test]
fn actual_add_and_forward_remove_share_sources_and_never_call_legacy_grind() {
    let prepared = PreparedNormalGrindArena::try_new(grind_plan(1)).unwrap();
    let recorder = Recorder::default();
    let runtime = runtime();
    let mut adapter =
        SimulationNormalGrindArenaAdapter::new(recorder.clone(), router(1), Arc::clone(&runtime));
    let first = adapter.register(&prepared, 0).unwrap();
    assert_eq!(recorder.events(), expected_non_grind(true, true));
    assert_eq!(ids(&runtime), [0x1234_5678_0000_0020; 2]);
    let second = adapter.register(&prepared, 0).unwrap();
    assert_eq!(ids(&runtime).len(), 4);
    recorder.clear_events();
    adapter.deregister(first, 0).unwrap();
    // Native Remove erases ALL equal GUIDs, including repeated registrations.
    assert!(ids(&runtime).is_empty());
    assert_eq!(recorder.events(), expected_non_grind(false, true));
    assert_eq!(adapter.live_registration_count(), 1);
    adapter.deregister(second, 0).unwrap();
    assert_eq!(adapter.live_registration_count(), 0);
}

#[test]
fn late_constructor_refusal_keeps_prior_asset_and_interleaved_prefix() {
    let prepared = PreparedNormalGrindArena::try_new(grind_plan(0)).unwrap();
    let recorder = Recorder::default();
    let runtime = runtime();
    let mut adapter =
        SimulationNormalGrindArenaAdapter::new(recorder.clone(), router(1), Arc::clone(&runtime));
    let error = adapter.register(&prepared, 0).unwrap_err();
    assert!(matches!(
        error,
        NormalGrindRegistrationError::Grind {
            source: NormalGrindDispatchError::Data(GrindDataError::Source(_)),
            ..
        }
    ));
    assert_eq!(adapter.fault().unwrap().operation_index(), 3);
    assert_eq!(
        adapter.fault().unwrap().cause(),
        SimulationNormalArenaFaultCause::GrindRefusal
    );
    assert_eq!(recorder.events(), expected_non_grind(true, false));
    assert_eq!(ids(&runtime), [0x1234_5678_0000_0020]);
    assert_eq!(adapter.live_registration_count(), 1);
    assert!(matches!(
        adapter.register(&prepared, 0),
        Err(NormalGrindRegistrationError::Arena(
            SimulationNormalArenaRegistrationError::AdapterFaulted(_)
        ))
    ));
}

#[test]
fn foreign_route_returns_wrapped_authority_without_grind_effects() {
    let prepared = PreparedNormalGrindArena::try_new(grind_plan(1)).unwrap();
    let runtime = runtime();
    let mut owner = SimulationNormalGrindArenaAdapter::new(
        Recorder::default(),
        router(1),
        Arc::clone(&runtime),
    );
    let mut foreign = SimulationNormalGrindArenaAdapter::new(
        Recorder::default(),
        router(1),
        Arc::clone(&runtime),
    );
    let token = owner.register(&prepared, 0).unwrap();
    let token = foreign
        .deregister(token, 0)
        .unwrap_err()
        .into_registration()
        .unwrap();
    assert_eq!(ids(&runtime).len(), 2);
    owner.deregister(token, 0).unwrap();
    assert!(ids(&runtime).is_empty());
}

#[test]
fn late_deregister_lock_refusal_consumes_token_and_keeps_live_ledger() {
    let prepared = PreparedNormalGrindArena::try_new(grind_plan(1)).unwrap();
    let recorder = Recorder::default();
    let runtime = runtime();
    let mut adapter =
        SimulationNormalGrindArenaAdapter::new(recorder.clone(), router(1), Arc::clone(&runtime));
    let token = adapter.register(&prepared, 0).unwrap();
    recorder.clear_events();
    let broken = Arc::clone(&runtime);
    assert!(
        std::thread::spawn(move || {
            let _guard = broken.lock().unwrap();
            panic!("grind owner poison");
        })
        .join()
        .is_err()
    );
    let error = adapter.deregister(token, 0).unwrap_err();
    assert!(matches!(
        &error,
        NormalGrindDeregistrationError::Grind {
            source: NormalGrindDispatchError::OwnerLock,
            ..
        }
    ));
    assert!(error.into_registration().is_none());
    assert_eq!(adapter.fault().unwrap().operation_index(), 1);
    assert_eq!(adapter.live_registration_count(), 1);
    assert_eq!(
        recorder.events(),
        [Event::Registry(
            SimulationRegistryAssetKind::WorldPainter,
            0x11,
            false,
            None
        )]
    );
}
