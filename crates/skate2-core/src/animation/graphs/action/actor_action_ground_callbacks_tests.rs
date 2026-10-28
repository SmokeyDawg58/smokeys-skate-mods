use super::*;
struct Services {
    events: Vec<&'static str>,
    front: bool,
    back: bool,
    air: bool,
    dt: f32,
    body: f32,
    takeoff: f32,
    fail: Option<&'static str>,
    inserted: Option<f32>,
}
impl Services {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            front: false,
            back: false,
            air: false,
            dt: 0.1,
            body: 0.5,
            takeoff: 0.6,
            fail: None,
            inserted: None,
        }
    }
    fn event(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.events.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl GroundActionServices for Services {
    type Error = &'static str;
    fn action_intent_exists(&mut self, name: &FastString36) -> Result<bool, Self::Error> {
        if *name == ActorGraphIntents::native_name(b"FrontFlip") {
            self.event("front")?;
            Ok(self.front)
        } else {
            self.event("back")?;
            Ok(self.back)
        }
    }
    fn require_motion(&mut self) -> Result<(), Self::Error> {
        self.event("motion")
    }
    fn filtered_state(&mut self) -> Result<FilteredPhysicsState, Self::Error> {
        self.event("physical")?;
        Ok(if self.air {
            FilteredPhysicsState::Air
        } else {
            FilteredPhysicsState::Ground
        })
    }
    fn delta_time(&mut self) -> Result<f32, Self::Error> {
        self.event("dt")?;
        Ok(self.dt)
    }
    fn body_flip_extension(&mut self) -> Result<f32, Self::Error> {
        self.event("bodylimit")?;
        Ok(self.body)
    }
    fn takeoff_extension(&mut self) -> Result<f32, Self::Error> {
        self.event("airlimit")?;
        Ok(self.takeoff)
    }
    fn insert_motion(&mut self, _: &FastString36, value: f32) -> Result<(), Self::Error> {
        self.event("insert")?;
        self.inserted = Some(value);
        Ok(())
    }
    fn erase_motion(&mut self, name: &FastString36) -> Result<(), Self::Error> {
        self.event(if *name == ActorGraphIntents::native_name(b"FrontFlip") {
            "erasefront"
        } else {
            "eraseback"
        })
    }
}
#[test]
fn begin_second_read_failure_preserves_first_native_store() {
    let mut instance = BodyFlippingSignalInstance::new();
    let mut s = Services::new();
    s.fail = Some("airlimit");
    assert_eq!(instance.begin(&mut s), Err("airlimit"));
    assert_eq!(instance.body_flip_time.to_bits(), 0.5_f32.to_bits());
    assert_eq!(instance.time_in_air.to_bits(), 0.0_f32.to_bits());
    assert!(!instance.is_flipping);
}
#[test]
fn false_body_window_still_samples_physical_then_erases_both_names() {
    let mut instance = BodyFlippingSignalInstance::new();
    let mut s = Services::new();
    instance.begin(&mut s).unwrap();
    s.events.clear();
    instance.update(&mut s).unwrap();
    assert_eq!(
        s.events,
        [
            "front",
            "back",
            "bodylimit",
            "dt",
            "bodylimit",
            "physical",
            "motion",
            "erasefront",
            "eraseback"
        ]
    );
}
#[test]
fn front_precedes_back_and_takeoff_lookup_is_repeated_after_positive_time() {
    let mut instance = BodyFlippingSignalInstance::new();
    let mut s = Services::new();
    s.front = true;
    s.back = true;
    s.air = true;
    instance.update(&mut s).unwrap();
    assert_eq!(
        s.events,
        [
            "front",
            "bodylimit",
            "physical",
            "airlimit",
            "dt",
            "airlimit",
            "motion",
            "insert"
        ]
    );
    assert_eq!(
        instance.last_flip_name,
        ActorGraphIntents::native_name(b"FrontFlip")
    );
    assert_eq!(s.inserted.unwrap().to_bits(), 1.0_f32.to_bits());
}
#[test]
fn unordered_takeoff_timer_does_not_read_final_limit_or_insert() {
    let mut instance = BodyFlippingSignalInstance::new();
    let mut s = Services::new();
    s.front = true;
    s.air = true;
    s.dt = f32::NAN;
    instance.update(&mut s).unwrap();
    assert!(instance.time_in_air.is_nan());
    assert_eq!(s.events.iter().filter(|&&e| e == "airlimit").count(), 1);
    assert!(s.inserted.is_none());
}
#[test]
fn failed_motion_acquisition_retains_both_window_stores() {
    let mut instance = BodyFlippingSignalInstance::new();
    let mut s = Services::new();
    s.front = true;
    s.air = true;
    s.fail = Some("motion");
    assert_eq!(instance.update(&mut s), Err("motion"));
    assert_eq!(instance.time_in_air.to_bits(), 0.1_f32.to_bits());
    assert_eq!(instance.body_flip_time.to_bits(), 0.0_f32.to_bits());
}
#[test]
fn time_intent_timer_store_precedes_insert_failure_and_end_does_not_reset() {
    let mut instance = TimeIntentInstance::new();
    let mut s = Services::new();
    s.back = true;
    s.fail = Some("insert");
    let action = ActorGraphIntents::native_name(b"Manual");
    let motion = ActorGraphIntents::native_name(b"ManualEngageTime");
    assert_eq!(instance.update(&action, &motion, &mut s), Err("insert"));
    assert_eq!(instance.time.to_bits(), 0.1_f32.to_bits());
    s.fail = None;
    instance.end(&motion, &mut s).unwrap();
    assert_eq!(instance.time.to_bits(), 0.1_f32.to_bits());
    s.back = false;
    s.fail = Some("eraseback");
    assert_eq!(instance.update(&action, &motion, &mut s), Err("eraseback"));
    assert_eq!(instance.time.to_bits(), 0.0_f32.to_bits());
}
