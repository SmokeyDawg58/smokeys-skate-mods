use super::*;

#[test]
fn same_entity_admission_reads_other_then_this_and_null_reads_neither() {
    let mut reads = Vec::new();
    assert!(
        same_entity(
            true,
            |other| {
                reads.push(other);
                Ok::<_, ()>(7)
            },
            |a, b| a == b
        )
        .unwrap()
    );
    assert_eq!(reads, [true, false]);
    assert!(!same_entity(false, |_| -> Result<u32, ()> { panic!() }, |a, b| a == b).unwrap());
    let mut reads = Vec::new();
    assert_eq!(
        same_entity(
            true,
            |other| {
                reads.push(other);
                if other { Ok(7) } else { Err("this") }
            },
            |a, b| a == b
        ),
        Err("this")
    );
    assert_eq!(reads, [true, false]);
}

#[test]
fn admitted_winners_keep_first_ties_and_unordered_then_reset_only_contact_flags() {
    let mut state = SkateboardContactSurface::new();
    assert_eq!(state.overall(), None);
    state.select(0, 0.5, 2 << 7).unwrap();
    state.select(0, 0.5, 3 << 7).unwrap();
    state.select(0, f32::NAN, 4 << 7).unwrap();
    assert_eq!(state.parts()[0].winner.unwrap().physics_type, 2);
    state.select(0, 0.75, (5 << 7) | 0xf07f).unwrap();
    assert_eq!(state.parts()[0].winner.unwrap().physics_type, 5);
    state.select(1, f32::NAN, 4 << 7).unwrap();
    state.select(1, 1.0, 3 << 7).unwrap();
    assert_eq!(state.parts()[1].winner.unwrap().physics_type, 4);
    state.begin_selection();
    assert!(!state.parts()[0].in_contact);
    assert_eq!(state.parts()[0].winner.unwrap().physics_type, 5);
    assert_eq!(state.vote().unwrap(), 1);
    state.select(1, -1.0, 2 << 7).unwrap();
    assert_eq!(state.parts()[1].winner.unwrap().physics_type, 2);
}

#[test]
fn vote_uses_four_wheels_only_lower_ties_and_source_candidate_range() {
    let mut state = SkateboardContactSurface::new();
    for (part, kind) in [(0, 4), (1, 2), (2, 4), (3, 2), (4, 9), (5, 9), (6, 9)] {
        state.select(part, 1.0, kind << 7).unwrap();
    }
    assert_eq!(state.vote().unwrap(), 2);
    state.begin_selection();
    for (part, kind) in [(0, 0), (1, 10), (2, 11), (3, 8)] {
        state.select(part, 0.0, kind << 7).unwrap();
    }
    assert_eq!(state.vote().unwrap(), 8);
    state.begin_selection();
    state.select(0, 0.0, 31 << 7).unwrap();
    assert_eq!(state.vote(), Err(31));
    assert_eq!(state.select(7, 1.0, 1 << 7), Err(7));
    assert_eq!(state.parts()[0].winner.unwrap().physics_type, 31);
}

#[test]
fn late_forced_surface_failure_keeps_old_overall_and_current_winners() {
    struct Cvar {
        reads: usize,
        value: Result<u32, ()>,
    }
    impl SkateboardContactSurfaceCvars for Cvar {
        type Error = ();
        fn forced_world_surface(&mut self) -> Result<u32, ()> {
            self.reads += 1;
            self.value
        }
    }
    let mut state = SkateboardContactSurface::new();
    let mut cvar = Cvar {
        reads: 0,
        value: Ok(9),
    };
    state.finish::<(), _>(&mut cvar).unwrap();
    assert_eq!(state.overall(), Some(9));
    state.select(0, 1.0, 2 << 7).unwrap();
    cvar.value = Err(());
    assert_eq!(
        state.finish::<(), _>(&mut cvar),
        Err(SkateboardContactSurfaceError::Cvar(()))
    );
    assert_eq!(state.overall(), Some(9));
    assert_eq!(state.parts()[0].winner.unwrap().physics_type, 2);
    state.select(0, 2.0, 31 << 7).unwrap();
    let reads = cvar.reads;
    assert_eq!(
        state.finish::<(), _>(&mut cvar),
        Err(SkateboardContactSurfaceError::InvalidVoteType(31))
    );
    assert_eq!(cvar.reads, reads);
}
