use super::*;
use std::{cell::RefCell, rc::Rc};

#[test]
fn vault_captures_keep_instance_identity_defaults_and_defer_malformed_scalar() {
    use crate::attribute_runtime::tests::{collection, definition, graph, node};
    let class = 17;
    let mut vault = graph(
        class,
        vec![definition(LOWER, 4, 4), definition(UPPER, 4, 1)],
    );
    collection(
        &mut vault,
        class,
        7,
        0,
        None,
        vec![
            node(LOWER, 0x40, 0x8000_0000),
            node(UPPER, 0x40, 0x7f00_0000),
        ],
    );
    collection(&mut vault, class, 8, 7, None, vec![]);
    let vault = Arc::new(vault);
    let mut first = SkeletonJumpVaultAttributes::from_retained_instances(
        vault.instance(class, 8),
        vault.instance(class, 9),
    );
    let mut second = SkeletonJumpVaultAttributes::from_retained_instances(
        vault.instance(class, 8),
        vault.instance(class, 9),
    );
    let global = first.capture(SkeletonJumpGlobal::Jumping).unwrap();
    assert!(matches!(
        second.lookup(&global, LOWER),
        Err(SkeletonJumpVaultError::Foreign)
    ));
    let lower = first.lookup(&global, LOWER).unwrap();
    let malformed = first.lookup(&global, UPPER).unwrap();
    assert_eq!(first.float(&lower).unwrap().to_bits(), 0x8000_0000);
    assert!(matches!(
        first.float(&malformed),
        Err(SkeletonJumpVaultError::Runtime(
            AttributeRuntimeError::ScalarSize { size: 1 }
        ))
    ));
    assert!(matches!(
        second.float(&lower),
        Err(SkeletonJumpVaultError::Foreign)
    ));
    let adjustment = first.capture(SkeletonJumpGlobal::Adjustment).unwrap();
    let absent = first.lookup(&adjustment, RETAIN_ADJUSTMENT).unwrap();
    assert_eq!(first.byte(&absent).unwrap(), 0);
    assert_eq!(first.float(&absent).unwrap().to_bits(), 0);
}

