//! Actual `HeadTracking` ctor82dca6f8, called82b573fc before `SkeletonIK`.
use crate::attribute_runtime::{
    AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use crate::reckoning_pid::{PhysicsControllerData, VectorPidFilter};
use crate::skeleton_root_transforms::SkeletonRootVector as Vector;
use std::sync::Arc;
#[path = "../../player/wipeout/physical_player_post_wipeout_head_tracking.rs"]
pub(crate) mod post_wipeout_update;

pub(crate) struct SkeletonHeadTracking {
    pub(crate) attributes: AttributeRuntimeInstance,
    pub(crate) filter: Option<VectorPidFilter>,
    pub(crate) target_position: Option<Vector>,
    pub(crate) rotation: Option<[f32; 4]>,
    pub(crate) rotation_velocity: Option<[f32; 4]>,
    pub(crate) valid_target: Option<bool>,
    attempted: bool,
}
impl SkeletonHeadTracking {
    /// HeadTracking::Reset 0x82dc6fa0, preserving PID/filter and attributes.
    pub(crate) fn reset_after_teleport(&mut self) {
        self.valid_target = Some(false);
        self.rotation = Some([0.0; 4]);
        self.rotation_velocity = Some([0.0; 4]);
        self.target_position = Some(Vector::known([0.0; 4]));
    }
    /// `PhysicsSkater` allocation0xb0 is not cleared; retain unknown fields until
    /// their actual constructor store. Generated wrapper82dca680 class key.
    pub(crate) fn before_constructor(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            attributes: vault.instance(0x58db_c281_5ca6_66fd, 0xd7ed_bd36_2d7d_2152),
            filter: None,
            target_position: None,
            rotation: None,
            rotation_velocity: None,
            valid_target: None,
            attempted: false,
        }
    }
    pub(crate) fn try_construct(&mut self) -> Result<(), HeadTrackingConstructorError> {
        if self.attempted {
            return Err(HeadTrackingConstructorError::AlreadyAttempted);
        }
        self.attempted = true;
        // Generated wrapper installs DefaultDataArea48 only when cached layout
        // is NULL. This is a layout read, not generic GetAttributePointer.
        let layout = self
            .attributes
            .try_cached_layout(0x30)
            .map_err(HeadTrackingConstructorError::Attribute)?;
        let control = layout.as_ref().map_or([0.0; 4], |data| {
            std::array::from_fn(|i| {
                f32::from_be_bytes(
                    data.bytes()[0x20 + i * 4..0x24 + i * 4]
                        .try_into()
                        .expect("checked 48-byte layout"),
                )
            })
        });
        // PIDFilter<Vector3>82e48e50 copies controls and initVal twice, then
        // zeroes three history vectors; HeadTracking passes four-lane zero.
        self.filter = Some(VectorPidFilter::new(
            PhysicsControllerData {
                proportional: control[0],
                integral: control[1],
                derivative: control[2],
                integral_blend: control[3],
            },
            [0.0; 4],
        ));
        self.target_position = Some(Vector::known([0.0; 4]));
        self.rotation = Some([0.0; 4]);
        self.rotation_velocity = Some([0.0; 4]);
        self.valid_target = Some(false);
        Ok(())
    }
}
#[derive(Debug)]
pub(crate) enum HeadTrackingConstructorError {
    AlreadyAttempted,
    Attribute(AttributeRuntimeError),
}
impl std::fmt::Display for HeadTrackingConstructorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => f.write_str("HeadTracking construction already attempted"),
            Self::Attribute(e) => write!(f, "HeadTracking layout: {e}"),
        }
    }
}
