//! `Actor::CalcRespawnTransform` 0x825f09a8, using the same retained PhysOut.
//! Its Ground100/104 caller reaches the on-board branch; the native off-board
//! branch is retained here because the method itself tests PhysOut state.
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_phys_out::{
        PhysOutMotionVector, PhysicalPlayerPhysOut, construct::ConcretePhysOutError,
    },
    reckoning_numeric::{ReckoningNumeric, cross_full4, try_normalize_safe},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[derive(Debug)]
pub(crate) enum CalcRespawnTransformError<E> {
    Output(ConcretePhysOutError<core::convert::Infallible>),
    Cvar(InputCvarError),
    Numeric(E),
}

impl<E: std::fmt::Debug> std::fmt::Display for CalcRespawnTransformError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Actor::CalcRespawnTransform: {self:?}")
    }
}

const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);
const UP: SkeletonRootVector = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
const RAISE: SkeletonRootVector = SkeletonRootVector::known([0.0, 0.2, 0.0, 0.0]);
const MIN_HORIZONTAL_SQUARE: f32 = f32::from_bits(0x3f66_6666);

/// Source `vpermwi 0x63`/multiply/`vnmsubfp`/permute graph, equivalent to a
/// full-lane cross for ordinary finite inputs. Unknown intrusive W never
/// becomes a published zero. XYZ and W are independent in this graph.
fn source_cross(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    let l = [left.xyz[0], left.xyz[1], left.xyz[2], left.w.unwrap_or(0.0)];
    let r = [
        right.xyz[0],
        right.xyz[1],
        right.xyz[2],
        right.w.unwrap_or(0.0),
    ];
    let result = cross_full4(l, r);
    SkeletonRootVector {
        xyz: [result[0], result[1], result[2]],
        w: left.w.zip(right.w).map(|_| result[3]),
    }
}

/// No new PhysOut or Actor copy is allocated. The cvar is read from its live
/// linked target after the first dot, as in the source. Numeric callbacks
/// retain the project's explicit Xenon rounding/estimate boundary.
pub(crate) fn try_calc_respawn_transform<N: ReckoningNumeric>(
    output: &PhysicalPlayerPhysOut,
    cvars: &InputCvarReferences,
    numeric: &mut N,
) -> Result<SkeletonRootMatrix, CalcRespawnTransformError<N::Error>> {
    use CalcRespawnTransformError as E;
    if output.try_state_output().map_err(E::Output)?.is_offboard {
        return Ok(output
            .try_skeleton_output()
            .map_err(E::Output)?
            .effective_skeleton_root_matrix);
    }

    // 0x825f0a38..0ab8: GetEffectiveDeckTransform and a full-lane +0.2 Y.
    let mut matrix = output
        .try_camera_board_output()
        .map_err(E::Output)?
        .effective_transform;
    matrix.set_column(3, matrix.columns()[3].zip(RAISE, |a, b| a + b));

    // 0x825f0aec..0b24: raw deck-velocity squared, then linked target value.
    let velocity = output
        .try_camera_motion_vector(PhysOutMotionVector::DeckVelocity)
        .map_err(E::Output)?;
    let square = numeric.dot_vector(velocity, velocity).map_err(E::Numeric)?;
    let insignificant = cvars
        .try_breadcrumb_insignificant_square_velocity()
        .map_err(E::Cvar)?;
    if !(square > insignificant) {
        return Ok(matrix);
    }

    // 0x825f0b40..0bd4: NormalizeSafe(raw velocity, zero), then two
    // unnormalized cross products and the strict horizontal-square gate.
    let direction = try_normalize_safe(velocity, ZERO, numeric).map_err(E::Numeric)?;
    let right = source_cross(UP, direction);
    let forward = source_cross(right, UP);
    let horizontal_square = numeric.dot_vector(forward, forward).map_err(E::Numeric)?;
    if horizontal_square > MIN_HORIZONTAL_SQUARE {
        matrix.set_column(0, right);
        matrix.set_column(1, UP);
        matrix.set_column(2, forward);
    }
    Ok(matrix)
}
