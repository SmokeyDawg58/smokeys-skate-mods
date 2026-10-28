use super::*;
use crate::skeleton_root_transforms::SkeletonRootVector;
#[test]
fn assertion_ignores_w_and_accepts_nonmatching_xyz_and_infinity() {
    let columns = [SkeletonRootVector {
        xyz: [1.0, 2.0, f32::INFINITY],
        w: None,
    }; 4];
    assert!(validate_anchor(SkeletonRootMatrix::from_columns(columns)).is_ok());
    let columns = [SkeletonRootVector::known([1.0, 2.0, 3.0, f32::NAN]); 4];
    assert!(validate_anchor(SkeletonRootMatrix::from_columns(columns)).is_ok());
}
#[test]
fn position_assertions_precede_basis_assertions() {
    let mut columns = [ZERO; 4];
    columns[0].xyz[0] = f32::NAN;
    columns[3].xyz[2] = f32::NAN;
    assert!(matches!(
        validate_anchor(SkeletonRootMatrix::from_columns(columns)),
        Err(RigSubjectMatrixError::MatrixNaN { column: 3, lane: 2 })
    ));
}