type Log = Rc<RefCell<Vec<String>>>;
struct Input {
    log: Log,
    fail: Option<u32>,
    null: bool,
    x: f32,
    z: f32,
    retained: [f32; 2],
}
impl Input {
    fn new(log: &Log) -> Self {
        Self {
            log: Rc::clone(log),
            fail: None,
            null: false,
            x: -0.0,
            z: 0.5,
            retained: [7.0, 8.0],
        }
    }
}
impl SkeletonJumpInput for Input {
    type Input = ();
    type Error = &'static str;
    fn capture_input(&mut self) -> Result<Option<()>, Self::Error> {
        self.log.borrow_mut().push("input".into());
        Ok((!self.null).then_some(()))
    }
    fn value(&mut self, (): &(), id: u32) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(format!("value{id}"));
        if self.fail == Some(id) {
            Err("axis")
        } else {
            Ok(if id == 74 { self.x } else { self.z })
        }
    }
    fn store_adjustment_x(&mut self, value: f32) -> Result<(), Self::Error> {
        self.log.borrow_mut().push("storeX".into());
        self.retained[0] = value;
        Ok(())
    }
    fn store_adjustment_z(&mut self, value: f32) -> Result<(), Self::Error> {
        self.log.borrow_mut().push("storeZ".into());
        self.retained[1] = value;
        Ok(())
    }
    fn adjustment_x(&mut self) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push("readX".into());
        Ok(self.retained[0])
    }
    fn adjustment_z(&mut self) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push("readZ".into());
        Ok(self.retained[1])
    }
}
struct Attrs {
    log: Log,
    lower: f32,
    upper: f32,
    retain: u8,
    fail: Option<u64>,
}
impl Attrs {
    fn new(log: &Log) -> Self {
        Self {
            log: Rc::clone(log),
            lower: 0.25,
            upper: 0.75,
            retain: 1,
            fail: None,
        }
    }
}
impl SkeletonJumpAttributes for Attrs {
    type Global = SkeletonJumpGlobal;
    type Pointer = u64;
    type Error = &'static str;
    fn capture(&mut self, kind: Self::Global) -> Result<Self::Global, Self::Error> {
        self.log.borrow_mut().push(format!("capture{kind:?}"));
        Ok(kind)
    }
    fn lookup(&mut self, _: &Self::Global, key: u64) -> Result<u64, Self::Error> {
        self.log.borrow_mut().push(format!("lookup{key:x}"));
        if self.fail == Some(key) {
            Err("lookup")
        } else {
            Ok(key)
        }
    }
    fn byte(&mut self, key: &u64) -> Result<u8, Self::Error> {
        self.log.borrow_mut().push(format!("byte{key:x}"));
        Ok(if *key == ENDPOINTS { 1 } else { self.retain })
    }
    fn float(&mut self, key: &u64) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(format!("float{key:x}"));
        Ok(if *key == LOWER {
            self.lower
        } else {
            self.upper
        })
    }
}
fn record(name: &str, value: f32) -> [u8; 0x40] {
    let mut raw = [0; 0x40];
    raw[..4].copy_from_slice(&value.to_be_bytes());
    for (index, word) in PackedFastString::<5>::encode(name)
        .unwrap()
        .words
        .iter()
        .enumerate()
    {
        raw[0x20 + index * 4..0x24 + index * 4].copy_from_slice(&word.to_be_bytes());
    }
    raw
}
fn start() -> SkeletonProcessedContact {
    let mut state = SkeletonProcessedContact::new();
    state.reset();
    state.begin_jump_events();
    state
}
#[test]
fn raw_type_zero_ignores_status_and_payload_but_not_type_or_name_location() {
    let log = Log::default();
    let mut input = Input::new(&log);
    let mut state = start();
    let mut raw = record("jump", 0.0);
    raw[0x34] = 0xff;
    raw[0x35] = 3;
    assert!(!state.try_apply_jump_event(&raw, &mut input).unwrap());
    raw[0x35] = 0;
    assert!(state.try_apply_jump_event(&raw, &mut input).unwrap());
    assert_eq!(state.jump_events().known_flags(), (JUMP, PREPARE | JUMP));
    let mut misplaced = [0; 0x40];
    misplaced[..20].copy_from_slice(&raw[0x20..0x34]);
    assert!(!state.try_apply_jump_event(&misplaced, &mut input).unwrap());
    assert!(log.borrow().is_empty());
}
#[test]
fn second_prepare_axis_refusal_retains_flag_before_either_skeleton_store() {
    let log = Log::default();
    let mut input = Input::new(&log);
    input.fail = Some(75);
    let mut state = start();
    assert_eq!(
        state.try_apply_jump_event(&record("PrepareJump", f32::NAN), &mut input),
        Err(SkeletonJumpError::Input("axis"))
    );
    assert_eq!(state.jump_events().known_flags(), (PREPARE, PREPARE | JUMP));
    assert_eq!(
        input.retained.map(f32::to_bits),
        [7.0_f32.to_bits(), 8.0_f32.to_bits()]
    );
    assert_eq!(*log.borrow(), ["input", "value74", "value75"]);
    assert_eq!(
        state.try_apply_jump_event(&record("jump", 1.0), &mut input),
        Err(SkeletonJumpError::NoActivePass)
    );
    assert!(!state.jump_events().complete());
}
#[test]
fn repeated_prepare_samples_again_null_preserves_and_fsel_keeps_signed_zero() {
    let log = Log::default();
    let mut input = Input::new(&log);
    input.z = f32::NAN;
    let mut state = start();
    state
        .try_apply_jump_event(&record("PrepareJump", 0.0), &mut input)
        .unwrap();
    assert_eq!(
        input.retained.map(f32::to_bits),
        [(-0.0_f32).to_bits(), 1.0_f32.to_bits()]
    );
    input.x = -2.0;
    input.z = 2.0;
    state
        .try_apply_jump_event(&record("PrepareJump", 0.0), &mut input)
        .unwrap();
    input.null = true;
    state
        .try_apply_jump_event(&record("PrepareJump", 0.0), &mut input)
        .unwrap();
    assert_eq!(
        input.retained.map(f32::to_bits),
        [(-1.0_f32).to_bits(), 1.0_f32.to_bits()]
    );
    assert_eq!(log.borrow().iter().filter(|v| *v == "input").count(), 3);
}
#[test]
fn endpoint_unordered_branches_and_lazy_upper_lookup_match_raw() {
    for (height, lower, upper, expected, upper_read) in [
        (f32::NAN, 0.25, 0.75, 0.125, false),
        (0.5, f32::NAN, 0.75, 0.125, false),
        (0.5, 0.25, f32::NAN, 1.0, true),
        (0.5, 0.25, 0.75, 0.5, true),
    ] {
        let log = Log::default();
        let mut input = Input::new(&log);
        let mut attrs = Attrs::new(&log);
        attrs.lower = lower;
        attrs.upper = upper;
        let mut state = start();
        for raw in [
            record("jump", 0.0),
            record("MinJump", 0.875),
            record("MinJump", 0.125),
            record("TrickHeight", height),
        ] {
            state.try_apply_jump_event(&raw, &mut input).unwrap();
        }
        state
            .try_finish_jump_events(&mut input, &mut attrs)
            .unwrap();
        assert_eq!(
            state.jump_events().strength().unwrap().to_bits(),
            f32::to_bits(expected)
        );
        assert_eq!(
            log.borrow().contains(&format!("lookup{UPPER:x}")),
            upper_read
        );
        assert!(state.jump_events().complete());
    }
}
#[test]
fn finish_strength_survives_late_failure_and_live_adjustment_uses_id78() {
    let log = Log::default();
    let mut input = Input::new(&log);
    let mut attrs = Attrs::new(&log);
    attrs.retain = 0;
    input.fail = Some(78);
    let mut state = start();
    state
        .try_apply_jump_event(&record("jump", 0.0), &mut input)
        .unwrap();
    state
        .try_apply_jump_event(&record("TrickHeight", 0.5), &mut input)
        .unwrap();
    assert_eq!(
        state.try_finish_jump_events(&mut input, &mut attrs),
        Err(SkeletonJumpError::Input("axis"))
    );
    assert_eq!(
        state.jump_events().strength().unwrap().to_bits(),
        0.5_f32.to_bits()
    );
    assert_eq!(state.jump_events().adjustments().0.unwrap().to_bits(), 0);
    assert_eq!(state.jump_events().adjustments().1.unwrap().to_bits(), 0);
    assert!(
        log.borrow()
            .ends_with(&["input".into(), "value74".into(), "value78".into()])
    );
    assert!(!state.jump_events().complete());
}
#[test]
fn allocation_does_not_claim_reset_and_hippy_is_read_after_strength() {
    let log = Log::default();
    let mut input = Input::new(&log);
    let mut attrs = Attrs::new(&log);
    let mut state = SkeletonProcessedContact::new();
    state.begin_jump_events();
    assert_eq!(
        state.try_finish_jump_events(&mut input, &mut attrs),
        Err(SkeletonJumpError::UnknownJumpFlag)
    );
    assert!(log.borrow().is_empty());
    state.reset();
    state.begin_jump_events();
    state
        .try_apply_jump_event(&record("HippyJumping", 0.0), &mut input)
        .unwrap();
    state
        .try_apply_jump_event(&record("TrickHeight", 0.625), &mut input)
        .unwrap();
    state
        .try_finish_jump_events(&mut input, &mut attrs)
        .unwrap();
    assert_eq!(
        state.jump_events().strength().unwrap().to_bits(),
        0.625_f32.to_bits()
    );
    assert!(!log.borrow().iter().any(|entry| entry == "hippy"));
    assert_eq!(
        state.jump_events().adjustments().0.unwrap().to_bits(),
        7.0_f32.to_bits()
    );
}

