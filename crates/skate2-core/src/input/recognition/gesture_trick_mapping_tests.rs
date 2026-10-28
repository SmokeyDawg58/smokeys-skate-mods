use super::*;
fn name(s: &str) -> FastString36 {
    ActorGraphIntents::native_name(s.as_bytes())
}
fn mapping() -> GestureTrickMapping {
    let mut m = GestureTrickMapping::new();
    m.try_initialize::<()>(|_, size, flags| {
        assert_eq!((size, flags), (48, 1));
        Ok(MappingAllocation::Host)
    })
    .unwrap();
    m
}
fn intents(names: &[&str]) -> ActorGraphIntents {
    let mut i = ActorGraphIntents::try_new::<()>().unwrap();
    for n in names {
        i.insert_or_assign(name(n), 0.0).unwrap();
    }
    i
}
struct Debug {
    reads: usize,
    fail: bool,
}
impl debug::GestureMappingDebug for Debug {
    type Error = &'static str;
    fn enabled(&mut self) -> Result<bool, Self::Error> {
        self.reads += 1;
        if self.fail { Err("debug") } else { Ok(false) }
    }
    fn after_strings(&mut self, _: &FastString36, _: &TrickIntents) -> Result<(), Self::Error> {
        panic!("disabled")
    }
}
fn debug() -> Debug {
    Debug {
        reads: 0,
        fail: false,
    }
}
fn blank() -> TrickIntents {
    TrickIntents {
        ordinary: name("before"),
        mirrored: name("before"),
    }
}
#[test]
fn native_bucket_order_and_head_prepend_override_action_insertion_order() {
    let m = mapping();
    let mut d = debug();
    let mut out = blank();
    // Both names hash to bucket0; N_InwardHeelflip was inserted later into mapping.
    for names in [
        ["InwardHeelflip", "N_InwardHeelflip", "Ollie"],
        ["Ollie", "N_InwardHeelflip", "InwardHeelflip"],
    ] {
        m.try_gesture_intents(
            &intents(&names),
            GestureGroup::Square,
            Some(&mut out),
            &mut d,
        )
        .unwrap();
        assert_eq!(out.ordinary, name("N_InwardHeelflip"));
    }
    assert_eq!(NativeMap::<(), 43, 45>::bucket(&name("InwardHeelflip")), 0);
    assert_eq!(NativeMap::<(), 43, 45>::bucket(&name("Ollie")), 11);
}
#[test]
fn all_original_records_are_retained_with_group_specific_fallbacks() {
    let m = mapping();
    assert_eq!(
        m.groups.each_ref().map(|g| g.len),
        [30, 30, 45, 45, 45, 45, 30]
    );
    assert_eq!(m.held.len, 4);
    let mut d = debug();
    let mut out = blank();
    m.try_gesture_intents(
        &intents(&["N_Kickflip"]),
        GestureGroup::Tail,
        Some(&mut out),
        &mut d,
    )
    .unwrap();
    assert_eq!(out.ordinary, name("Ollie"));
    m.try_gesture_intents(
        &intents(&["Kickflip"]),
        GestureGroup::Nose,
        Some(&mut out),
        &mut d,
    )
    .unwrap();
    assert_eq!(out.ordinary, name("Nollie"));
}
#[test]
fn output_copy_precedes_debug_failure_and_miss_does_not_sample_debug() {
    let m = mapping();
    let mut d = Debug {
        reads: 0,
        fail: true,
    };
    let mut out = blank();
    assert!(
        !m.try_gesture_intents(&intents(&[]), GestureGroup::Square, Some(&mut out), &mut d)
            .unwrap()
    );
    assert_eq!(d.reads, 0);
    assert_eq!(
        m.try_gesture_intents(
            &intents(&["Ollie"]),
            GestureGroup::Square,
            Some(&mut out),
            &mut d
        ),
        Err(MappingError::Collaborator("debug"))
    );
    assert_eq!(out.ordinary, name("Ollie"));
    assert_eq!(d.reads, 1);
}
#[test]
fn native_null_pair_is_only_dereferenced_by_actual_output_consumer() {
    let mut m = GestureTrickMapping::new();
    m.try_initialize::<()>(|i, _, _| {
        Ok(if i == 0 {
            MappingAllocation::Null
        } else {
            MappingAllocation::Host
        })
    })
    .unwrap();
    let mut d = debug();
    let i = intents(&["Ollie"]);
    assert!(
        m.try_gesture_intents(&i, GestureGroup::Square, None, &mut d)
            .unwrap()
    );
    let mut out = blank();
    assert_eq!(
        m.try_gesture_intents(&i, GestureGroup::Square, Some(&mut out), &mut d),
        Err(MappingError::NullPair)
    );
    assert_eq!(d.reads, 1);
}
#[test]
fn allocation_failure_retains_previous_insertions_and_cannot_restart_ctor() {
    let mut m = GestureTrickMapping::new();
    assert_eq!(
        m.try_initialize(|i, _, _| if i == 2 {
            Err("allocation")
        } else {
            Ok(MappingAllocation::Host)
        }),
        Err(MappingError::Collaborator("allocation"))
    );
    assert_eq!(m.groups[6].len, 2);
    assert!(!m.complete);
    assert_eq!(
        m.try_initialize::<()>(|_, _, _| panic!("no replay")),
        Err(MappingError::AlreadyStarted)
    );
}
#[test]
fn held_miss_leaves_existing_output_and_hit_uses_exact_original_name() {
    let m = mapping();
    let mut out = name("unchanged");
    assert!(!m.try_held_intent(&name("Ollie"), &mut out).unwrap());
    assert_eq!(out, name("unchanged"));
    assert!(m.try_held_intent(&name("N_Kickflip"), &mut out).unwrap());
    assert_eq!(out, name("N_KickflipHold"));
}
