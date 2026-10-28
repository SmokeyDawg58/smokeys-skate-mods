//! `UpdateRigSubjectMatrix82e41208` over the same retained Subject and Rig.
use crate::{
    camera_man::CameraMan,
    camera_rig::CameraRig,
    camera_subject::ZERO,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    skeleton_root_transforms::SkeletonRootMatrix,
};

#[derive(Debug)]
pub(crate) enum RigSubjectMatrixError {
    OwnerLock,
    MissingSubject,
    AnchorIndex(u32),
    MatrixNaN { column: usize, lane: usize },
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}

impl CameraMan {
    pub(crate) fn try_update_rig_subject_matrix(
        &mut self,
        dt: f32,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), RigSubjectMatrixError> {
        let anchor = self.shots.current.anchor;
        if anchor != self.current_anchor {
            self.current_anchor = anchor;
            self.try_update_anchor(0.0, cvars, overlay)?;
            self.rig.second_flags |= 0x40;
            self.rig.second_flags_known |= 0x40;
        }
        self.try_update_anchor(dt, cvars, overlay)?;
        self.rig.subject_acceleration = self
            .anchor_velocity
            .zip(self.rig.subject_velocity, |new, old| new - old);
        self.rig.subject_velocity = self.anchor_velocity;
        self.rig.subject_angular_velocity = ZERO;
        let mut columns = *self
            .subject
            .as_ref()
            .ok_or(RigSubjectMatrixError::MissingSubject)?
            .lock()
            .map_err(|_| RigSubjectMatrixError::OwnerLock)?
            .board_effective_transform
            .columns();
        columns[3] = self.anchor_position;
        self.rig
            .try_set_anchor_matrix(SkeletonRootMatrix::from_columns(columns))
    }

    /// `UpdateAnchor82e3e6a8`: dt0 and teleports retain old anchor velocity.
    fn try_update_anchor(
        &mut self,
        dt: f32,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), RigSubjectMatrixError> {
        use RigSubjectMatrixError as E;
        let subject = self.subject.as_ref().ok_or(E::MissingSubject)?;
        let teleported = subject.lock().map_err(|_| E::OwnerLock)?.flags.teleported;
        let reset = if dt == 0.0 || teleported || self.heading_follows_model_time > 0.0 {
            teleported
        } else {
            false
        };
        let old = self.anchor_position;
        self.anchor_position = *subject
            .lock()
            .map_err(|_| E::OwnerLock)?
            .anchor_positions
            .get(
                usize::try_from(self.current_anchor)
                    .map_err(|_| E::AnchorIndex(self.current_anchor))?,
            )
            .ok_or(E::AnchorIndex(self.current_anchor))?;
        if dt > 0.0 && !reset {
            let delta = self.anchor_position.zip(old, |new, old| new - old);
            let inverse = 1.0 / dt;
            self.anchor_velocity = delta.map(|lane| lane * inverse);
        }
        if cvars.try_camera_show_anchor_debug().map_err(E::Cvar)? {
            overlay.require().map_err(E::Overlay)?;
            let mut index = 0;
            while index
                < subject
                    .lock()
                    .map_err(|_| E::OwnerLock)?
                    .anchor_positions
                    .len()
            {
                let owner = subject.lock().map_err(|_| E::OwnerLock)?;
                // Native debug build retains getter traversal; rendered draw
                // calls are absent in this executable body.
                owner.anchor_positions.get(index).ok_or(E::AnchorIndex(
                    u32::try_from(index).expect("native anchor count"),
                ))?;
                index += 1;
            }
        }
        Ok(())
    }
}

impl CameraRig {
    /// `SetAnchorMatrix82e3f9d8`: six assertions finish before matrix stores.
    fn try_set_anchor_matrix(
        &mut self,
        matrix: SkeletonRootMatrix,
    ) -> Result<(), RigSubjectMatrixError> {
        validate_anchor(matrix)?;
        self.anchor_matrix = matrix;
        self.flags |= 0x10;
        Ok(())
    }
}
fn validate_anchor(matrix: SkeletonRootMatrix) -> Result<(), RigSubjectMatrixError> {
    let columns = matrix.columns();
    for column in [3, 0, 1, 2] {
        for lane in 0..3 {
            if columns[column].xyz[lane].is_nan() {
                return Err(RigSubjectMatrixError::MatrixNaN { column, lane });
            }
        }
    }
    Ok(())
}
impl std::fmt::Display for RigSubjectMatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("Rig anchor owner lock"),
            Self::MissingSubject => f.write_str("Rig anchor Subject missing"),
            Self::AnchorIndex(index) => write!(f, "Subject anchor index {index}"),
            Self::MatrixNaN { column, lane } => {
                write!(f, "Rig anchor matrix assertion column{column} lane{lane}")
            }
            Self::Cvar(error) => write!(f, "{error}"),
            Self::Overlay(error) => write!(f, "{error}"),
        }
    }
}
#[cfg(test)]
#[path = "camera_rig_subject_matrix_tests.rs"]
mod tests;