#[test]
fn unknown_hippy_refuses_after_strength_and_raw_event_survives_next_begin() {
    let log = Log::default();
    let mut input = Input::new(&log);
    let mut attrs = Attrs::new(&log);
    let mut state = SkeletonProcessedContact::new();
    state.begin_jump_events();
    state
        .try_apply_jump_event(&record("jump", 0.0), &mut input)
        .unwrap();
    state
        .try_apply_jump_event(&record("TrickHeight", 0.625), &mut input)
        .unwrap();
    assert_eq!(
        state.try_finish_jump_events(&mut input, &mut attrs),
        Err(SkeletonJumpError::UnknownHippyFlag)
    );
    assert!(state.jump_events().strength().is_some());
    assert_eq!(state.jump_events().adjustments(), (None, None));
    state.begin_jump_events();
    let mut raw = record("HippyJumping", 0.0);
    raw[0x35] = 1;
    assert!(!state.try_apply_jump_event(&raw, &mut input).unwrap());
    assert_eq!(state.physics_state.flags_8d0, None);
    raw[0x35] = 0;
    raw[0x34] = 0xff;
    assert_eq!(
        state.try_apply_jump_event(&raw, &mut input),
        Err(SkeletonJumpError::UnknownHippyFlag)
    );
    assert_eq!(state.physics_state.flags_8d0, None);
    state.reset();
    state.begin_jump_events();
    assert!(state.try_apply_jump_event(&raw, &mut input).unwrap());
    assert_eq!(state.physics_state.flags_8d0, Some(0x8000));
    state.begin_jump_events();
    assert_eq!(state.physics_state.flags_8d0, Some(0x8000));
    state.reset();
    assert_eq!(state.physics_state.flags_8d0, Some(0));
}

#[test]
fn spin_is_source_negated_type_zero_payload_with_reset_and_last_match_retention() {
    let log = Log::default();
    let mut input = Input::new(&log);
    let mut state = SkeletonProcessedContact::new();
    assert!(state.jump_events().spin().is_none());
    state.reset();
    assert_eq!(state.jump_events().spin().unwrap().to_bits(), 0);
    state.begin_jump_events();
    for bits in [0, 0x8000_0000, 0x3f80_0000, 0x7fc1_2345] {
        let mut raw = record("Spin", f32::from_bits(bits));
        raw[0x34] = 0;
        assert!(state.try_apply_jump_event(&raw, &mut input).unwrap());
        assert_eq!(
            state.jump_events().spin().unwrap().to_bits(),
            bits ^ 0x8000_0000
        );
    }
    let mut wrong_type = record("Spin", 1.0);
    wrong_type[0x35] = 3;
    assert!(!state.try_apply_jump_event(&wrong_type, &mut input).unwrap());
    state.begin_jump_events();
    assert_eq!(state.jump_events().spin().unwrap().to_bits(), 0xffc1_2345);
    assert!(log.borrow().is_empty());
    state.reset();
    assert_eq!(state.jump_events().spin().unwrap().to_bits(), 0);
}
