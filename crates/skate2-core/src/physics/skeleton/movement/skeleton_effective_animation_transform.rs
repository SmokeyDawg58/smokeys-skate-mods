//! `Skeleton::GetEffectiveAnimToWorldTransform82b3ba48` on retained roots.
use super::SkeletonNormalAssembly;
use crate::{
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootMatrix,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SkeletonEffectiveAnimationUnknownMirror;
impl std::fmt::Display for SkeletonEffectiveAnimationUnknownMirror {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("effective Skeleton animation transform requires Processed8cc mirror flag")
    }
}
#[expect(
    clippy::neg_multiply,
    reason = "native vmulfp scales all four lanes by -1; preserve arithmetic instead of a sign-bit operation"
)]
fn mirror_transform(mut transform: SkeletonRootMatrix, mirror: bool) -> SkeletonRootMatrix {
    if mirror {
        let mut columns = *transform.columns();
        for index in [0, 2] {
            columns[index].xyz = columns[index].xyz.map(|lane| lane * -1.0);
            columns[index].w = columns[index].w.map(|lane| lane * -1.0);
        }
        transform = SkeletonRootMatrix::from_columns(columns);
    }
    transform
}
impl SkeletonNormalAssembly {
    /// Caller validates the retained pair and supplies its same Processed owner.
    /// The source tests animation mirroring8cc bit8, independently of board flip.
    pub(crate) fn try_effective_animation_to_world(
        &self,
        processed: &SkeletonProcessedContact,
    ) -> Result<SkeletonRootMatrix, SkeletonEffectiveAnimationUnknownMirror> {
        let transform = *self.root_transforms.animation_to_world();
        let flags = processed
            .physics_state_fields()
            .flags_8cc
            .ok_or(SkeletonEffectiveAnimationUnknownMirror)?;
        Ok(mirror_transform(transform, flags & 8 != 0))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton_root_transforms::SkeletonRootVector;
    #[test]
    fn mirror_scales_all_x_z_lanes_and_preserves_unknown_w_other_columns() {
        let columns = [
            SkeletonRootVector {
                xyz: [1.0, -0.0, 3.0],
                w: Some(7.0),
            },
            SkeletonRootVector::known([4.0; 4]),
            SkeletonRootVector {
                xyz: [-2.0, 0.0, 5.0],
                w: None,
            },
            SkeletonRootVector::known([9.0; 4]),
        ];
        let transform = SkeletonRootMatrix::from_columns(columns);
        assert_eq!(mirror_transform(transform, false), transform);
        let mirrored = mirror_transform(transform, true);
        assert_eq!(
            mirrored.columns()[0].xyz.map(f32::to_bits),
            [-1.0_f32, 0.0, -3.0].map(f32::to_bits)
        );
        assert_eq!(
            mirrored.columns()[0].w.unwrap().to_bits(),
            (-7.0_f32).to_bits()
        );
        assert_eq!(mirrored.columns()[2].w, None);
        assert_eq!(mirrored.columns()[1], columns[1]);
        assert_eq!(mirrored.columns()[3], columns[3]);
    }
}
