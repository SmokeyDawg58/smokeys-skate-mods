//! Native record stores with explicit knowledge of packed vector W lanes.
use super::{
    SKELETON_PART_COUNT, SkeletonHistoryInput, SkeletonPhysicsHistory,
    SkeletonPhysicsHistoryError as Error, SkeletonPhysicsHistoryNumeric,
    SkeletonPhysicsVectorField as Field, lane_knowledge::RecordVectorField as RecordField,
    validate_vector,
};
use crate::skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector};

fn xyz_assertion_lanes(vector: Vector) -> [f32; 4] {
    // The native assertion/dot3 ignores W. This is not a persisted vector value.
    [vector.xyz[0], vector.xyz[1], vector.xyz[2], 0.0]
}
impl SkeletonPhysicsHistory {
    pub(crate) fn try_reset_typed<G, E>(
        &mut self,
        mut getter: G,
    ) -> Result<(), Error<E, std::convert::Infallible>>
    where
        G: FnMut(usize) -> Result<Matrix, E>,
    {
        for part in 0..SKELETON_PART_COUNT {
            let matrix = getter(part).map_err(|error| Error::LivePart { part, error })?;
            for (column, value) in matrix.columns().iter().enumerate() {
                validate_vector(
                    xyz_assertion_lanes(*value),
                    Some(part),
                    Field::TransformColumn(column),
                )?;
            }
            self.record.store_transform_typed(part, &matrix);
            let position = matrix.columns()[3];
            validate_vector(xyz_assertion_lanes(position), Some(part), Field::Position)?;
            self.record
                .store_vector_typed(RecordField::Position(part), position);
            self.record
                .store_vector_typed(RecordField::Velocity(part), Vector::known([0.0; 4]));
        }
        Ok(())
    }
    pub(crate) fn try_update_typed<G, I, N, GE>(
        &mut self,
        supplied_root: Matrix,
        up: &I::Up,
        get_live_part: &mut G,
        input: &mut I,
        numeric: &mut N,
    ) -> Result<(), Error<GE, I::Error>>
    where
        G: FnMut(usize) -> Result<Matrix, GE>,
        I: SkeletonHistoryInput,
        N: SkeletonPhysicsHistoryNumeric,
    {
        self.time_delta = f32::from_bits(0x3c88_8889);
        self.update_part_typed(0, supplied_root)?;
        let proxy = get_live_part(0).map_err(|error| Error::LivePart { part: 0, error })?;
        self.record
            .store_vector_typed(RecordField::ProxyVolumePosition, proxy.columns()[3]);
        for part in 1..SKELETON_PART_COUNT {
            let matrix = get_live_part(part).map_err(|error| Error::LivePart { part, error })?;
            self.update_part_typed(part, matrix)?;
        }
        self.calculate_centre_of_mass_typed(up, input, numeric)
    }
    pub(super) fn update_part_typed<G, C>(
        &mut self,
        part: usize,
        matrix: Matrix,
    ) -> Result<(), Error<G, C>> {
        for (column, value) in matrix.columns().iter().enumerate() {
            validate_vector(
                xyz_assertion_lanes(*value),
                Some(part),
                Field::TransformColumn(column),
            )?;
        }
        self.record.store_transform_typed(part, &matrix);
        let next = matrix.columns()[3];
        let old = self.record.vector_typed(RecordField::Position(part));
        let velocity = next.zip(old, |a, b| (a - b) * f32::from_bits(0x426f_ffff));
        let delta = velocity.zip(
            self.record.vector_typed(RecordField::Velocity(part)),
            |a, b| a - b,
        );
        validate_vector(xyz_assertion_lanes(delta), Some(part), Field::VelocityDelta)?;
        self.record
            .store_vector_typed(RecordField::VelocityDelta(part), delta);
        validate_vector(xyz_assertion_lanes(velocity), Some(part), Field::Velocity)?;
        self.record
            .store_vector_typed(RecordField::Velocity(part), velocity);
        validate_vector(xyz_assertion_lanes(next), Some(part), Field::Position)?;
        self.record
            .store_vector_typed(RecordField::Position(part), next);
        Ok(())
    }
    pub(super) fn calculate_centre_of_mass_typed<G, I, N>(
        &mut self,
        up: &I::Up,
        input: &mut I,
        numeric: &mut N,
    ) -> Result<(), Error<G, I::Error>>
    where
        I: SkeletonHistoryInput,
        N: SkeletonPhysicsHistoryNumeric,
    {
        if self.total_mass.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(Error::InvalidTotalMass);
        }
        let mut centre = Vector::known([0.0; 4]);
        let mut velocity = Vector::known([0.0; 4]);
        for part in 0..SKELETON_PART_COUNT {
            let weight = self.fractional_part_masses[part];
            let position = self
                .record
                .transform_typed(part)
                .expect("native part range")
                .columns()[3];
            centre = position.zip(centre, |position, sum| position.mul_add(weight, sum));
            velocity = self
                .record
                .vector_typed(RecordField::Velocity(part))
                .zip(velocity, |velocity, sum| velocity.mul_add(weight, sum));
        }
        if input.use_xyz_centre_of_mass().map_err(Error::Cvar)? == 0 {
            let root = self
                .record
                .transform_typed(0)
                .expect("native root")
                .columns()[3];
            let displacement = centre.zip(root, |a, b| a - b);
            let up = input.read_up_vector_typed(up).map_err(Error::UpVector)?;
            let projection = numeric.dot3(
                displacement
                    .known_lanes()
                    .unwrap_or_else(|| xyz_assertion_lanes(displacement)),
                up.known_lanes().unwrap_or_else(|| xyz_assertion_lanes(up)),
            );
            centre = up.zip(root, |up, root| up.mul_add(projection, root));
        }
        validate_vector(xyz_assertion_lanes(centre), None, Field::CentreOfMass)?;
        self.record
            .store_vector_typed(RecordField::CentreOfMass, centre);
        validate_vector(
            xyz_assertion_lanes(velocity),
            None,
            Field::CentreOfMassVelocity,
        )?;
        self.record
            .store_vector_typed(RecordField::CentreOfMassVelocity, velocity);
        Ok(())
    }
}
