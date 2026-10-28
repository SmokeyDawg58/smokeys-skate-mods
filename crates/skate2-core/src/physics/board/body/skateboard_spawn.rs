//! Original live board relocation, `SkateboardBody::SetTransform` 0x82b34a90.
//! The source caller supplies current cvar access and assertion handling;
//! this module does not hardcode the default or invent a spawn location.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "spawn awaits live player construction")
)]

use crate::skateboard_spawn_math::{
    SkateboardSpawnMathNumeric, skateboard_spawn_is_orthogonal, skateboard_spawn_normalize,
};

pub(crate) trait SkateboardSpawnServices {
    fn current_orthogonal_tolerance(&mut self) -> f32;
    /// Source `FailAssert` + `DebugBreak`. If the host handler returns, execution
    /// continues as after a resumed native assertion; panic may stop it.
    fn orthogonality_assertion_failed(&mut self);
}

/// Fallible live target resolution; unlinked cvars stop before Part access.
pub(crate) trait FallibleSkateboardSpawnServices {
    fn try_current_orthogonal_tolerance(
        &mut self,
    ) -> Result<f32, crate::input_cvar_runtime::InputCvarError>;
    fn orthogonality_assertion_failed(&mut self);
}
// Compatibility for existing controlled corpus services. Production implements
// the fallible interface directly and never substitutes an unlinked default.
impl<T: SkateboardSpawnServices> FallibleSkateboardSpawnServices for T {
    fn try_current_orthogonal_tolerance(
        &mut self,
    ) -> Result<f32, crate::input_cvar_runtime::InputCvarError> {
        Ok(self.current_orthogonal_tolerance())
    }
    fn orthogonality_assertion_failed(&mut self) {
        SkateboardSpawnServices::orthogonality_assertion_failed(self);
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkateboardSpawnError<P> {
    Cvar(crate::input_cvar_runtime::InputCvarError),
    Parts(P),
}
impl<P: std::fmt::Display> std::fmt::Display for SkateboardSpawnError<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => write!(f, "spawn tolerance cvar: {error:?}"),
            Self::Parts(error) => write!(f, "spawn Part: {error}"),
        }
    }
}

/// Ordered live operations, not a snapshot/commit transaction. The concrete
/// implementation keeps every body capability inside its owning session.
pub(crate) trait SkateboardSpawnParts {
    type Error;
    fn live_board_part(&mut self, part: usize) -> Result<[f32; 16], Self::Error>;
    fn set_board_part(&mut self, part: usize, matrix: [f32; 16]) -> Result<(), Self::Error>;
    fn set_hook_part(&mut self, matrix: [f32; 16]) -> Result<(), Self::Error>;
}

/// Source read/write chronology. Host errors preserve the completed prefix.
/// Hook membership failure is detected at the final hook operation, after
/// board writes; safely returning an error replaces native null dereference.
pub(crate) fn try_set_skateboard_transform<P: SkateboardSpawnParts>(
    parts: &mut P,
    requested: [f32; 16],
    services: &mut impl FallibleSkateboardSpawnServices,
    numeric: &mut impl SkateboardSpawnMathNumeric,
) -> Result<(), SkateboardSpawnError<P::Error>> {
    struct KnownParts<'a, P>(&'a mut P);
    impl<P: SkateboardSpawnParts> TypedSkateboardSpawnParts for KnownParts<'_, P> {
        type Error = P::Error;
        fn live_board_part(
            &mut self,
            part: usize,
        ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, Self::Error> {
            self.0
                .live_board_part(part)
                .map(crate::skeleton_root_transforms::SkeletonRootMatrix::known)
        }
        fn set_board_part(
            &mut self,
            part: usize,
            matrix: crate::skeleton_root_transforms::SkeletonRootMatrix,
        ) -> Result<(), Self::Error> {
            self.0.set_board_part(
                part,
                matrix
                    .known_lanes()
                    .expect("known source matrices preserve all lanes"),
            )
        }
        fn set_hook_part(
            &mut self,
            matrix: crate::skeleton_root_transforms::SkeletonRootMatrix,
        ) -> Result<(), Self::Error> {
            self.0
                .set_hook_part(matrix.known_lanes().expect("known requested matrix"))
        }
    }
    try_set_skateboard_transform_typed(
        &mut KnownParts(parts),
        crate::skeleton_root_transforms::SkeletonRootMatrix::known(requested),
        services,
        numeric,
    )
}

