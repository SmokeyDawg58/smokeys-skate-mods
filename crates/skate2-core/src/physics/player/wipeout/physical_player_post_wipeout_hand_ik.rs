//! Child of `skeleton_ik_state`: `DoPostPhysicsHandIK82b56310`.
use super::{
    SkeletonIkState,
    post_wipeout_foot::{FootIkError, FootIkSource, commit, two_bone},
};
use crate::{
    ragdoll::SkeletonPartIndex as Part,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_ik_targets::SkeletonIkTargets,
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};

impl SkeletonIkState {
    /// Reference: 0x82b56310. Caller selects !teleport && air && board-contact,
    /// after any selected foot helper and before mandatory `IK.UpdatePostPhysics`.
    #[expect(
        clippy::type_complexity,
        reason = "preserve separate source and numeric failures"
    )]
    pub(crate) fn try_post_wipeout_hand_ik<
        S: FootIkSource,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        targets: &SkeletonIkTargets,
        deck: &Matrix,
        source: &mut S,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), FootIkError<S::Error, N::Error, R::Error>> {
        hand_entries(
            self,
            targets.parts(),
            targets.parents(),
            deck,
            source,
            |source, part, delta| two_bone(targets.data(), part, delta, source, numeric, rounding),
        )
    }
}

/// The production caller graph is separate from the existing two-bone solver
/// so tests can distinguish solver-false, solver failure and late store failure.
fn hand_entries<S: FootIkSource, N, R>(
    state: &mut SkeletonIkState,
    parts: &[Option<Part>; 4],
    parents: &[Option<Part>; 4],
    deck: &Matrix,
    source: &mut S,
    mut solve: impl FnMut(&mut S, Part, Vector) -> Result<bool, FootIkError<S::Error, N, R>>,
) -> Result<(), FootIkError<S::Error, N, R>> {
    use FootIkError as E;
    for entry in 2..4 {
        if state.parts_on_deck[entry] == 0 {
            continue;
        }
        let target = deck.compose(&state.part_to_deck[entry]);
        let part = parts[entry].ok_or(E::Missing("post hand IK primary"))?;
        // Native record P load5641c precedes target stores56438..44.
        let old_position = source.record(part).map_err(E::Source)?.columns()[3];
        // Native stores K,I,J,P with no fallible operation between them.
        for column in [2, 0, 1, 3] {
            state.part_to_world[entry].set_column(column, target.columns()[column]);
        }
        // This is the optional IK parent sentinel, not SkeletonData genealogy.
        // Preserve its assertion AFTER target publication (56448..64).
        if parents[entry].is_some() {
            return Err(E::Missing("post hand IK requires absent parent (-1)"));
        }
        let delta = target.columns()[3].zip(old_position, |new, old| new - old);
        if solve(source, part, delta)? {
            commit(source, part, &state.part_to_world[entry])?;
        }
        // Reached on solver-false too, but not skipped entries or failures.
        if source.draw_post_phys_ik().map_err(E::Source)? {
            source.require_overlay().map_err(E::Source)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "exact target and retained translation stores"
)]
mod tests {
    use super::*;

    struct Source {
        matrix: Matrix,
        events: Vec<(&'static str, Option<Part>)>,
        fail_read: bool,
        fail_store: bool,
        draw: bool,
    }
    impl FootIkSource for Source {
        type Error = &'static str;
        fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
            panic!("hand caller does not read flags")
        }
        fn state_timer(&mut self) -> Result<f32, Self::Error> {
            panic!("hand caller does not read timer")
        }
        fn minimum_deck_up_y(&mut self) -> Result<f32, Self::Error> {
            panic!("hand caller does not read deck angle")
        }
        fn deck_bounds_margin(&mut self) -> Result<Vector, Self::Error> {
            panic!("hand caller does not read bounds")
        }
        fn feet_relative_height(&mut self, _: bool) -> Result<f32, Self::Error> {
            panic!("hand caller does not read feet height")
        }
        fn record(&mut self, part: Part) -> Result<Matrix, Self::Error> {
            self.events.push(("read", Some(part)));
            if self.fail_read {
                Err("read")
            } else {
                Ok(self.matrix)
            }
        }
        fn set_part(&mut self, part: Part, _: &Matrix) -> Result<(), Self::Error> {
            self.events.push(("part", Some(part)));
            Ok(())
        }
        fn set_record(&mut self, part: Part, _: &Matrix) -> Result<(), Self::Error> {
            self.events.push(("record", Some(part)));
            if self.fail_store {
                Err("record store")
            } else {
                Ok(())
            }
        }
        fn draw_post_feet_ik(&mut self) -> Result<bool, Self::Error> {
            panic!("solver owns feet debug")
        }
        fn draw_post_phys_ik(&mut self) -> Result<bool, Self::Error> {
            self.events.push(("draw", None));
            Ok(self.draw)
        }
        fn require_overlay(&mut self) -> Result<(), Self::Error> {
            self.events.push(("overlay", None));
            Ok(())
        }
    }
    fn fixture() -> (SkeletonIkState, Matrix, Source) {
        let mut targets = SkeletonIkTargets::before_skeleton_data_init();
        let state = SkeletonIkState::new(&mut targets);
        let deck = Matrix::known(super::super::IDENTITY);
        let source = Source {
            matrix: deck,
            events: Vec::new(),
            fail_read: false,
            fail_store: false,
            draw: false,
        };
        (state, deck, source)
    }
    const PARTS: [Option<Part>; 4] = [None, None, Some(Part::LeftHand), Some(Part::RightHand)];

    #[test]
    fn skipped_hands_do_not_require_mapping_records_or_debug() {
        let (mut state, deck, mut source) = fixture();
        state.parts_on_deck = [1, 1, 0, 0];
        let result: Result<(), FootIkError<_, (), ()>> = hand_entries(
            &mut state,
            &[None; 4],
            &[None; 4],
            &deck,
            &mut source,
            |_, _, _| panic!("skipped solver"),
        );
        result.unwrap();
        assert!(source.events.is_empty());
    }

    #[test]
    fn read_failure_preserves_target_but_parent_assertion_keeps_published_target() {
        let (mut state, deck, mut source) = fixture();
        state.parts_on_deck[2] = 1;
        let mut columns = *deck.columns();
        columns[3] = Vector::known([2.0, 3.0, 4.0, 0.0]);
        state.part_to_deck[2] = Matrix::from_columns(columns);
        let old = state.part_to_world[2];
        let parents = [None, None, Some(Part::LeftHand), None];
        source.fail_read = true;
        let result: Result<(), FootIkError<_, (), ()>> = hand_entries(
            &mut state,
            &PARTS,
            &parents,
            &deck,
            &mut source,
            |_, _, _| panic!("must fail before solve"),
        );
        assert!(matches!(result, Err(FootIkError::Source("read"))));
        assert_eq!(state.part_to_world[2], old);
        source.fail_read = false;
        let result: Result<(), FootIkError<_, (), ()>> = hand_entries(
            &mut state,
            &PARTS,
            &parents,
            &deck,
            &mut source,
            |_, _, _| panic!("must assert before solve"),
        );
        assert!(matches!(result, Err(FootIkError::Missing(_))));
        assert_eq!(state.part_to_world[2].columns()[3].xyz, [2.0, 3.0, 4.0]);
        assert_eq!(source.events, [("read", Some(Part::LeftHand)); 2]);
    }

    #[test]
    fn ascending_primary_hands_and_solver_false_still_draw_with_unknown_delta_w() {
        let (mut state, deck, mut source) = fixture();
        state.parts_on_deck[2] = 9;
        state.parts_on_deck[3] = 1;
        // Neither do_ik nor external-target state gates this helper.
        state.do_ik = false;
        state.external_target_set[2] = true;
        source.draw = true;
        let mut columns = *source.matrix.columns();
        columns[3] = Vector {
            xyz: [1.0, 2.0, 3.0],
            w: None,
        };
        source.matrix = Matrix::from_columns(columns);
        let result: Result<(), FootIkError<_, (), ()>> = hand_entries(
            &mut state,
            &PARTS,
            &[None; 4],
            &deck,
            &mut source,
            |source, part, delta| {
                source.events.push(("solve", Some(part)));
                assert_eq!(delta.xyz, [-1.0, -2.0, -3.0]);
                assert_eq!(delta.w, None);
                Ok(false)
            },
        );
        result.unwrap();
        assert_eq!(
            source.events,
            [
                ("read", Some(Part::LeftHand)),
                ("solve", Some(Part::LeftHand)),
                ("draw", None),
                ("overlay", None),
                ("read", Some(Part::RightHand)),
                ("solve", Some(Part::RightHand)),
                ("draw", None),
                ("overlay", None),
            ]
        );
    }

    #[test]
    fn solver_success_then_record_failure_preserves_part_prefix_and_stops_right_hand() {
        let (mut state, deck, mut source) = fixture();
        state.parts_on_deck[2..4].fill(1);
        source.fail_store = true;
        let result: Result<(), FootIkError<_, (), ()>> = hand_entries(
            &mut state,
            &PARTS,
            &[None; 4],
            &deck,
            &mut source,
            |source, part, _| {
                source.events.push(("solve", Some(part)));
                Ok(true)
            },
        );
        assert!(matches!(result, Err(FootIkError::Source("record store"))));
        assert_eq!(
            source.events,
            [
                ("read", Some(Part::LeftHand)),
                ("solve", Some(Part::LeftHand)),
                ("part", Some(Part::LeftHand)),
                ("record", Some(Part::LeftHand)),
            ]
        );
    }
}
