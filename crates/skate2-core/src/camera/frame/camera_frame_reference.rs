//! Same Subject reference traversal, `CalculateReferencePoint82e3c540/c328`.
use super::{CameraFrameComposer, composition::FrameCompositionError as E};
use crate::{
    camera_shot::CameraShot, camera_subject::ZERO,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

impl CameraFrameComposer {
    pub(super) fn reference_point(&self, shot: &CameraShot, fallback: Vector) -> Result<Vector, E> {
        let subject = self.subject.as_ref().ok_or(E::MissingSubject)?;
        let count = subject
            .lock()
            .map_err(|_| E::OwnerLock)?
            .reference_positions
            .len();
        if count > 9 {
            return Err(E::ReferenceCount(count));
        }
        let weights = shot.reference_weights;
        let count = subject
            .lock()
            .map_err(|_| E::OwnerLock)?
            .reference_positions
            .len();
        let actual = subject
            .lock()
            .map_err(|_| E::OwnerLock)?
            .reference_positions
            .len();
        if count != actual || count > weights.len() {
            return Err(E::ReferenceCount(actual));
        }
        weighted_reference(&weights[..count], fallback, |index| {
            let offset = shot.board_offset;
            let up = subject.lock().map_err(|_| E::OwnerLock)?.up;
            let offset = up.map(|lane| lane * offset);
            let reference = *subject
                .lock()
                .map_err(|_| E::OwnerLock)?
                .reference_positions
                .get(index)
                .ok_or(E::ReferenceCount(index))?;
            Ok(reference.zip(offset, |a, b| a + b))
        })
    }
}

/// Zero weights retain their actual point reads and IEEE arithmetic.
#[expect(
    clippy::assign_op_pattern,
    reason = "native total is weight plus old total, in that operand order"
)]
pub(super) fn weighted_reference(
    weights: &[f32],
    fallback: Vector,
    mut point: impl FnMut(usize) -> Result<Vector, E>,
) -> Result<Vector, E> {
    let mut result = ZERO;
    let mut total = 0.0;
    for (index, weight) in weights.iter().copied().enumerate() {
        let adjusted = point(index)?;
        total = weight + total;
        result = adjusted.zip(result, |value, old| value.mul_add(weight, old));
    }
    result = if total.abs() < f32::from_bits(0x38d1_b717) {
        fallback
    } else {
        let inverse = 1.0 / total;
        result.map(|lane| lane * inverse)
    };
    if let Some(lane) = result.xyz.iter().position(|lane| lane.is_nan()) {
        return Err(E::ReferenceNaN(lane));
    }
    Ok(result)
}
