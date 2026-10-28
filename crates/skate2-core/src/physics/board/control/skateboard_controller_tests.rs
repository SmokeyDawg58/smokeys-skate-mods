use crate::physics_state_ids::{PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_LOCOMOTION};
use super::*;
use crate::attribute_runtime::tests::{collection, definition, graph, node};

type Payloads = (
    Arc<RwLock<SimulationDriveFrames>>,
    Arc<RwLock<SimulationDriveDynamics>>,
);

#[test]
fn drop_board_keeps_active_state_until_release_finishes_and_preserves_failure_prefix() {
    for (initial, hiding, final_state) in [(0, false, 2), (1, false, 2), (4, true, 3)] {
        let mut owner = SkateboardController::<()>::new();
        owner.phase = SkateboardControllerPhase::Complete;
        owner.system_on = Some(true);
        owner.state = Some(initial);
        owner.hand = Some(0);
        let result = owner.try_request_drop_with(|owner, is_hiding| {
            assert_eq!((owner.state, owner.system_on), (Some(initial), Some(true)));
            assert_eq!(is_hiding, hiding);
            owner.hand = Some(2); // completed collaborator prefix, not rolled back
            Err(SkateboardControllerError::Service("release failed"))
        });
        assert_eq!(
            result,
            Err(SkateboardControllerError::Service("release failed"))
        );
        assert_eq!((owner.state, owner.hand), (Some(initial), Some(2)));
        owner
            .try_request_drop_with(|_, _| Ok::<_, SkateboardControllerError<()>>(()))
            .unwrap();
        assert_eq!(
            (owner.state, owner.system_on),
            (Some(final_state), Some(true))
        );
    }
}

#[test]
fn drop_board_inactive_assertion_precedes_free_and_hiding_noops() {
    for state in [2, 3] {
        let mut owner = SkateboardController::<()>::new();
        owner.phase = SkateboardControllerPhase::Complete;
        owner.system_on = Some(false);
        owner.state = Some(state);
        assert_eq!(
            owner.try_request_drop_with::<()>(|_, _| panic!("inactive callback")),
            Err(SkateboardControllerError::SystemNotOn)
        );
        owner.system_on = Some(true);
        owner
            .try_request_drop_with::<()>(|_, _| panic!("no-op callback"))
            .unwrap();
        assert_eq!(owner.state, Some(state));
    }
}

