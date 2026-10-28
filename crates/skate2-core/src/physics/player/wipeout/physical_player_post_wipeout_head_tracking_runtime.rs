//! Child of `input_cvar_runtime`; actual retained Vault/PID/cvar binding.
//! Mount here so capture can borrow the existing private target allocations.
use super::{DEFINITIONS, InputCvarError, InputCvarReferences, Target, Value};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    reckoning_numeric::ReckoningNumeric,
    skeleton_head_tracking::{
        SkeletonHeadTracking,
        post_wipeout_update::{self, HeadTrackingError, HeadTrackingSource, HeadTrackingState},
    },
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};

#[derive(Debug)]
pub(crate) enum HeadTrackingLiveError {
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    AttributeSize {
        key: u64,
        bytes: usize,
        required: usize,
    },
}
impl std::fmt::Display for HeadTrackingLiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(e) => write!(f, "head attributes: {e}"),
            Self::Cvar(e) => write!(f, "head cvar: {e}"),
            Self::AttributeSize {
                key,
                bytes,
                required,
            } => write!(
                f,
                "head attribute {key:016x} has {bytes} bytes, requires {required}"
            ),
        }
    }
}
struct Source<'a> {
    attributes: &'a AttributeRuntimeInstance,
    cvars: &'a InputCvarReferences,
}
struct ForceTarget(Target);
struct AngleTarget(Target);

