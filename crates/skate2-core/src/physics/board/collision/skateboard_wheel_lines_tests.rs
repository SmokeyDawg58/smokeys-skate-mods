use super::*;

#[derive(Default)]
struct Services {
    events: Vec<String>,
    inputs: Vec<CollisionLineInput>,
    fail: Option<String>,
    reject: Option<usize>,
    hits: [Option<SkateboardWheelLineHit>; 5],
}
impl Services {
    fn event(&mut self, event: String) -> Result<(), &'static str> {
        let failed = self.fail.as_ref() == Some(&event);
        self.events.push(event);
        if failed { Err("injected") } else { Ok(()) }
    }
}
impl SkateboardWheelLineServices for Services {
    type Batch = u32;
    type Error = &'static str;
    fn acquire(&mut self) -> Result<u32, Self::Error> {
        self.event("acquire".into())?;
        Ok(17)
    }
    fn begin(&mut self, batch: &u32) -> Result<(), Self::Error> {
        assert_eq!(*batch, 17);
        self.event("begin".into())
    }
    fn part_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.event(format!("part {part}"))?;
        Ok(SkeletonRootVector {
            xyz: [f32::from(u16::try_from(part).unwrap()), 10.0, 20.0],
            w: None,
        })
    }
    fn add(&mut self, batch: &u32, input: CollisionLineInput) -> Result<bool, Self::Error> {
        assert_eq!(*batch, 17);
        let index = self.inputs.len();
        self.event(format!("add {index}"))?;
        self.inputs.push(input);
        Ok(self.reject != Some(index))
    }
    fn end(&mut self, batch: &u32) -> Result<(), Self::Error> {
        assert_eq!(*batch, 17);
        self.event("end".into())
    }
    fn sync(&mut self, batch: &u32) -> Result<(), Self::Error> {
        assert_eq!(*batch, 17);
        self.event("sync".into())
    }
    fn result(
        &mut self,
        batch: &u32,
        index: u32,
    ) -> Result<Option<SkateboardWheelLineHit>, Self::Error> {
        assert_eq!(*batch, 17);
        self.event(format!("get {index}"))?;
        Ok(self.hits[index as usize].take())
    }
    fn release(&mut self, batch: &u32) -> Result<(), Self::Error> {
        assert_eq!(*batch, 17);
        self.event("release".into())
    }
}
fn hit(parameter: f32) -> SkateboardWheelLineHit {
    SkateboardWheelLineHit {
        position: SkeletonRootVector::known([4.0, 5.0, 6.0, 7.0]),
        normal: SkeletonRootVector::known([1.0, 0.0, 0.0, 9.0]),
        parameter,
        surface: 0xf123_4567,
    }
}

#[test]
fn start_retains_one_batch_and_ignores_wheel_rejection_but_deck_is_world_down() {
    let mut state = SkateboardWheelLines::new();
    let mut services = Services {
        reject: Some(1),
        ..Default::default()
    };
    state
        .try_start(
            SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0]),
            &mut services,
        )
        .unwrap();
    assert!(state.has_batch());
    assert_eq!(
        services.events,
        [
            "acquire", "begin", "part 0", "add 0", "part 1", "add 1", "part 2", "add 2", "part 3",
            "add 3", "part 6", "add 4", "end"
        ]
    );
    for input in &services.inputs[..4] {
        assert_eq!(input.fatness.to_bits(), 0);
        assert_eq!(
            input.end.xyz.map(f32::to_bits),
            [input.start.xyz[0] - 0.2, 9.6, 19.4].map(f32::to_bits)
        );
        assert_eq!(input.end.w, None);
    }
    assert_eq!(
        services.inputs[4].end.xyz.map(f32::to_bits),
        [6.0_f32, -90.0, 20.0].map(f32::to_bits)
    );
    assert_eq!(services.inputs[4].fatness.to_bits(), 0x3e19_999a);
    assert_eq!(state.deck.start, Some(services.inputs[4].start));
    assert_eq!(state.deck.valid, Some(false));
    assert!(state.normals.iter().all(Option::is_none));
    assert_eq!(
        state.try_start(UP, &mut services),
        Err(SkateboardWheelLineError::BatchAlreadyRetained)
    );
}

#[test]
fn begin_failure_and_deck_rejection_retain_acquisition_without_end_or_release() {
    for failure in [true, false] {
        let mut state = SkateboardWheelLines::new();
        let mut services = Services {
            fail: failure.then(|| "begin".into()),
            reject: (!failure).then_some(4),
            ..Default::default()
        };
        assert!(state.try_start(UP, &mut services).is_err());
        assert!(state.has_batch());
        assert!(
            !services
                .events
                .iter()
                .any(|event| event == "end" || event == "release")
        );
        assert_eq!(state.deck.valid, if failure { None } else { Some(false) });
    }
}

#[test]
fn sync_and_result_failures_expose_only_the_source_output_prefix() {
    for failure in ["sync", "get 1", "release"] {
        let mut state = SkateboardWheelLines::new();
        let mut services = Services::default();
        state.try_start(UP, &mut services).unwrap();
        services.fail = Some(failure.into());
        assert!(state.try_end(&mut services).is_err());
        assert!(state.has_batch());
        if failure == "sync" {
            assert_eq!(state.height, None);
            assert_eq!(state.normals, [None; 4]);
        } else if failure == "get 1" {
            assert_eq!(state.distances, [Some(0.0), Some(0.0), None, None]);
            assert_eq!(state.normals, [Some(UP), Some(UP), None, None]);
        } else {
            assert_eq!(state.distances, [Some(0.0); 4]);
            assert_eq!(state.deck.valid, Some(false));
        }
    }
}

#[test]
fn real_result_defaults_height_nan_and_deck_miss_preserve_previous_hit_fields() {
    let mut state = SkateboardWheelLines::new();
    let mut services = Services::default();
    state.try_start(UP, &mut services).unwrap();
    services.hits[0] = Some(hit(0.5));
    services.hits[2] = Some(hit(f32::NAN));
    services.hits[4] = Some(hit(0.1));
    state.try_end(&mut services).unwrap();
    assert!(!state.has_batch());
    assert!(state.height.unwrap().is_nan());
    assert_eq!(state.distances[0], Some(0.1));
    assert_eq!(state.normals[1], Some(UP));
    assert_eq!(state.distances[1], Some(0.0));
    assert_eq!(state.deck.valid, Some(true));
    assert_eq!(state.deck.surface, Some(0xf123_4567));
    let prior = (state.deck.position, state.deck.normal, state.deck.surface);
    services.inputs.clear();
    state.try_start(UP, &mut services).unwrap();
    services.hits[0] = Some(hit(-0.0));
    services.hits[1] = Some(hit(0.0));
    state.try_end(&mut services).unwrap();
    assert_eq!(state.height.unwrap().to_bits(), 0x8000_0000);
    assert_eq!(state.deck.valid, Some(false));
    assert_eq!(
        (state.deck.position, state.deck.normal, state.deck.surface),
        prior
    );
    assert_eq!(
        state.try_end(&mut services),
        Err(SkateboardWheelLineError::NoBatch)
    );
}
