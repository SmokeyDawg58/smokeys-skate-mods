//! Late processed reads and ordered passed-output stores for `GroundSkateboard`.
use crate::skeleton_root_transforms::SkeletonRootMatrix;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroundedInputError {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
}
impl std::fmt::Display for GroundedInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "grounded processed field missing: {v}"),
            Self::Poisoned => f.write_str("grounded processed lock poisoned"),
            Self::ForeignOwner => f.write_str("grounded processed owner differs"),
        }
    }
}
pub(crate) trait GroundedInputRead {
    fn time_delta(&self) -> Result<f32, GroundedInputError>;
    fn gravity(&self) -> Result<f32, GroundedInputError>;
    fn current_state(&self) -> Result<i32, GroundedInputError>;
    fn flags_8c8(&self, mask: u32) -> Result<u32, GroundedInputError>;
    fn flags_8d0(&self, mask: u32) -> Result<u32, GroundedInputError>;
}
pub(crate) trait GroundedOutputWrite {
    fn write_orientation(&self, value: SkeletonRootMatrix) -> Result<(), GroundedInputError>;
    fn clear_orientation_flag(&self) -> Result<(), GroundedInputError>;
}
impl GroundedInputRead for super::SkeletonGeneralInput {
    fn time_delta(&self) -> Result<f32, GroundedInputError> {
        Ok(self.time_delta)
    }
    fn gravity(&self) -> Result<f32, GroundedInputError> {
        Ok(self.gravity)
    }
    fn current_state(&self) -> Result<i32, GroundedInputError> {
        Ok(self.current_physics_state)
    }
    fn flags_8c8(&self, mask: u32) -> Result<u32, GroundedInputError> {
        Ok(self.flags_8c8 & mask)
    }
    fn flags_8d0(&self, mask: u32) -> Result<u32, GroundedInputError> {
        Ok(self.flags_8d0 & mask)
    }
}

impl GroundedOutputWrite for std::cell::RefCell<&mut super::SkeletonGroundedPassedOutput> {
    fn write_orientation(&self, value: SkeletonRootMatrix) -> Result<(), GroundedInputError> {
        self.borrow_mut().orientation = value;
        Ok(())
    }
    fn clear_orientation_flag(&self) -> Result<(), GroundedInputError> {
        self.borrow_mut().flags_8c4 &= !0x0008_0000;
        Ok(())
    }
}