struct Services {
    enabled: [bool; 2],
    captures: usize,
    events: Vec<String>,
    fail: Option<&'static str>,
    admitted: Vec<Payloads>,
    null: bool,
}
impl Services {
    fn new(enabled: [bool; 2]) -> Self {
        Self {
            enabled,
            captures: 0,
            events: vec![],
            fail: None,
            admitted: vec![],
            null: false,
        }
    }
    fn event(&mut self, event: String) -> Result<(), &'static str> {
        let refused = self.fail == Some(event.as_str());
        self.events.push(event);
        if refused { Err("refused") } else { Ok(()) }
    }
}
impl SkateboardControllerServices for Services {
    type Error = &'static str;
    type Cvar = usize;
    type Simulation = ();
    type Body = usize;
    type Drive = usize;
    fn capture_use_hand_drives(&mut self) -> Result<usize, Self::Error> {
        let target = self.captures;
        self.event(format!("capture{target}"))?;
        self.captures += 1;
        Ok(target)
    }
    fn use_hand_drives(&mut self, target: &usize) -> Result<bool, Self::Error> {
        self.event(format!("value{target}"))?;
        Ok(self.enabled[*target])
    }
    fn capture_simulation(&mut self) -> Result<(), Self::Error> {
        self.event("simulation".into())
    }
    fn skeleton_body(&mut self, part: usize) -> Result<usize, Self::Error> {
        self.event(format!("hand{part}"))?;
        Ok(part)
    }
    fn deck_body(&mut self) -> Result<usize, Self::Error> {
        self.event("deck".into())?;
        Ok(6)
    }
    fn add_drive(
        &mut self,
        (): &(),
        deck: usize,
        hand: usize,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<Option<usize>, Self::Error> {
        self.event(format!("add{hand}"))?;
        assert_eq!(deck, 6);
        assert_eq!(dynamics.read().unwrap().linear.raw_type, 2);
        assert_eq!(dynamics.read().unwrap().angular.strength.to_bits(), 0);
        self.admitted
            .push((Arc::clone(frames), Arc::clone(dynamics)));
        Ok((!self.null).then_some(hand))
    }
    fn remove_drive(&mut self, (): &(), drive: &usize) -> Result<(), Self::Error> {
        self.event(format!("remove{drive}"))
    }
}
fn vault() -> Arc<AttributeRuntimeVault> {
    Arc::new(graph(CLASS, vec![]))
}

#[test]
fn actual_admission_payloads_and_repeated_cvar_reads_follow_source_order() {
    let mut owner = SkateboardController::new();
    assert!(owner.retrieval().is_none());
    assert!(owner.system_on().is_none());
    let occurrence = Arc::clone(owner.occurrence());
    let mut services = Services::new([true, true]);
    owner.try_construct(&vault(), &mut services).unwrap();
    assert_eq!(
        services.events,
        [
            "capture0",
            "value0",
            "capture1",
            "value1",
            "simulation",
            "hand3",
            "deck",
            "add3",
            "hand7",
            "deck",
            "add7"
        ]
    );
    assert!(Arc::ptr_eq(&occurrence, owner.occurrence()));
    for slot in 0..2 {
        assert!(Arc::ptr_eq(
            owner.frames(slot).unwrap(),
            &services.admitted[slot].0
        ));
        assert!(Arc::ptr_eq(
            owner.dynamics(slot).unwrap(),
            &services.admitted[slot].1
        ));
        let frame = owner.frames(slot).unwrap().read().unwrap();
        assert_eq!(frame.position_a.map(f32::to_bits), [0; 4]);
        assert_eq!(
            frame.quaternion_b.map(f32::to_bits),
            [0, 0, 0, 1.0_f32.to_bits()]
        );
    }
    assert_eq!(owner.drive(0), Some(&3));
    assert_eq!(owner.drive(1), Some(&7));
    let retrieval = owner.retrieval().unwrap();
    for matrix in [retrieval.start, retrieval.target, retrieval.current] {
        assert_eq!(
            matrix.known_lanes().unwrap().map(f32::to_bits),
            IDENTITY.map(f32::to_bits)
        );
    }
    assert_eq!(
        [
            retrieval.time,
            retrieval.total_time,
            retrieval.parametric_time,
            retrieval.warped_parametric_time
        ]
        .map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(owner.hand(), Some(2));
    assert_eq!(owner.state(), Some(0));
    owner.try_stop::<&str>().unwrap();
    for state in [100, 103, 200, 201, 499, 505, 700] {
        owner.try_enter_physics_state::<&str>(state).unwrap();
    }
    assert_eq!(
        owner.try_construct(&vault(), &mut services),
        Err(SkateboardControllerError::NotFresh(
            SkateboardControllerPhase::Complete
        ))
    );
}

#[test]
fn both_cvar_gates_are_independent_and_capacity_null_is_an_actual_outcome() {
    for (enabled, expected) in [([false, true], 2), ([true, false], 4)] {
        let mut owner = SkateboardController::new();
        let mut services = Services::new(enabled);
        owner.try_construct(&vault(), &mut services).unwrap();
        assert_eq!(services.events.len(), expected);
        assert_eq!(owner.phase(), SkateboardControllerPhase::Complete);
        assert_eq!(
            owner.dynamics(0).unwrap().read().unwrap().linear.raw_type,
            0
        );
    }
    let mut owner = SkateboardController::new();
    let mut services = Services::new([true, true]);
    services.null = true;
    owner.try_construct(&vault(), &mut services).unwrap();
    assert_eq!(services.admitted.len(), 2);
    assert!(owner.drive(0).is_none());
    assert!(owner.drive(1).is_none());
    assert_eq!(
        owner.dynamics(1).unwrap().read().unwrap().linear.raw_type,
        2
    );
}

#[test]
fn late_body_failure_retains_first_admission_and_refuses_constructor_replay() {
    let mut owner = SkateboardController::new();
    let mut services = Services::new([true, true]);
    services.fail = Some("hand7");
    assert_eq!(
        owner.try_construct(&vault(), &mut services),
        Err(SkateboardControllerError::Service("refused"))
    );
    assert_eq!(owner.drive(0), Some(&3));
    assert!(owner.drive(1).is_none());
    assert!(Arc::ptr_eq(
        owner.frames(0).unwrap(),
        &services.admitted[0].0
    ));
    assert_eq!(owner.system_on(), Some(false));
    assert_eq!(
        owner.try_stop::<&str>(),
        Err(SkateboardControllerError::NotComplete(
            SkateboardControllerPhase::InitHandDrives
        ))
    );
    let before = services.events.len();
    assert_eq!(
        owner.try_construct(&vault(), &mut services),
        Err(SkateboardControllerError::NotFresh(
            SkateboardControllerPhase::InitHandDrives
        ))
    );
    assert_eq!(services.events.len(), before);
    let mut cleanup = Services::new([true, true]);
    owner.try_remove_drives(&mut cleanup).unwrap();
    assert_eq!(
        cleanup.events,
        ["capture0", "value0", "simulation", "remove3"]
    );
    assert!(owner.drive(0).is_none());
}

#[test]
fn active_release_refuses_before_stores_and_invariant_is_checked_after_clear() {
    let mut owner = SkateboardController::new();
    owner
        .try_construct(&vault(), &mut Services::new([false, false]))
        .unwrap();
    owner.system_on = Some(true);
    owner.state = Some(1);
    owner.hand = Some(0);
    assert_eq!(
        owner.try_stop::<&str>(),
        Err(SkateboardControllerError::PendingLetGo)
    );
    assert_eq!(owner.system_on(), Some(true));
    assert_eq!(owner.state(), Some(1));
    owner.state = Some(0);
    assert_eq!(
        owner.try_stop::<&str>(),
        Err(SkateboardControllerError::HandInvariant {
            system_on: false,
            hand: 0
        })
    );
    assert_eq!(owner.system_on(), Some(false));
    for state in PHYSICS_STATE_OFFBOARD..=PHYSICS_STATE_OFFBOARD_LOCOMOTION {
        assert_eq!(
            owner.try_enter_physics_state::<&str>(state),
            Err(SkateboardControllerError::PendingStartController)
        );
    }
}

#[test]
fn active_stop_calls_release_before_clearing_state_and_system() {
    let mut owner = SkateboardController::new();
    owner
        .try_construct(&vault(), &mut Services::new([false, false]))
        .unwrap();
    owner.system_on = Some(true);
    owner.state = Some(1);
    owner.hand = Some(0);
    assert_eq!(
        owner.try_stop_with::<&str>(|current| {
            assert_eq!(
                (current.system_on(), current.state(), current.hand()),
                (Some(true), Some(1), Some(0))
            );
            Err(SkateboardControllerError::Service("release pending"))
        }),
        Err(SkateboardControllerError::Service("release pending"))
    );
    assert_eq!(
        (owner.system_on(), owner.state(), owner.hand()),
        (Some(true), Some(1), Some(0))
    );
    owner
        .try_stop_with::<&str>(|current| {
            assert_eq!(
                (current.system_on(), current.state()),
                (Some(true), Some(1))
            );
            current.hand = Some(2);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        (owner.system_on(), owner.state(), owner.hand()),
        (Some(false), Some(0), Some(2))
    );
}

#[test]
fn constructor_retains_original_controller_default_instance() {
    const KEY: u64 = 77;
    let mut source = graph(CLASS, vec![definition(KEY, 4, 4)]);
    collection(
        &mut source,
        CLASS,
        DEFAULT,
        0,
        None,
        vec![node(KEY, 0x40, 123)],
    );
    let source = Arc::new(source);
    let mut owner = SkateboardController::new();
    owner
        .try_construct(&source, &mut Services::new([false, false]))
        .unwrap();
    // Binding proof uses the retained instance rather than a second lookup owner.
    let value = owner.attributes().unwrap().try_scalar_word(KEY, 0).unwrap();
    assert_eq!(value, Some(123));
}

#[test]
fn removal_preserves_late_gate_and_each_success_before_a_late_failure() {
    let mut owner = SkateboardController::new();
    owner
        .try_construct(&vault(), &mut Services::new([true, true]))
        .unwrap();
    let mut disabled = Services::new([false, false]);
    owner.try_remove_drives(&mut disabled).unwrap();
    assert_eq!(disabled.events, ["capture0", "value0"]);
    assert_eq!(owner.drive(0), Some(&3));
    let mut removal = Services::new([true, true]);
    removal.fail = Some("remove7");
    assert_eq!(
        owner.try_remove_drives(&mut removal),
        Err(SkateboardControllerError::Service("refused"))
    );
    assert_eq!(
        removal.events,
        ["capture0", "value0", "simulation", "remove3", "remove7"]
    );
    assert!(owner.drive(0).is_none());
    assert_eq!(owner.drive(1), Some(&7));
    let mut removal = Services::new([true, true]);
    owner.try_remove_drives(&mut removal).unwrap();
    assert_eq!(
        removal.events,
        ["capture0", "value0", "simulation", "remove7"]
    );
    assert!(owner.drive(1).is_none());
}

#[test]
fn start_controller_holds_with_source_order_and_skips_every_read_once_enabled() {
    use std::cell::RefCell;
    let mut owner = SkateboardController::<usize>::new();
    owner
        .try_construct(&vault(), &mut Services::new([false, false]))
        .unwrap();
    let events = RefCell::new(Vec::new());
    owner
        .try_start_with(
            || {
                events.borrow_mut().push("8d0");
                Ok::<_, &'static str>(0)
            },
            || {
                events.borrow_mut().push("hold board");
                Ok(())
            },
            || {
                events.borrow_mut().push("8cc");
                Ok(8)
            },
            |_, hand| {
                assert_eq!(hand, 0);
                events.borrow_mut().push("drive");
                Ok(())
            },
            |_| panic!("LetGo is not reached"),
        )
        .unwrap();
    assert_eq!(*events.borrow(), ["8d0", "hold board", "8cc", "drive"]);
    assert_eq!(
        (owner.state(), owner.hand(), owner.system_on()),
        (Some(1), Some(0), Some(true))
    );
    owner
        .try_start_with::<&'static str>(
            || panic!("enabled controller must not read processed flags"),
            || panic!("enabled controller must not prepare Hold"),
            || panic!("enabled controller must not read hand flags"),
            |_, _| panic!("enabled controller must not enable a drive"),
            |_| panic!("enabled controller must not LetGo"),
        )
        .unwrap();
}

#[test]
fn start_controller_free_branch_and_late_hold_failure_keep_native_prefix() {
    let mut owner = SkateboardController::<usize>::new();
    owner
        .try_construct(&vault(), &mut Services::new([false, false]))
        .unwrap();
    let mut let_go_count = 0;
    owner
        .try_start_with(
            || Ok::<_, &'static str>(0x400),
            || panic!("free branch cannot Hold"),
            || panic!("free branch cannot read hand flags"),
            |_, _| panic!("free branch cannot enable drive"),
            |controller| {
                assert_eq!(
                    (controller.state(), controller.system_on()),
                    (Some(0), Some(false))
                );
                let_go_count += 1;
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(let_go_count, 1);
    assert_eq!(
        (owner.state(), owner.hand(), owner.system_on()),
        (Some(2), Some(2), Some(true))
    );

    let mut owner = SkateboardController::<usize>::new();
    owner
        .try_construct(&vault(), &mut Services::new([false, false]))
        .unwrap();
    assert_eq!(
        owner.try_start_with(
            || Ok::<_, &'static str>(0),
            || Ok(()),
            || Ok(0),
            |_, _| Err("drive failed"),
            |_| panic!("Hold branch cannot LetGo"),
        ),
        Err(SkateboardControllerError::Service("drive failed"))
    );
    assert_eq!(
        (owner.state(), owner.hand(), owner.system_on()),
        (Some(1), Some(1), Some(false))
    );
}

#[test]
fn hiding_retrieval_resets_before_failed_read_and_copies_unknown_w() {
    use crate::skeleton_root_transforms::SkeletonRootVector;
    let mut owner = SkateboardController::<()>::new();
    let dirty = SkeletonRootMatrix::known([9.0; 16]);
    owner.retrieval = Some(ControllerRetrieval {
        start: dirty,
        target: dirty,
        current: dirty,
        time: 9.0,
        total_time: 8.0,
        parametric_time: 7.0,
        warped_parametric_time: 6.0,
    });
    assert_eq!(
        owner.try_initialize_hiding_retrieval(|| Err("deck missing")),
        Err(SkateboardControllerError::Service("deck missing"))
    );
    let r = owner.retrieval().unwrap();
    assert_eq!(r.start, SkeletonRootMatrix::known(IDENTITY));
    assert_eq!(r.target, r.start);
    assert_eq!(r.current, r.start);
    assert_eq!(
        [
            r.time,
            r.total_time,
            r.parametric_time,
            r.warped_parametric_time
        ],
        [0.0; 4]
    );
    let deck = SkeletonRootMatrix::from_columns(std::array::from_fn(|i| SkeletonRootVector {
        xyz: [i as f32, -0.0, 12.0],
        w: if i == 3 { None } else { Some(i as f32) },
    }));
    owner
        .try_initialize_hiding_retrieval(|| Ok::<_, &str>(deck))
        .unwrap();
    let r = owner.retrieval().unwrap();
    assert_eq!(r.start, deck);
    assert_eq!(r.start.columns()[3].w, None);
    assert_eq!(r.start.columns()[3].xyz[1].to_bits(), (-0.0f32).to_bits());
    assert_eq!(r.target, SkeletonRootMatrix::known(IDENTITY));
    assert_eq!(r.current, r.target);
}

#[test]
fn start_free_release_retains_partial_effects_without_publishing_success() {
    let mut owner = SkateboardController::<()>::new();
    owner.phase = SkateboardControllerPhase::Complete;
    owner.state = Some(4);
    owner.system_on = Some(false);
    owner.hand = Some(0);
    let result = owner.try_start_with(
        || Ok::<_, &str>(0x800),
        || panic!("free branch"),
        || panic!("free branch"),
        |_, _| panic!("free branch"),
        |controller| {
            assert_eq!(controller.state(), Some(4));
            controller.hand = Some(2);
            Err("late release error")
        },
    );
    assert_eq!(
        result,
        Err(SkateboardControllerError::Service("late release error"))
    );
    assert_eq!(
        (owner.state(), owner.system_on(), owner.hand()),
        (Some(4), Some(false), Some(2))
    );
}

#[test]
fn returning_hold_reuses_hold_even_when_system_already_on_and_keeps_failure_prefix() {
    use std::cell::RefCell;
    let mut owner = SkateboardController::<usize>::new();
    owner
        .try_construct(&vault(), &mut Services::new([false, false]))
        .unwrap();
    owner.system_on = Some(true);
    owner.state = Some(4);
    let calls = RefCell::new(Vec::new());
    let result = owner.try_start_or_hold_with(
        false,
        || panic!("Hold does not execute Start's initial flags read"),
        || {
            calls.borrow_mut().push("board");
            Ok::<_, &str>(())
        },
        || {
            calls.borrow_mut().push("hand flags");
            Ok(8)
        },
        |owner, hand| {
            assert_eq!((owner.state(), owner.hand()), (Some(1), Some(0)));
            assert_eq!(hand, 0);
            calls.borrow_mut().push("hand drive");
            Err("drive failed")
        },
        |_| panic!("Hold does not release"),
    );
    assert_eq!(
        result,
        Err(SkateboardControllerError::Service("drive failed"))
    );
    assert_eq!(*calls.borrow(), ["board", "hand flags", "hand drive"]);
    assert_eq!(
        (owner.state(), owner.hand(), owner.system_on()),
        (Some(1), Some(0), Some(true))
    );
}
