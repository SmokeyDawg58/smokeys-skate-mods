use super::*;
use crate::actor_graph_construction::ActorGraphIntents;
fn name(s: &[u8]) -> FastString36 {
    ActorGraphIntents::native_name(s)
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source predicate and failure controls"
)]
struct Source {
    trace: Vec<&'static str>,
    found: bool,
    mirror: bool,
    dont: bool,
    held: bool,
    fail: Option<&'static str>,
    map: Vec<FastString36>,
}
impl Source {
    fn new() -> Self {
        Self {
            trace: Vec::new(),
            found: true,
            mirror: false,
            dont: false,
            held: true,
            fail: None,
            map: Vec::new(),
        }
    }
    fn event(&mut self, s: &'static str) -> Result<(), &'static str> {
        self.trace.push(s);
        if self.fail == Some(s) { Err(s) } else { Ok(()) }
    }
}
impl GestureTrickSource for Source {
    type Error = &'static str;
    type Action = ();
    type Motion = ();
    type Animation = ();
    type ActionMap = ();
    type MotionMap = ();
    type Mapping = ();
    fn resolve_action(&mut self) -> Result<(), Self::Error> {
        self.event("action")
    }
    fn resolve_motion(&mut self) -> Result<(), Self::Error> {
        self.event("motion")
    }
    fn resolve_animation(&mut self) -> Result<(), Self::Error> {
        self.event("animation")
    }
    fn require_begin_interfaces(&mut self, (): &(), (): &(), (): &()) -> Result<(), Self::Error> {
        self.event("require-all")
    }
    fn require_motion(&mut self, (): &()) -> Result<(), Self::Error> {
        self.event("require-motion")
    }
    fn motion_map(&mut self, (): &()) -> Result<(), Self::Error> {
        self.event("motion-map")
    }
    fn action_map(&mut self, (): &()) -> Result<(), Self::Error> {
        self.event("action-map")
    }
    fn mapping(&mut self) -> Result<(), Self::Error> {
        self.event("mapping")
    }
    fn gesture_intents(
        &mut self,
        (): &(),
        (): &(),
        _: GestureGroup,
    ) -> Result<Option<TrickIntents>, Self::Error> {
        self.event("lookup")?;
        Ok(self.found.then(|| TrickIntents {
            ordinary: name(b"Ollie"),
            mirrored: name(b"Nollie"),
        }))
    }
    fn mirrored(&mut self, (): &()) -> Result<bool, Self::Error> {
        self.event("mirror")?;
        Ok(self.mirror)
    }
    fn dont_mirror(&mut self, (): &()) -> Result<bool, Self::Error> {
        self.event("dont")?;
        Ok(self.dont)
    }
    fn insert(&mut self, (): &(), name: &FastString36, value: f32) -> Result<(), Self::Error> {
        self.event("insert")?;
        assert_eq!(value.to_bits(), 0);
        self.map.push(*name);
        Ok(())
    }
    fn erase(&mut self, (): &(), name: &FastString36) -> Result<(), Self::Error> {
        self.event("erase")?;
        self.map.retain(|n| n != name);
        Ok(())
    }
    fn held_intent(
        &mut self,
        (): &(),
        _: &FastString36,
        destination: &mut FastString36,
    ) -> Result<bool, Self::Error> {
        self.event("held")?;
        if self.held {
            *destination = name(b"HeldOllie");
        }
        Ok(self.held)
    }
}
#[test]
fn missing_match_does_not_dereference_null_instance_or_touch_motion() {
    let mut source = Source::new();
    source.found = false;
    begin(&mut source, None, GestureGroup::Square, None).unwrap();
    assert_eq!(
        source.trace,
        [
            "action",
            "motion",
            "animation",
            "require-all",
            "motion-map",
            "action-map",
            "mapping",
            "lookup"
        ]
    );
    assert!(source.map.is_empty());
}
#[test]
fn successful_lookup_hits_null_found_store_before_mirror_or_insert() {
    let mut source = Source::new();
    assert_eq!(
        begin(&mut source, None, GestureGroup::Square, None),
        Err(GestureTrickError::NullInstance)
    );
    assert_eq!(source.trace.last(), Some(&"lookup"));
    assert!(source.map.is_empty());
}
#[test]
fn override_skips_both_mirror_reads_but_still_requires_original_mapping() {
    let mut source = Source::new();
    let mut instance = GestureTrickInstance::new();
    begin(
        &mut source,
        Some(&mut instance),
        GestureGroup::Tail,
        Some(&name(b"Override")),
    )
    .unwrap();
    assert!(!source.trace.contains(&"mirror"));
    assert!(!source.trace.contains(&"dont"));
    assert_eq!(instance.gesture, name(b"Override"));
    assert!(instance.first_update);
    assert_eq!(&source.trace[6..], ["mapping", "lookup", "insert", "held"]);
}
#[test]
fn mirror_gate_and_partial_failure_preserve_found_before_name_copy() {
    let mut source = Source::new();
    source.mirror = true;
    source.fail = Some("insert");
    let mut instance = GestureTrickInstance::new();
    assert_eq!(
        begin(&mut source, Some(&mut instance), GestureGroup::Square, None),
        Err(GestureTrickError::Source("insert"))
    );
    assert!(instance.found);
    assert_eq!(instance.gesture, name(b""));
    assert!(!instance.is_held);
    assert_eq!(&source.trace[8..], ["mirror", "dont", "insert"]);
}
#[test]
fn first_update_skips_interfaces_then_held_swap_and_end_use_source_map_calls() {
    let mut source = Source::new();
    let mut instance = GestureTrickInstance::new();
    begin(&mut source, Some(&mut instance), GestureGroup::Square, None).unwrap();
    source.trace.clear();
    update(&mut source, Some(&mut instance)).unwrap();
    assert!(source.trace.is_empty());
    update(&mut source, Some(&mut instance)).unwrap();
    assert_eq!(
        source.trace,
        ["motion", "require-motion", "motion-map", "erase", "insert"]
    );
    assert_eq!(source.map, [name(b"HeldOllie")]);
    source.trace.clear();
    end(&mut source, Some(&instance)).unwrap();
    assert_eq!(
        source.trace,
        [
            "motion",
            "require-motion",
            "motion-map",
            "erase",
            "motion-map",
            "erase"
        ]
    );
    assert!(source.map.is_empty());
    assert!(instance.found);
    assert!(instance.is_held);
}
#[test]
fn failed_held_insert_preserves_prior_erase() {
    let mut source = Source::new();
    let mut instance = GestureTrickInstance::new();
    begin(&mut source, Some(&mut instance), GestureGroup::Square, None).unwrap();
    update(&mut source, Some(&mut instance)).unwrap();
    source.fail = Some("insert");
    assert_eq!(
        update(&mut source, Some(&mut instance)),
        Err(GestureTrickError::Source("insert"))
    );
    assert!(source.map.is_empty());
    assert!(instance.found);
}
