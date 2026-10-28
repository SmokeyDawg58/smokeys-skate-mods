//! Knowledge of native W lanes beside the existing `PhysRecord` host projections.
//! Register as `skeleton_physics_history::lane_knowledge`; no second record.
use super::{SKELETON_PART_COUNT, SkeletonPhysicsHistory, SkeletonPhysicsRecord};
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};

#[derive(Debug)]
pub(super) struct RecordLaneKnowledge {
    transform: [[bool; 4]; SKELETON_PART_COUNT],
    position: [bool; SKELETON_PART_COUNT],
    velocity: [bool; SKELETON_PART_COUNT],
    velocity_delta: [bool; SKELETON_PART_COUNT],
    centre_of_mass: bool,
    centre_of_mass_velocity: bool,
    proxy_volume_position: bool,
}
impl RecordLaneKnowledge {
    /// `PhysRecord`82b3d6f0 writes all transform/vector lanes, including zero W.
    pub(super) const fn constructor_values() -> Self {
        Self {
            transform: [[true; 4]; SKELETON_PART_COUNT],
            position: [true; SKELETON_PART_COUNT],
            velocity: [true; SKELETON_PART_COUNT],
            velocity_delta: [true; SKELETON_PART_COUNT],
            centre_of_mass: true,
            centre_of_mass_velocity: true,
            proxy_volume_position: true,
        }
    }
}

/// These are the existing native vector fields, not new cached calculations.
#[derive(Clone, Copy)]
pub(crate) enum RecordVectorField {
    Position(usize),
    Velocity(usize),
    VelocityDelta(usize),
    CentreOfMass,
    CentreOfMassVelocity,
    ProxyVolumePosition,
}

impl SkeletonPhysicsRecord {
    pub(crate) fn transform_typed(&self, part: usize) -> Option<SkeletonRootMatrix> {
        let matrix = self.transforms.get(part)?;
        Some(SkeletonRootMatrix::from_columns(std::array::from_fn(
            |column| {
                let offset = column * 4;
                SkeletonRootVector {
                    xyz: [matrix[offset], matrix[offset + 1], matrix[offset + 2]],
                    w: self.lanes.transform[part][column].then_some(matrix[offset + 3]),
                }
            },
        )))
    }
    /// Call at the native full-matrix store, AFTER its existing XYZ assertions.
    /// Unknown Ws receive canonical host projection storage, marked unknown.
    /// Only `transform_typed` may promote this storage into source-facing lanes.
    pub(super) fn store_transform_typed(&mut self, part: usize, matrix: &SkeletonRootMatrix) {
        for (column, value) in matrix.columns().iter().enumerate() {
            let offset = column * 4;
            self.transforms[part][offset..offset + 3].copy_from_slice(&value.xyz);
            self.transforms[part][offset + 3] =
                value.w.unwrap_or(if column == 3 { 1.0 } else { 0.0 });
            self.lanes.transform[part][column] = value.w.is_some();
        }
    }
    pub(crate) fn vector_typed(&self, field: RecordVectorField) -> SkeletonRootVector {
        let (value, known) = match field {
            RecordVectorField::Position(i) => (&self.positions[i], self.lanes.position[i]),
            RecordVectorField::Velocity(i) => (&self.velocities[i], self.lanes.velocity[i]),
            RecordVectorField::VelocityDelta(i) => {
                (&self.velocity_deltas[i], self.lanes.velocity_delta[i])
            }
            RecordVectorField::CentreOfMass => (&self.centre_of_mass, self.lanes.centre_of_mass),
            RecordVectorField::CentreOfMassVelocity => (
                &self.centre_of_mass_velocity,
                self.lanes.centre_of_mass_velocity,
            ),
            RecordVectorField::ProxyVolumePosition => (
                &self.proxy_volume_position,
                self.lanes.proxy_volume_position,
            ),
        };
        SkeletonRootVector {
            xyz: [value[0], value[1], value[2]],
            w: known.then_some(value[3]),
        }
    }
    /// Call at each existing vector store. Prior validated prefix remains live.
    pub(super) fn store_vector_typed(
        &mut self,
        field: RecordVectorField,
        value: SkeletonRootVector,
    ) {
        let (storage, known) = match field {
            RecordVectorField::Position(i) => (&mut self.positions[i], &mut self.lanes.position[i]),
            RecordVectorField::Velocity(i) => {
                (&mut self.velocities[i], &mut self.lanes.velocity[i])
            }
            RecordVectorField::VelocityDelta(i) => (
                &mut self.velocity_deltas[i],
                &mut self.lanes.velocity_delta[i],
            ),
            RecordVectorField::CentreOfMass => {
                (&mut self.centre_of_mass, &mut self.lanes.centre_of_mass)
            }
            RecordVectorField::CentreOfMassVelocity => (
                &mut self.centre_of_mass_velocity,
                &mut self.lanes.centre_of_mass_velocity,
            ),
            RecordVectorField::ProxyVolumePosition => (
                &mut self.proxy_volume_position,
                &mut self.lanes.proxy_volume_position,
            ),
        };
        storage[..3].copy_from_slice(&value.xyz);
        storage[3] = value.w.unwrap_or(0.0); // Explicit host scratch, never known W.
        *known = value.w.is_some();
    }
}

