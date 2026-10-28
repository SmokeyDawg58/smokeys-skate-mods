//! `Skeleton::GetPartTransform`82b45e18 reads the plain animation Instance+4.
use super::SkeletonNormalAssembly;
use crate::{
    attribute_runtime::AttributeRuntimeError, skeleton_root_transforms::SkeletonRootMatrix,
};

#[derive(Debug)]
pub(crate) enum SkeletonPartTransformQueryError {
    MissingAttributes,
    Attribute(AttributeRuntimeError),
    PartOutsideRecord { part: usize },
    AnimationUnproduced { part: usize },
}
impl std::fmt::Display for SkeletonPartTransformQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAttributes => {
                f.write_str("Skeleton plain animation instance was not transferred")
            }
            Self::Attribute(error) => write!(f, "Skeleton part-transform attribute: {error:?}"),
            Self::PartOutsideRecord { part } => write!(
                f,
                "Skeleton part {part} is outside the retained physics record"
            ),
            Self::AnimationUnproduced { part } => write!(
                f,
                "Skeleton animation volume for part {part} is unavailable"
            ),
        }
    }
}
impl SkeletonNormalAssembly {
    pub(crate) fn try_query_part_transform(
        &self,
        part: usize,
    ) -> Result<SkeletonRootMatrix, SkeletonPartTransformQueryError> {
        use SkeletonPartTransformQueryError as E;
        const KEY: u64 = 0x1409_323f_fccf_d7ea;
        let attribute = self
            .animation_query_instance
            .as_ref()
            .ok_or(E::MissingAttributes)?
            .try_get_data(KEY, 0)
            .map_err(E::Attribute)?;
        // Native missing pointer -> DefaultDataArea1 -> byte0. The load reads
        // just the first byte; no generated PhysicsAnimation layout is used.
        let byte =
            match attribute {
                None => 0,
                Some(data) => *data.bytes().first().ok_or(E::Attribute(
                    AttributeRuntimeError::PayloadSize {
                        key: KEY,
                        expected: 1,
                        actual: 0,
                    },
                ))?,
            };
        if byte != 0 {
            self.physics_history
                .record()
                .transform_typed(part)
                .ok_or(E::PartOutsideRecord { part })
        } else {
            let volume = self
                .animation_targets
                .volume_row(part)
                .ok_or(E::AnimationUnproduced { part })?;
            Ok(self
                .root_transforms
                .animation_to_world()
                .compose(&SkeletonRootMatrix::known(*volume)))
        }
    }
}