pub(crate) struct HeadOrientationRuntime<'a>(pub(crate) &'a InputCvarReferences);
pub(crate) struct HeadFractionTarget(Target);
impl crate::dynamic_assembly_session::skeleton_normal::post_wipeout_head_orientation::HeadOrientationCvars for HeadOrientationRuntime<'_> {
    type Error = InputCvarError;
    type HeadTarget = HeadFractionTarget;
    fn capture_head_fraction(&mut self) -> Result<Self::HeadTarget, Self::Error> {
        self.0.targets.lock().map_err(|_| InputCvarError::Lock)?[223].clone()
            .map(HeadFractionTarget).ok_or(InputCvarError::Unlinked(DEFINITIONS[223].label))
    }
    fn head_fraction(&mut self, target: &Self::HeadTarget) -> Result<f32, Self::Error> {
        match *target.0.lock().map_err(|_| InputCvarError::Lock)? {
            Value::Float(value) => Ok(value), _ => Err(InputCvarError::WrongType),
        }
    }
    fn spine_fraction(&mut self) -> Result<f32, Self::Error> { self.0.try_current_float(224) }
}
impl InputCvarReferences {
    pub(crate) fn try_update_skeleton_pose_buffer(&self) -> Result<bool, InputCvarError> {
        self.try_current_boolean(222)
    }
}
impl Source<'_> {
    fn capture(&self, index: usize) -> Result<Target, HeadTrackingLiveError> {
        self.cvars
            .targets
            .lock()
            .map_err(|_| HeadTrackingLiveError::Cvar(InputCvarError::Lock))?[index]
            .clone()
            .ok_or(HeadTrackingLiveError::Cvar(InputCvarError::Unlinked(
                DEFINITIONS[index].label,
            )))
    }
    fn attribute<const N: usize>(&self, key: u64) -> Result<[f32; N], HeadTrackingLiveError> {
        let Some(data) = self
            .attributes
            .try_get_data(key, 0)
            .map_err(HeadTrackingLiveError::Attribute)?
        else {
            // Native DefaultDataArea4/16, not an invented absent-field skip.
            return Ok([0.0; N]);
        };
        if data.bytes().len() < N * 4 {
            return Err(HeadTrackingLiveError::AttributeSize {
                key,
                bytes: data.bytes().len(),
                required: N * 4,
            });
        }
        Ok(std::array::from_fn(|i| {
            f32::from_be_bytes(
                data.bytes()[i * 4..i * 4 + 4]
                    .try_into()
                    .expect("checked attribute length"),
            )
        }))
    }
}
impl HeadTrackingSource for Source<'_> {
    type Error = HeadTrackingLiveError;
    type ForceTarget = ForceTarget;
    type AngleTarget = AngleTarget;
    fn controller_layout(&mut self) -> Result<[f32; 4], Self::Error> {
        let layout = self
            .attributes
            .try_cached_layout(0x30)
            .map_err(HeadTrackingLiveError::Attribute)?;
        // Constructor installs the genuine DefaultDataArea48 for null layout.
        Ok(layout.as_ref().map_or([0.0; 4], |data| {
            std::array::from_fn(|i| {
                f32::from_be_bytes(
                    data.bytes()[0x20 + i * 4..0x24 + i * 4]
                        .try_into()
                        .expect("checked layout length"),
                )
            })
        }))
    }
    fn capture_force_look_at(&mut self) -> Result<Self::ForceTarget, Self::Error> {
        self.capture(215).map(ForceTarget)
    }
    fn capture_maximum_target_angle(&mut self) -> Result<Self::AngleTarget, Self::Error> {
        self.capture(216).map(AngleTarget)
    }
    fn force_look_at(&mut self, target: &Self::ForceTarget) -> Result<bool, Self::Error> {
        match *target
            .0
            .lock()
            .map_err(|_| HeadTrackingLiveError::Cvar(InputCvarError::Lock))?
        {
            Value::Boolean(value) => Ok(value),
            _ => Err(HeadTrackingLiveError::Cvar(InputCvarError::WrongType)),
        }
    }
    fn maximum_target_angle(&mut self, target: &Self::AngleTarget) -> Result<f32, Self::Error> {
        match *target
            .0
            .lock()
            .map_err(|_| HeadTrackingLiveError::Cvar(InputCvarError::Lock))?
        {
            Value::Float(value) => Ok(value),
            _ => Err(HeadTrackingLiveError::Cvar(InputCvarError::WrongType)),
        }
    }
    fn forced_target(&mut self) -> Result<[f32; 4], Self::Error> {
        self.attribute(0xdb6c_599d_0f2b_ece1)
    }
    fn acceleration_cap(&mut self) -> Result<f32, Self::Error> {
        self.attribute::<1>(0xccaf_d906_f935_6d1e).map(|v| v[0])
    }
    fn draw_head_adjust(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_current_boolean(219)
            .map_err(HeadTrackingLiveError::Cvar)
    }
    fn clamp_ideal(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_current_boolean(217)
            .map_err(HeadTrackingLiveError::Cvar)
    }
    fn largest_up_angle(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_current_float(218)
            .map_err(HeadTrackingLiveError::Cvar)
    }
    fn rotation_limits(&mut self) -> Result<[[f32; 4]; 2], Self::Error> {
        let layout = self
            .attributes
            .try_cached_layout(0x30)
            .map_err(HeadTrackingLiveError::Attribute)?;
        Ok(std::array::from_fn(|v| {
            std::array::from_fn(|i| {
                layout.as_ref().map_or(0.0, |d| {
                    f32::from_be_bytes(
                        d.bytes()[v * 16 + i * 4..v * 16 + i * 4 + 4]
                            .try_into()
                            .unwrap(),
                    )
                })
            })
        }))
    }
}
impl SkeletonHeadTracking {
    pub(crate) fn try_post_wipeout_update<N: ReckoningNumeric>(
        &mut self,
        world_to_animation: &Matrix,
        head_transform: &Matrix,
        output: &mut Vector,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), HeadTrackingError<HeadTrackingLiveError, N::Error>> {
        let state = HeadTrackingState {
            filter: &mut self.filter,
            target_position: &mut self.target_position,
            rotation: &mut self.rotation,
            rotation_velocity: &mut self.rotation_velocity,
            valid_target: &mut self.valid_target,
        };
        let mut source = Source {
            attributes: &self.attributes,
            cvars,
        };
        post_wipeout_update::try_update(
            state,
            world_to_animation,
            head_transform,
            output,
            &mut source,
            numeric,
        )
    }
}