impl SkeletonPhysicsHistory {
    pub(crate) fn post_wipeout_record_transform(&self, part: usize) -> SkeletonRootMatrix {
        self.record
            .transform_typed(part)
            .expect("native PhysRecord part0..23")
    }
    /// Source root-adjustment stores preserve unrelated positions/velocities.
    /// Native `PhysRecord::SetTransform82b3a800` validates I/J/K/P XYZ before
    /// the first store. W is excluded, and infinity is accepted.
    pub(crate) fn try_set_post_wipeout_record_transform(
        &mut self,
        part: usize,
        matrix: &SkeletonRootMatrix,
    ) -> Result<
        (),
        super::SkeletonPhysicsHistoryError<std::convert::Infallible, std::convert::Infallible>,
    > {
        for (axis, column) in matrix.columns().iter().enumerate() {
            super::validate_vector(
                [column.xyz[0], column.xyz[1], column.xyz[2], 0.0],
                Some(part),
                super::SkeletonPhysicsVectorField::TransformColumn(axis),
            )?;
        }
        self.record.store_transform_typed(part, matrix);
        Ok(())
    }
    /// `AddOffset`82b3d7b0: translations1..23 only; no root or cached-vector store.
    /// W remains unknown if either actual add operand is unknown, even for zero.
    pub(crate) fn add_post_wipeout_skateboard_error_typed(&mut self, offset: SkeletonRootVector) {
        for part in 1..SKELETON_PART_COUNT {
            let matrix = self.post_wipeout_record_transform(part);
            let position = matrix.columns()[3].zip(offset, |a, b| a + b);
            self.record.transforms[part][12..15].copy_from_slice(&position.xyz);
            self.record.transforms[part][15] = position.w.unwrap_or(1.0);
            self.record.lanes.transform[part][3] = position.w.is_some();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constructor_lanes_are_known_and_typed_stores_can_invalidate_them() {
        let mut history = SkeletonPhysicsHistory::new([1.0; 24]);
        assert!(
            history
                .post_wipeout_record_transform(0)
                .columns()
                .iter()
                .all(|v| v.w == Some(0.0))
        );
        let mut matrix = SkeletonRootMatrix::known(super::super::FRESH_VOLUME_TRANSFORM);
        matrix.set_column(3, SkeletonRootVector::known([0.0, 0.0, 0.0, 1.0]));
        history
            .try_set_post_wipeout_record_transform(0, &matrix)
            .unwrap();
        assert_eq!(history.post_wipeout_record_transform(0), matrix);
        let mut unknown = matrix;
        unknown.set_column(
            2,
            SkeletonRootVector {
                xyz: [0.0, 0.0, 1.0],
                w: None,
            },
        );
        history
            .try_set_post_wipeout_record_transform(0, &unknown)
            .unwrap();
        assert!(
            history.post_wipeout_record_transform(0).columns()[2]
                .w
                .is_none()
        );
        assert_eq!(
            history.post_wipeout_record_transform(0).columns()[3].w,
            Some(1.0)
        );
    }
    #[test]
    fn unknown_offset_updates_xyz_without_rewriting_root_or_cached_position() {
        let mut history = SkeletonPhysicsHistory::new([1.0; 24]);
        let matrix = SkeletonRootMatrix::known(super::super::FRESH_VOLUME_TRANSFORM);
        for part in 0..24 {
            history
                .try_set_post_wipeout_record_transform(part, &matrix)
                .unwrap();
        }
        history.record.store_vector_typed(
            RecordVectorField::Position(1),
            SkeletonRootVector::known([9.0, 8.0, 7.0, 6.0]),
        );
        history.add_post_wipeout_skateboard_error_typed(SkeletonRootVector {
            xyz: [2.0, 3.0, 4.0],
            w: None,
        });
        assert_eq!(history.post_wipeout_record_transform(0), matrix);
        for part in 1..24 {
            let transform = history.post_wipeout_record_transform(part);
            assert_eq!(
                transform.columns()[3].xyz.map(f32::to_bits),
                [2.0_f32, 3.0, 4.0].map(f32::to_bits)
            );
            assert!(transform.columns()[3].w.is_none());
            assert_eq!(transform.columns()[0], matrix.columns()[0]);
        }
        assert_eq!(
            history
                .record
                .vector_typed(RecordVectorField::Position(1))
                .known_lanes(),
            Some([9.0, 8.0, 7.0, 6.0])
        );
    }
    #[test]
    fn known_w_bits_survive_and_unknown_vector_store_invalidates_old_knowledge() {
        let mut history = SkeletonPhysicsHistory::new([1.0; 24]);
        let field = RecordVectorField::CentreOfMass;
        history
            .record
            .store_vector_typed(field, SkeletonRootVector::known([1.0, 2.0, 3.0, -0.0]));
        assert_eq!(
            history.record.vector_typed(field).w.unwrap().to_bits(),
            0x8000_0000
        );
        history.record.store_vector_typed(
            field,
            SkeletonRootVector {
                xyz: [4.0, 5.0, 6.0],
                w: None,
            },
        );
        assert!(history.record.vector_typed(field).w.is_none());
    }
}

#[cfg(test)]
mod checked_transform_tests {
    use super::*;
    #[test]
    fn every_xyz_assertion_precedes_all_matrix_stores_and_ignores_w() {
        let mut history = SkeletonPhysicsHistory::new([1.0; 24]);
        let prior = history.post_wipeout_record_transform(3);
        for axis in 0..4 {
            let mut invalid = prior;
            invalid.set_column(0, SkeletonRootVector::known([2.0, 3.0, 4.0, 5.0]));
            invalid.set_column(
                axis,
                SkeletonRootVector {
                    xyz: [f32::NAN, 6.0, 7.0],
                    w: None,
                },
            );
            let error = history
                .try_set_post_wipeout_record_transform(3, &invalid)
                .unwrap_err();
            assert!(
                matches!(error, super::super::SkeletonPhysicsHistoryError::InvalidVector {
                part: Some(3), field: super::super::SkeletonPhysicsVectorField::TransformColumn(found)
            } if found == axis)
            );
            assert_eq!(history.post_wipeout_record_transform(3), prior);
        }
        let mut valid = prior;
        valid.set_column(
            2,
            SkeletonRootVector::known([f32::INFINITY, f32::NEG_INFINITY, 0.0, f32::NAN]),
        );
        history
            .try_set_post_wipeout_record_transform(3, &valid)
            .unwrap();
        let stored = history.post_wipeout_record_transform(3).columns()[2];
        assert_eq!(
            stored.xyz.map(f32::to_bits),
            valid.columns()[2].xyz.map(f32::to_bits)
        );
        assert!(stored.w.unwrap().is_nan());
        assert_eq!(history.record().positions()[3].map(f32::to_bits), [0; 4]);
    }
}
