//! Actual `FreeCamManager::Update82e40940`, using its retained parameter owner.
use crate::{
    camera_attributes::CameraParameterPublication, camera_man::CameraFreeCam,
    skeleton_root_transforms::SkeletonRootVector,
};

#[cfg(test)]
#[path = "camera_free_cam_update_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum FreeCamUpdateError {
    OwnerLock,
    MissingParameter(&'static str),
}
impl std::fmt::Display for FreeCamUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("free camera parameter lock"),
            Self::MissingParameter(name) => write!(f, "unproduced free camera parameter {name}"),
        }
    }
}

fn required(value: Option<f32>, name: &'static str) -> Result<f32, FreeCamUpdateError> {
    value.ok_or(FreeCamUpdateError::MissingParameter(name))
}

/// Source's two FSELs, including its unordered-to-upper-bound result.
fn clamp_target(value: f32, limit: f32) -> f32 {
    let lower = if (-limit) - value >= 0.0 {
        -limit
    } else {
        value
    };
    if limit - lower >= 0.0 { lower } else { limit }
}

impl CameraFreeCam {
    pub(crate) fn try_update(
        &mut self,
        look_stick: SkeletonRootVector,
    ) -> Result<(), FreeCamUpdateError> {
        let attributes = self
            .parameters
            .lock()
            .map_err(|_| FreeCamUpdateError::OwnerLock)?;
        let CameraParameterPublication::Fields(parameters) = &attributes.parameters else {
            return Err(FreeCamUpdateError::MissingParameter("parameters"));
        };
        let old_heading = self.heading_offset;
        let old_elevation = self.elevation_offset;
        let heading_offset = required(parameters.free_cam_heading_offset, "heading_offset")?;
        let radians = f32::from_bits(0x3c8e_fa35);
        let margin = 5.0 * radians;
        let heading_speed = required(parameters.free_cam_heading_speed, "heading_speed")?;
        let heading_input = look_stick.xyz[0] * heading_offset;
        let elevation_speed = required(parameters.free_cam_elevation_speed, "elevation_speed")?;
        let heading_step = heading_speed * f32::from_bits(0x3c88_8889);
        let elevation_step = elevation_speed * f32::from_bits(0x3c88_8889);
        let limit = f32::from_bits(0x3fc9_0fdb) - margin;
        let desired_heading = heading_input * radians;
        let desired_heading = clamp_target(desired_heading, limit);
        self.heading_offset = (desired_heading - old_heading).mul_add(heading_step, old_heading);
        let elevation_offset = required(parameters.free_cam_elevation_offset, "elevation_offset")?;
        let elevation_input = look_stick.xyz[1] * elevation_offset;
        let desired_elevation = clamp_target(elevation_input * radians, limit);
        self.elevation_offset =
            (desired_elevation - old_elevation).mul_add(elevation_step, old_elevation);
        Ok(())
    }
}
