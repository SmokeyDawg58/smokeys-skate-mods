//! Actual `Rig::GetSubjectSpeed82e3cd18`, shared by conditions and shot blending.
use crate::{
    camera_rig::CameraRig,
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
};

#[derive(Debug)]
pub(crate) enum SubjectSpeedError<E> {
    MissingSubject,
    OwnerLock,
    Numeric(E),
}

impl CameraRig {
    pub(crate) fn try_subject_speed<N: ReckoningNumeric>(
        &self,
        numeric: &mut N,
    ) -> Result<f32, SubjectSpeedError<N::Error>> {
        let mut velocity = self.subject_velocity;
        velocity.xyz[1] = 0.0;
        let subject = self
            .subject
            .as_ref()
            .ok_or(SubjectSpeedError::MissingSubject)?
            .lock()
            .map_err(|_| SubjectSpeedError::OwnerLock)?;
        let squared = numeric
            .dot_vector(velocity, velocity)
            .map_err(SubjectSpeedError::Numeric)?;
        let forward = subject.board_effective_transform.columns()[2];
        let along = numeric
            .dot_vector(velocity, forward)
            .map_err(SubjectSpeedError::Numeric)?;
        let estimate = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(SubjectSpeedError::Numeric)?;
        let speed = length_from_squared_and_estimate(squared, estimate);
        Ok(if (0.0 > along) == (self.second_flags & 0x80 != 0) {
            speed
        } else {
            -speed
        })
    }
}

impl<E: std::fmt::Display> std::fmt::Display for SubjectSpeedError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSubject => f.write_str("missing camera speed Subject"),
            Self::OwnerLock => f.write_str("camera speed Subject lock"),
            Self::Numeric(error) => write!(f, "camera speed numeric: {error}"),
        }
    }
}
