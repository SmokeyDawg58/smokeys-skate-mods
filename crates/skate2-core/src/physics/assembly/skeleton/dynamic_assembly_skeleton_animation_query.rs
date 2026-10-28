//! Actual `Skeleton::IsSkateboardAnimated82b4cd08` and `Animating82b45b60`.
use super::SkeletonNormalAssembly;
use crate::attribute_runtime::AttributeRuntimeError;

#[derive(Debug)]
pub(crate) enum SkeletonAnimationQueryError {
    UnknownMode,
    UnknownPose,
    MissingAttributes,
    Attribute(AttributeRuntimeError),
}
impl std::fmt::Display for SkeletonAnimationQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownMode => f.write_str("Skeleton physics-animation mode is uninitialized"),
            Self::UnknownPose => f.write_str("Skeleton animation-volume board row is unproduced"),
            Self::MissingAttributes => {
                f.write_str("Skeleton physics-animation attributes unavailable")
            }
            Self::Attribute(e) => write!(f, "Skeleton animation query attribute: {e}"),
        }
    }
}

impl SkeletonNormalAssembly {
    /// Controlled original-rig fixture through the existing source mapping
    /// producer. This does not execute the remaining `ProcessData` adjustments.
    #[cfg(test)]
    pub(crate) fn map_original_pose_for_animation_query(
        &mut self,
        pose: &[crate::animation::AnimationMatrix],
    ) -> Result<(), crate::skeleton_animation_targets::SkeletonAnimationTargetError> {
        self.current_animation.invalidate_mapped_pose();
        self.animation_targets
            .try_map_current_pose(&self.state.binding.body().data, pose)
    }
    /// Reads SAME retained mapped/adjusted animation-volume row and the existing
    /// Skeleton attribute instance. No constructor-pose substitution or copy.
    pub(crate) fn try_is_skateboard_animated(&self) -> Result<bool, SkeletonAnimationQueryError> {
        use SkeletonAnimationQueryError as E;
        let mode = self.physics_animation_state.ok_or(E::UnknownMode)?;
        let animated = query(
            mode,
            || {
                self.animation_targets
                    .volume_row(0)
                    .map(|row| row[9])
                    .ok_or(E::UnknownPose)
            },
            || {
                self.current_animation_attributes
                    .as_ref()
                    .ok_or(E::MissingAttributes)?
                    .skateboard_animation_threshold()
                    .map_err(E::Attribute)
            },
        )?;
        if mode != 1 && std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
            let pitch = self.animation_targets.volume_row(0).map(|row| row[9].abs());
            let threshold = self
                .current_animation_attributes
                .as_ref()
                .and_then(|attributes| attributes.skateboard_animation_threshold().ok());
            eprintln!(
                "known air: skateboard animated mode={mode} board_z_y={pitch:?} threshold={threshold:?} result={animated}"
            );
        }
        Ok(animated)
    }
}

fn query<E>(
    mode: i32,
    row_y: impl FnOnce() -> Result<f32, E>,
    threshold: impl FnOnce() -> Result<f32, E>,
) -> Result<bool, E> {
    if mode == 2 {
        return Ok(true);
    }
    // Source loads mapped board axis-Z Y, clears sign bit, THEN reads Vault.
    let y = row_y()?.abs();
    let minimum = threshold()?;
    if y > minimum {
        Ok(mode != 1)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn forced_physics_still_requires_pose_and_attribute_but_forced_animation_skips_both() {
        assert!(query(2, || Err("pose"), || Err("attribute")).unwrap());
        assert_eq!(query(1, || Err("pose"), || Err("attribute")), Err("pose"));
        assert_eq!(query(1, || Ok(-0.5), || Err("attribute")), Err("attribute"));
        let calls = RefCell::new(Vec::new());
        assert!(
            !query::<()>(
                1,
                || {
                    calls.borrow_mut().push("pose");
                    Ok(-0.5)
                },
                || {
                    calls.borrow_mut().push("attribute");
                    Ok(0.2)
                }
            )
            .unwrap()
        );
        assert_eq!(*calls.borrow(), ["pose", "attribute"]);
    }

    #[test]
    fn strict_comparison_preserves_negative_zero_and_unordered_semantics() {
        for (y, minimum, expected) in [
            (-0.5, 0.2, true),
            (-0.0, 0.0, false),
            (0.5, 0.5, false),
            (f32::NAN, 0.0, false),
            (0.5, f32::NAN, false),
        ] {
            assert_eq!(query::<()>(0, || Ok(y), || Ok(minimum)).unwrap(), expected);
        }
    }
}