pub(crate) trait TypedSkateboardSpawnParts {
    type Error;
    fn live_board_part(
        &mut self,
        part: usize,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, Self::Error>;
    fn set_board_part(
        &mut self,
        part: usize,
        matrix: crate::skeleton_root_transforms::SkeletonRootMatrix,
    ) -> Result<(), Self::Error>;
    fn set_hook_part(
        &mut self,
        matrix: crate::skeleton_root_transforms::SkeletonRootMatrix,
    ) -> Result<(), Self::Error>;
}

/// Same SetTransform chronology, retaining unknown physical-body W lanes.
pub(crate) fn try_set_skateboard_transform_typed<P: TypedSkateboardSpawnParts>(
    parts: &mut P,
    requested: crate::skeleton_root_transforms::SkeletonRootMatrix,
    services: &mut impl FallibleSkateboardSpawnServices,
    numeric: &mut impl SkateboardSpawnMathNumeric,
) -> Result<(), SkateboardSpawnError<P::Error>> {
    let tolerance = services
        .try_current_orthogonal_tolerance()
        .map_err(SkateboardSpawnError::Cvar)?;
    if !skateboard_spawn_is_orthogonal(&requested.xyz_scratch(), tolerance, numeric) {
        services.orthogonality_assertion_failed();
    }
    let old_deck = parts
        .live_board_part(6)
        .map_err(SkateboardSpawnError::Parts)?;
    let inverse_deck = old_deck.inverse();
    let normalized = normalize_typed(requested, numeric);
    let displacement = normalized.compose(&inverse_deck);
    parts
        .set_board_part(6, normalized)
        .map_err(SkateboardSpawnError::Parts)?;
    for part in 0..6 {
        let live = parts
            .live_board_part(part)
            .map_err(SkateboardSpawnError::Parts)?;
        let moved = displacement.compose(&live);
        let normalized = normalize_typed(moved, numeric);
        parts
            .set_board_part(part, normalized)
            .map_err(SkateboardSpawnError::Parts)?;
    }
    parts
        .set_hook_part(requested)
        .map_err(SkateboardSpawnError::Parts)
}

fn normalize_typed(
    input: crate::skeleton_root_transforms::SkeletonRootMatrix,
    numeric: &mut impl SkateboardSpawnMathNumeric,
) -> crate::skeleton_root_transforms::SkeletonRootMatrix {
    use crate::skeleton_root_transforms::SkeletonRootMatrix;
    let mut output =
        SkeletonRootMatrix::known(skateboard_spawn_normalize(&input.xyz_scratch(), numeric));
    // XYZ reductions never consume W. Each earlier axis subtracts every later
    // axis, so unknown W propagates even when a projection happens to be zero.
    for axis in 0..3 {
        if input.columns()[axis..3].iter().any(|v| v.w.is_none()) {
            let mut column = output.columns()[axis];
            column.w = None;
            output.set_column(axis, column);
        }
    }
    output.set_column(3, input.columns()[3]);
    output
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact representable source-order discriminators.
mod tests {
    use super::*;
    use crate::{
        skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
        skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    };

    struct Parts {
        board: [[f32; 16]; 7],
        hook: Option<[f32; 16]>,
        calls: Vec<(bool, usize)>,
        fail: Option<usize>,
    }
    impl Parts {
        fn new() -> Self {
            let mut board = [FRESH_VOLUME_TRANSFORM; 7];
            board[6][12] = 4.0;
            board[0][12] = 5.0;
            Self {
                board,
                hook: None,
                calls: Vec::new(),
                fail: None,
            }
        }
    }
    impl SkateboardSpawnParts for Parts {
        type Error = usize;
        fn live_board_part(&mut self, part: usize) -> Result<[f32; 16], usize> {
            self.calls.push((false, part));
            Ok(self.board[part])
        }
        fn set_board_part(&mut self, part: usize, matrix: [f32; 16]) -> Result<(), usize> {
            self.calls.push((true, part));
            if self.fail == Some(part) {
                return Err(part);
            }
            self.board[part] = matrix;
            // Deliberately expose a prior-write effect to the next live getter.
            if part == 0 {
                self.board[1][12] = 7.0;
            }
            Ok(())
        }
        fn set_hook_part(&mut self, matrix: [f32; 16]) -> Result<(), usize> {
            self.calls.push((true, 7));
            if self.fail == Some(7) {
                return Err(7);
            }
            self.hook = Some(matrix);
            Ok(())
        }
    }
    struct Services {
        reads: usize,
        assertions: usize,
        tolerance: f32,
    }
    impl SkateboardSpawnServices for Services {
        fn current_orthogonal_tolerance(&mut self) -> f32 {
            self.reads += 1;
            self.tolerance
        }
        fn orthogonality_assertion_failed(&mut self) {
            self.assertions += 1;
        }
    }
    fn services() -> Services {
        Services {
            reads: 0,
            assertions: 0,
            tolerance: 0.01,
        }
    }

    #[test]
    fn typed_normalization_retains_unknown_w_without_changing_xyz_or_numeric_calls() {
        use crate::skeleton_root_transforms::SkeletonRootMatrix as M;
        let mut input = FRESH_VOLUME_TRANSFORM;
        input[0] = 1.01;
        input[6] = 0.125;
        input[12] = 17.0;
        for unknown_axis in 0..4 {
            let mut typed = M::known(input);
            let mut column = typed.columns()[unknown_axis];
            column.w = None;
            typed.set_column(unknown_axis, column);
            let mut known_numeric = CountingNumeric::default();
            let mut typed_numeric = CountingNumeric::default();
            let known = skateboard_spawn_normalize(&input, &mut known_numeric);
            let actual = normalize_typed(typed, &mut typed_numeric);
            assert_eq!(known_numeric.estimates, typed_numeric.estimates);
            for axis in 0..4 {
                assert_eq!(
                    actual.columns()[axis].xyz.map(f32::to_bits),
                    std::array::from_fn(|i| known[axis * 4 + i].to_bits())
                );
                let unknown = if axis == 3 {
                    unknown_axis == 3
                } else {
                    unknown_axis < 3 && axis <= unknown_axis
                };
                assert_eq!(actual.columns()[axis].w.is_none(), unknown);
            }
        }
    }

    #[derive(Default)]
    struct CountingNumeric {
        estimates: usize,
    }
    impl SkateboardSpawnMathNumeric for CountingNumeric {
        fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> f32 {
            HostSkateboardSpawnMathNumeric.dot3(left, right)
        }
        fn dot4(&mut self, left: [f32; 4], right: [f32; 4]) -> f32 {
            HostSkateboardSpawnMathNumeric.dot4(left, right)
        }
        fn reciprocal_sqrt_estimate(&mut self, value: f32) -> f32 {
            self.estimates += 1;
            HostSkateboardSpawnMathNumeric.reciprocal_sqrt_estimate(value)
        }
    }

    #[test]
    fn live_order_retains_pre_deck_displacement_and_original_hook_matrix() {
        let mut parts = Parts::new();
        let mut services = services();
        let mut requested = FRESH_VOLUME_TRANSFORM;
        requested[0] = 1.01;
        requested[12] = 10.0;
        requested[15] = f32::from_bits(0x8000_0000);
        let mut numeric = CountingNumeric::default();
        try_set_skateboard_transform(&mut parts, requested, &mut services, &mut numeric).unwrap();
        assert_eq!(
            numeric.estimates, 21,
            "seven distinct three-axis Math normalizations"
        );
        assert_eq!(
            parts.calls,
            vec![
                (false, 6),
                (true, 6),
                (false, 0),
                (true, 0),
                (false, 1),
                (true, 1),
                (false, 2),
                (true, 2),
                (false, 3),
                (true, 3),
                (false, 4),
                (true, 4),
                (false, 5),
                (true, 5),
                (true, 7)
            ]
        );
        assert_eq!(parts.board[0][12], 11.0);
        assert_eq!(
            parts.board[1][12], 13.0,
            "live read sees preceding setter effect"
        );
        assert_ne!(parts.board[6][0].to_bits(), requested[0].to_bits());
        assert_eq!(
            parts.hook.unwrap().map(f32::to_bits),
            requested.map(f32::to_bits)
        );
        assert_eq!((services.reads, services.assertions), (1, 0));
    }

    #[test]
    fn later_part_failure_keeps_written_prefix_and_never_reaches_hook() {
        let mut parts = Parts::new();
        parts.fail = Some(2);
        let mut requested = FRESH_VOLUME_TRANSFORM;
        requested[12] = 10.0;
        assert_eq!(
            try_set_skateboard_transform(
                &mut parts,
                requested,
                &mut services(),
                &mut HostSkateboardSpawnMathNumeric
            ),
            Err(SkateboardSpawnError::Parts(2))
        );
        assert_eq!(parts.board[6][12], 10.0);
        assert_eq!(parts.board[0][12], 11.0);
        assert_eq!(parts.board[2][12], 0.0);
        assert_eq!(parts.calls.last(), Some(&(true, 2)));
        assert!(parts.hook.is_none());
    }

    #[test]
    fn missing_hook_is_a_late_host_failure_after_seven_board_writes() {
        let mut parts = Parts::new();
        parts.fail = Some(7);
        let mut requested = FRESH_VOLUME_TRANSFORM;
        requested[12] = 10.0;
        assert_eq!(
            try_set_skateboard_transform(
                &mut parts,
                requested,
                &mut services(),
                &mut HostSkateboardSpawnMathNumeric
            ),
            Err(SkateboardSpawnError::Parts(7))
        );
        assert_eq!(parts.calls.iter().filter(|(write, _)| *write).count(), 8);
        assert_eq!(parts.board[5][12], 6.0);
    }

    #[test]
    fn current_tolerance_is_sampled_each_call_and_resumed_assertion_continues() {
        let mut parts = Parts::new();
        let mut services = services();
        let mut requested = FRESH_VOLUME_TRANSFORM;
        requested[0] = 2.0;
        try_set_skateboard_transform(
            &mut parts,
            requested,
            &mut services,
            &mut HostSkateboardSpawnMathNumeric,
        )
        .unwrap();
        assert_eq!(services.assertions, 1);
        services.tolerance = f32::INFINITY;
        try_set_skateboard_transform(
            &mut parts,
            requested,
            &mut services,
            &mut HostSkateboardSpawnMathNumeric,
        )
        .unwrap();
        assert_eq!((services.reads, services.assertions), (2, 1));
        assert_eq!(parts.hook.unwrap(), requested);
    }
}
