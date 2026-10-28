//! Scalar observations of original splat/vector predicates. W is not inspected.
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};

/// `IsBadData82b2b398`: each XYZ lane is splatted then compared with itself.
/// Infinities pass; replacing this with `is_finite` would change source behavior.
pub(super) fn is_bad_data(transform: &SkeletonRootMatrix) -> bool {
    transform
        .columns()
        .iter()
        .any(|column| column.xyz.iter().any(|v| v.is_nan()))
}

/// The caller has already loaded deck velocity, min cvar, then Processed190
/// before260dc. Max cvar26104/108 is loaded only after ordered first comparison.
pub(super) fn try_accelerated<E>(
    current_deck_y: f32,
    minimum_bad_y: f32,
    captured_old_velocity: SkeletonRootVector,
    max_good_y: impl FnOnce() -> Result<f32, E>,
) -> Result<bool, E> {
    if current_deck_y > minimum_bad_y {
        Ok(max_good_y()? > captured_old_velocity.xyz[1])
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::{is_bad_data, try_accelerated};
    use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};

    #[test]
    fn bad_data_checks_every_xyz_but_neither_infinity_nor_packed_w() {
        let column = SkeletonRootVector {
            xyz: [f32::INFINITY, 0.0, -0.0],
            w: Some(f32::NAN),
        };
        assert!(!is_bad_data(&SkeletonRootMatrix::from_columns([column; 4])));
        for part in 0..4 {
            for lane in 0..3 {
                let mut columns = [column; 4];
                columns[part].xyz[lane] = f32::NAN;
                assert!(is_bad_data(&SkeletonRootMatrix::from_columns(columns)));
            }
        }
    }

    #[test]
    fn first_unordered_or_equal_skips_late_cvar() {
        let old = SkeletonRootVector {
            xyz: [0.0, 0.0, 0.0],
            w: None,
        };
        for (current, minimum) in [(f32::NAN, 25.0), (26.0, f32::NAN), (25.0, 25.0)] {
            assert!(
                !try_accelerated::<()>(current, minimum, old, || panic!(
                    "late cvar must be skipped"
                ))
                .unwrap()
            );
        }
    }

    #[test]
    fn second_predicate_is_ordered_strict_and_propagates_late_failure() {
        for (old_y, maximum, expected) in [
            (0.0, 5.0, true),
            (5.0, 5.0, false),
            (f32::NAN, 5.0, false),
            (0.0, f32::NAN, false),
        ] {
            let old = SkeletonRootVector {
                xyz: [0.0, old_y, 0.0],
                w: None,
            };
            assert_eq!(
                try_accelerated::<()>(26.0, 25.0, old, || Ok(maximum)).unwrap(),
                expected
            );
            assert_eq!(
                try_accelerated(26.0, 25.0, old, || Err("late failure")),
                Err("late failure")
            );
        }
    }
}
