#![expect(
    clippy::float_cmp,
    reason = "exact retained submissions and source sentinels"
)]
use super::*;
use crate::{
    input_cvar_runtime::InputCvarManager,
    physical_player_assembly::tests::issued_owner_for_grind_tests,
    physical_player_trajectory_owners::{OnboardTrajectoryOwner, TrajectoryOwnerBindings},
    reckoning_numeric::HostReckoningNumeric,
};

fn owner(count: i16) -> Arc<PhysicalPlayerAssemblyOwner> {
    let (_, owner) = issued_owner_for_grind_tests();
    let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
    let mut selector =
        OnboardTrajectoryOwner::new(TrajectoryOwnerBindings::new(&owner, processed), false);
    selector.num_trajectories = count;
    selector.start_trajectory_position = Vector::known([10.0, 20.0, 30.0, 4.0]);
    selector.launch.time_delta = 0.5;
    for (index, candidate) in selector.data.iter_mut().enumerate() {
        candidate.velocity = Vector {
            xyz: [f32::from(u8::try_from(index).unwrap()) + 1.0, 2.0, 3.0],
            w: None,
        };
    }
    owner.trajectories().lock().unwrap().onboard = Some(selector);
    owner
}
struct Source {
    owner: Arc<PhysicalPlayerAssemblyOwner>,
    calls: Vec<&'static str>,
    submissions: Vec<(usize, BallisticTrajectoryInput, bool)>,
    fail: Option<&'static str>,
    fail_slot: Option<usize>,
    ai: bool,
    ai_trajectory: AiTrajectory,
    debug: bool,
}
impl Source {
    fn new(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Self {
        Self {
            owner: Arc::clone(owner),
            calls: Vec::new(),
            submissions: Vec::new(),
            fail: None,
            fail_slot: None,
            ai: false,
            ai_trajectory: AiTrajectory {
                position: [0.0; 4],
                velocity: [0.0; 4],
                gravity: [0.0; 4],
                offset: [0.0; 4],
                flags: 0,
            },
            debug: false,
        }
    }
    fn call(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl ContinueSource<HostReckoningNumeric> for Source {
    type Error = &'static str;
    type Layout = (f32, f32);
    type Pointer = f32;
    fn maximum_time(
        &mut self,
        velocity: Vector,
        _: &mut HostReckoningNumeric,
    ) -> Result<f32, Self::Error> {
        self.call("maximum")?;
        assert!(self.owner.trajectories().try_lock().is_ok());
        assert_eq!(velocity.xyz, [1.0, 2.0, 3.0]);
        Ok(0.75)
    }
    fn ai_active(&mut self) -> Result<bool, Self::Error> {
        self.call("ai")?;
        Ok(self.ai)
    }
    fn ai_trajectory(&mut self) -> Result<AiTrajectory, Self::Error> {
        self.call("ai trajectory")?;
        Ok(self.ai_trajectory)
    }
    fn numeric(&self, _: crate::reckoning_numeric::ReckoningHostNumericError) -> Self::Error {
        "numeric"
    }
    fn capture_trajectory(&mut self) -> Result<Self::Layout, Self::Error> {
        self.call("layout")?;
        Ok((0.1, 0.2))
    }
    fn show_trajectories(&mut self) -> Result<bool, Self::Error> {
        self.call("show")?;
        Ok(self.submissions.len() % 2 == 0)
    }
    fn radius_pointer(&mut self) -> Result<Self::Pointer, Self::Error> {
        self.call("radius_lookup")?;
        Ok(0.3)
    }
    fn error_end(&mut self, layout: &Self::Layout) -> Result<f32, Self::Error> {
        self.call("error_end")?;
        Ok(layout.1)
    }
    fn radius_value(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        self.call("radius_value")?;
        Ok(*pointer)
    }
    fn error_start(&mut self, layout: &Self::Layout) -> Result<f32, Self::Error> {
        self.call("error_start")?;
        Ok(layout.0)
    }
    fn gravity(&self) -> Vector {
        Vector::known([0.0, -9.8, 0.0, 0.0])
    }
    fn submit(
        &mut self,
        slot: usize,
        input: BallisticTrajectoryInput,
        draw: bool,
        _: &mut HostReckoningNumeric,
    ) -> Result<(), Self::Error> {
        self.call("submit")?;
        // The actual queue implementation takes this exact lock to reset its
        // future; detect retention here without a blocking/reentrant lock call.
        let trajectories = self
            .owner
            .trajectories()
            .try_lock()
            .expect("selector lock released before actual queue");
        assert!(!trajectories.onboard.as_ref().unwrap().just_started_query);
        if self.fail_slot == Some(slot) {
            return Err("queue full");
        }
        self.submissions.push((slot, input, draw));
        Ok(())
    }
    fn debug_draw(&mut self) -> Result<bool, Self::Error> {
        assert!(
            self.owner
                .trajectories()
                .lock()
                .unwrap()
                .onboard
                .as_ref()
                .unwrap()
                .just_started_query
        );
        self.call("debug")?;
        Ok(self.debug)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.call("overlay")
    }
}

#[test]
fn seven_actual_slots_submit_with_source_read_order_and_released_selector_lock() {
    let owner = owner(7);
    let mut source = Source::new(&owner);
    continue_launch(&owner, &mut source, &mut HostReckoningNumeric).unwrap();
    assert_eq!(&source.calls[..2], &["maximum", "ai"]);
    for iteration in 0..7 {
        assert_eq!(
            &source.calls[2 + iteration * 7..9 + iteration * 7],
            &[
                "layout",
                "show",
                "radius_lookup",
                "error_end",
                "radius_value",
                "error_start",
                "submit",
            ]
        );
    }
    assert_eq!(source.calls.last(), Some(&"debug"));
    assert_eq!(source.submissions.len(), 7);
    let (_, input, draw) = source.submissions[0];
    assert_eq!(
        input.position,
        Vector {
            xyz: [10.5, 21.0, 31.5],
            w: None
        }
    );
    assert_eq!(input.gravity, Vector::known([0.0, -9.8, 0.0, 0.0]));
    assert_eq!(
        (
            input.duration,
            input.radius,
            input.maximum_error_start,
            input.maximum_error_end
        ),
        (0.75, 0.3, 0.1, 0.2)
    );
    assert!(draw);
    assert!(!source.submissions[1].2);
    let trajectories = owner.trajectories().lock().unwrap();
    let selector = trajectories.onboard.as_ref().unwrap();
    assert!(selector.just_started_query);
    assert!(!selector.ready);
    assert!(selector.queries.iter().all(|query| !query.ready));
}

#[test]
fn far_ai_trajectory_keeps_the_ordinary_launch() {
    let owner = owner(7);
    let mut source = Source::new(&owner);
    source.ai = true;
    source.ai_trajectory.position = [100.0, 0.0, 0.0, 0.0];
    continue_launch(&owner, &mut source, &mut HostReckoningNumeric).unwrap();
    assert_eq!(&source.calls[..3], ["maximum", "ai", "ai trajectory"]);
    let selector = owner.trajectories().lock().unwrap();
    assert!(!selector.onboard.as_ref().unwrap().ai_trajectory_following);
}

#[test]
fn useable_ai_trajectory_follows_the_path_in_the_first_query() {
    let owner = owner(7);
    let mut source = Source::new(&owner);
    source.ai = true;
    let (start, delta) = {
        let trajectories = owner.trajectories().lock().unwrap();
        let selector = trajectories.onboard.as_ref().unwrap();
        (selector.start_trajectory_position.xyz, selector.launch.time_delta)
    };
    source.ai_trajectory.position = std::array::from_fn(|lane| {
        if lane < 3 { [1.0f32, 2.0, 3.0][lane].mul_add(delta, start[lane]) } else { 0.0 }
    });
    source.ai_trajectory.velocity = [1.0, 2.0, 3.0, 0.0];
    source.ai_trajectory.flags = AI_VALID_TRAJECTORY;
    continue_launch(&owner, &mut source, &mut HostReckoningNumeric).unwrap();
    assert_eq!(source.submissions.len(), 1);
    assert_eq!(source.submissions[0].0, 0);
    let trajectories = owner.trajectories().lock().unwrap();
    let selector = trajectories.onboard.as_ref().unwrap();
    assert!(selector.ai_trajectory_following);
    assert_eq!(selector.num_trajectories, 1);
    assert!(selector.just_started_query);
}

#[test]
fn later_submission_failure_retains_earlier_submission_without_just_started() {
    let owner = owner(7);
    let mut source = Source::new(&owner);
    source.fail_slot = Some(1);
    assert!(matches!(
        continue_launch(&owner, &mut source, &mut HostReckoningNumeric),
        Err(ContinuationError::Source("queue full"))
    ));
    assert_eq!(source.submissions.len(), 1);
    assert!(!source.calls.contains(&"debug"));
    assert!(
        !owner
            .trajectories()
            .lock()
            .unwrap()
            .onboard
            .as_ref()
            .unwrap()
            .just_started_query
    );
}

#[test]
fn radius_float_failure_occurs_after_error_end_and_before_error_start() {
    let owner = owner(1);
    let mut source = Source::new(&owner);
    source.fail = Some("radius_value");
    assert!(continue_launch(&owner, &mut source, &mut HostReckoningNumeric).is_err());
    assert_eq!(
        source.calls,
        [
            "maximum",
            "ai",
            "layout",
            "show",
            "radius_lookup",
            "error_end",
            "radius_value"
        ]
    );
    assert!(source.submissions.is_empty());
}

#[test]
fn zero_count_debug_tail_fetches_radius_without_float_read() {
    let owner = owner(0);
    let mut source = Source::new(&owner);
    source.debug = true;
    source.fail = Some("radius_value");
    continue_launch(&owner, &mut source, &mut HostReckoningNumeric).unwrap();
    assert_eq!(
        source.calls,
        ["maximum", "ai", "debug", "overlay", "radius_lookup"]
    );
    assert!(source.submissions.is_empty());
}

#[test]
fn post_store_debug_failure_preserves_just_started() {
    let owner = owner(0);
    let mut source = Source::new(&owner);
    source.debug = true;
    source.fail = Some("overlay");
    assert!(continue_launch(&owner, &mut source, &mut HostReckoningNumeric).is_err());
    assert!(
        owner
            .trajectories()
            .lock()
            .unwrap()
            .onboard
            .as_ref()
            .unwrap()
            .just_started_query
    );
    assert_eq!(source.calls, ["maximum", "ai", "debug", "overlay"]);
}

#[test]
fn show_trajectories_uses_actual_linked_live_cvar() {
    let references = InputCvarReferences::new_static_initialized();
    assert!(references.try_show_trajectories().is_err());
    let mut manager = InputCvarManager::new();
    manager.try_link(&references).unwrap();
    assert!(!references.try_show_trajectories().unwrap());
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkShowTrajectories=1\r\n"))
        .unwrap();
    assert!(references.try_show_trajectories().unwrap());
}
