//! Retained mass and physics-record projection of `SkeletonState`.
//!
//! This is not its animation record, attribute instance, or native reference
//! layout. The actual assembly owner supplies constructor `PartMass` values and
//! live Part getters, and enforces the two-call post-Set initialization phase.
//! Errors replace native assertions/service failures and retain earlier stores.
//! Full W lanes survive; validity checks reject XYZ NaN only, not infinity.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "physics history awaits local player lifecycle")
)]

use crate::ragdoll::SKELETON_PART_COUNT;
use crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};
use std::fmt;
#[path = "skeleton_physics_history_lane_knowledge.rs"]
pub(crate) mod lane_knowledge;
#[path = "skeleton_physics_history_typed_update.rs"]
mod typed_update;
#[path = "skeleton_physics_history_wipeout_angular.rs"]
mod wipeout_angular;

#[cfg(test)]
#[path = "skeleton_physics_history_tests.rs"]
mod tests;

/// The single `vmsum3fp` at 0x82b3d650 has an explicit numeric boundary.
pub(crate) trait SkeletonPhysicsHistoryNumeric {
    fn dot3(&mut self, displacement: [f32; 4], up: [f32; 4]) -> f32;
}

/// The owner acquires the current `ProcessedPhysIn` collaborator after each
/// supplied-root getter. Its Up lanes are loaded only on the later projected
/// COM branch. `Up` retains that collaborator, not a fabricated native address.
pub(crate) trait SkeletonHistoryInput {
    type Error;
    type Up;
    fn acquire_up_vector(&mut self) -> Result<Self::Up, Self::Error>;
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error>;
    fn read_up_vector(&mut self, up: &Self::Up) -> Result<[f32; 4], Self::Error>;
    /// Full-lane fixture/service input; live partial-lane owners override this.
    fn read_up_vector_typed(&mut self, up: &Self::Up) -> Result<SkeletonRootVector, Self::Error> {
        self.read_up_vector(up).map(SkeletonRootVector::known)
    }
}

/// Host separate f32 products and left-associated additions; not a claim about
/// Xenon's internal reduction, denormal handling, or NaN payload propagation.
pub(crate) struct HostSkeletonPhysicsHistoryNumeric;
impl SkeletonPhysicsHistoryNumeric for HostSkeletonPhysicsHistoryNumeric {
    fn dot3(&mut self, displacement: [f32; 4], up: [f32; 4]) -> f32 {
        (displacement[0] * up[0] + displacement[1] * up[1]) + displacement[2] * up[2]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonPhysicsVectorField {
    TransformColumn(usize),
    VelocityDelta,
    Velocity,
    Position,
    CentreOfMass,
    CentreOfMassVelocity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SkeletonPhysicsMassError;
impl fmt::Display for SkeletonPhysicsMassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Skeleton total PartMass is not greater than zero")
    }
}
impl std::error::Error for SkeletonPhysicsMassError {}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonPhysicsHistoryError<G, C> {
    LivePart {
        part: usize,
        error: G,
    },
    Cvar(C),
    UpVector(C),
    InvalidTotalMass,
    InvalidVector {
        part: Option<usize>,
        field: SkeletonPhysicsVectorField,
    },
}
impl<G: fmt::Display, C: fmt::Display> fmt::Display for SkeletonPhysicsHistoryError<G, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LivePart { part, error } => write!(f, "Skeleton history Part {part}: {error}"),
            Self::Cvar(error) => write!(f, "Skeleton COM cvar: {error}"),
            Self::UpVector(error) => write!(f, "Skeleton COM up vector: {error}"),
            Self::InvalidTotalMass => SkeletonPhysicsMassError.fmt(f),
            Self::InvalidVector { part, field } => {
                write!(f, "Skeleton history {part:?} {field:?} contains XYZ NaN")
            }
        }
    }
}
impl<G: std::error::Error + 'static, C: std::error::Error + 'static> std::error::Error
    for SkeletonPhysicsHistoryError<G, C>
{
}

/// Native `PhysRecord` constructor 0x82b3d6f0: 24 I/J/K bases, zero translations and
/// vectors. Read-only views do not give the caller an alternate mutation owner.
#[derive(Debug)]
pub(crate) struct SkeletonPhysicsRecord {
    lanes: lane_knowledge::RecordLaneKnowledge,
    transforms: [[f32; 16]; SKELETON_PART_COUNT],
    positions: [[f32; 4]; SKELETON_PART_COUNT],
    velocities: [[f32; 4]; SKELETON_PART_COUNT],
    velocity_deltas: [[f32; 4]; SKELETON_PART_COUNT],
    centre_of_mass: [f32; 4],
    centre_of_mass_velocity: [f32; 4],
    proxy_volume_position: [f32; 4],
}
impl SkeletonPhysicsRecord {
    /// Host projections; source-facing consumers use `transform_typed`.
    pub(crate) fn transforms(&self) -> &[[f32; 16]; SKELETON_PART_COUNT] {
        &self.transforms
    }
    pub(crate) fn positions(&self) -> &[[f32; 4]; SKELETON_PART_COUNT] {
        &self.positions
    }
    pub(crate) fn velocities(&self) -> &[[f32; 4]; SKELETON_PART_COUNT] {
        &self.velocities
    }
    pub(crate) fn velocity_deltas(&self) -> &[[f32; 4]; SKELETON_PART_COUNT] {
        &self.velocity_deltas
    }
    pub(crate) fn centre_of_mass(&self) -> [f32; 4] {
        self.centre_of_mass
    }
    pub(crate) fn centre_of_mass_velocity(&self) -> [f32; 4] {
        self.centre_of_mass_velocity
    }
    pub(crate) fn proxy_volume_position(&self) -> [f32; 4] {
        self.proxy_volume_position
    }
}

/// One authoritative history, not Clone. The supplied masses are the original
/// Body constructor's unscaled `PartMass` stores, never effective body inertia.
#[derive(Debug)]
pub(crate) struct SkeletonPhysicsHistory {
    part_masses: [f32; SKELETON_PART_COUNT],
    fractional_part_masses: [f32; SKELETON_PART_COUNT],
    total_mass: f32,
    time_delta: f32,
    record: SkeletonPhysicsRecord,
}
impl SkeletonPhysicsHistory {
    #[cfg(test)]
    fn update_part<G, C>(
        &mut self,
        part: usize,
        matrix: [f32; 16],
    ) -> Result<(), SkeletonPhysicsHistoryError<G, C>> {
        self.update_part_typed(part, SkeletonRootMatrix::known(matrix))
    }
    #[cfg(test)]
    fn calculate_centre_of_mass<G, I: SkeletonHistoryInput, N: SkeletonPhysicsHistoryNumeric>(
        &mut self,
        up: &I::Up,
        input: &mut I,
        numeric: &mut N,
    ) -> Result<(), SkeletonPhysicsHistoryError<G, I::Error>> {
        self.calculate_centre_of_mass_typed(up, input, numeric)
    }
    pub(crate) fn new(unscaled_part_mass: [f32; SKELETON_PART_COUNT]) -> Self {
        Self {
            part_masses: unscaled_part_mass,
            fractional_part_masses: [0.0; SKELETON_PART_COUNT],
            total_mass: 0.0,
            time_delta: 0.0,
            record: SkeletonPhysicsRecord {
                lanes: lane_knowledge::RecordLaneKnowledge::constructor_values(),
                transforms: [FRESH_VOLUME_TRANSFORM; SKELETON_PART_COUNT],
                positions: [[0.0; 4]; SKELETON_PART_COUNT],
                velocities: [[0.0; 4]; SKELETON_PART_COUNT],
                velocity_deltas: [[0.0; 4]; SKELETON_PART_COUNT],
                centre_of_mass: [0.0; 4],
                centre_of_mass_velocity: [0.0; 4],
                proxy_volume_position: [0.0; 4],
            },
        }
    }
    pub(crate) fn part_masses(&self) -> &[f32; SKELETON_PART_COUNT] {
        &self.part_masses
    }
    /// Actual Body constructor copies each original unscaled mass into the
    /// previously constructed `SkeletonState`. No normalization or mass sum yet.
    pub(crate) fn store_body_part_mass(&mut self, part: usize, mass: f32) {
        self.part_masses[part] = mass;
    }
    pub(crate) fn fractional_part_masses(&self) -> &[f32; SKELETON_PART_COUNT] {
        &self.fractional_part_masses
    }
    pub(crate) fn total_mass(&self) -> f32 {
        self.total_mass
    }
    pub(crate) fn time_delta(&self) -> f32 {
        self.time_delta
    }
    pub(crate) fn record(&self) -> &SkeletonPhysicsRecord {
        &self.record
    }

    /// Physics portion of `SkeletonState::Reset` (0x82b3d238..0x82b3d328).
    /// Read each actual Part after all body reset writes, validate all matrix
    /// columns before copying, then store its position and zero velocity.
    /// Masses, timestep, velocity deltas, COM and proxy position survive.
    /// The owning Skeleton commits the separate animation/proxy-target tail
    /// only after this complete ascending pass succeeds.
    pub(crate) fn try_reset<G, E>(
        &mut self,
        mut getter: G,
    ) -> Result<(), SkeletonPhysicsHistoryError<E, std::convert::Infallible>>
    where
        G: FnMut(usize) -> Result<[f32; 16], E>,
    {
        self.try_reset_typed(|part| getter(part).map(SkeletonRootMatrix::known))
    }

    /// 0x82b3d0e0: ascending f32 sum, strict positive assertion, ONE division.
    #[expect(
        clippy::assign_op_pattern,
        reason = "preserve source mass + accumulated total operand order"
    )]
    pub(crate) fn try_initialize_masses(&mut self) -> Result<(), SkeletonPhysicsMassError> {
        self.total_mass = 0.0;
        for mass in self.part_masses {
            self.total_mass = mass + self.total_mass;
        }
        if self.total_mass.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(SkeletonPhysicsMassError);
        }
        let reciprocal = 1.0 / self.total_mass;
        for part in 0..SKELETON_PART_COUNT {
            self.fractional_part_masses[part] = self.part_masses[part] * reciprocal;
        }
        Ok(())
    }

    /// `UpdatePhysicsRecord` 0x82b48288. Root and the up collaborator have been
    /// acquired by the actual owner. The getter reads proxy Part0 and Parts1..23;
    /// the current cvar is sampled only after all Part records and COM sums.
    /// Up lanes load only on the projection branch, after the cvar sample.
    /// The immutable supplied root snapshot excludes concurrent native alias
    /// mutation; source assertions become prefix-preserving typed errors.
    pub(crate) fn try_update<G, I, N, GE>(
        &mut self,
        supplied_root: [f32; 16],
        up: &I::Up,
        get_live_part: &mut G,
        input: &mut I,
        numeric: &mut N,
    ) -> Result<(), SkeletonPhysicsHistoryError<GE, I::Error>>
    where
        G: FnMut(usize) -> Result<[f32; 16], GE>,
        I: SkeletonHistoryInput,
        N: SkeletonPhysicsHistoryNumeric,
    {
        self.try_update_typed(
            SkeletonRootMatrix::known(supplied_root),
            up,
            &mut |part| get_live_part(part).map(SkeletonRootMatrix::known),
            input,
            numeric,
        )
    }
}

fn position(matrix: [f32; 16]) -> [f32; 4] {
    [matrix[12], matrix[13], matrix[14], matrix[15]]
}

/// vrlimi(mask1, rotation1) produces [X,Y,Z,X]; its self equality all-true
/// predicate accepts infinity, rejects XYZ NaN, and ignores original W.
fn validate_vector<G, C>(
    vector: [f32; 4],
    part: Option<usize>,
    field: SkeletonPhysicsVectorField,
) -> Result<(), SkeletonPhysicsHistoryError<G, C>> {
    if vector[..3].iter().any(|value| value.is_nan()) {
        Err(SkeletonPhysicsHistoryError::InvalidVector { part, field })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod reset_tests {
    use super::*;

    fn seeded() -> SkeletonPhysicsHistory {
        let mut history = SkeletonPhysicsHistory::new([2.0; SKELETON_PART_COUNT]);
        history.try_initialize_masses().unwrap();
        history.time_delta = 0.125;
        history.record.positions = [[31.0; 4]; SKELETON_PART_COUNT];
        history.record.velocities = [[41.0; 4]; SKELETON_PART_COUNT];
        history.record.velocity_deltas = [[51.0; 4]; SKELETON_PART_COUNT];
        history.record.centre_of_mass = [61.0; 4];
        history.record.centre_of_mass_velocity = [71.0; 4];
        history.record.proxy_volume_position = [81.0; 4];
        history
    }

    fn matrix(part: usize) -> [f32; 16] {
        let mut result = FRESH_VOLUME_TRANSFORM;
        result[12..].copy_from_slice(&[f32::from(u8::try_from(part).unwrap()), -3.0, 7.0, -0.0]);
        result
    }

    #[test]
    fn reset_changes_only_transforms_positions_and_full_zero_velocities() {
        let mut history = seeded();
        let fractions = history.fractional_part_masses;
        let mut visits = Vec::new();
        history
            .try_reset(|part| {
                visits.push(part);
                Ok::<_, ()>(matrix(part))
            })
            .unwrap();
        assert_eq!(visits, (0..24).collect::<Vec<_>>());
        for part in 0..24 {
            assert_eq!(
                history.record.transforms[part].map(f32::to_bits),
                matrix(part).map(f32::to_bits)
            );
            assert_eq!(
                history.record.positions[part].map(f32::to_bits),
                position(matrix(part)).map(f32::to_bits)
            );
            assert_eq!(history.record.velocities[part].map(f32::to_bits), [0; 4]);
            assert_eq!(
                history.record.velocity_deltas[part].map(f32::to_bits),
                [51.0_f32.to_bits(); 4]
            );
        }
        assert_eq!(
            history.part_masses.map(f32::to_bits),
            [2.0_f32.to_bits(); 24]
        );
        assert_eq!(
            history.fractional_part_masses.map(f32::to_bits),
            fractions.map(f32::to_bits)
        );
        assert_eq!(history.total_mass.to_bits(), 48.0_f32.to_bits());
        assert_eq!(history.time_delta.to_bits(), 0.125_f32.to_bits());
        assert_eq!(
            history.record.centre_of_mass.map(f32::to_bits),
            [61.0_f32.to_bits(); 4]
        );
        assert_eq!(
            history.record.centre_of_mass_velocity.map(f32::to_bits),
            [71.0_f32.to_bits(); 4]
        );
        assert_eq!(
            history.record.proxy_volume_position.map(f32::to_bits),
            [81.0_f32.to_bits(); 4]
        );
    }

    #[test]
    fn later_getter_failure_retains_completed_rows_without_touching_later_rows() {
        let mut history = seeded();
        let mut visits = Vec::new();
        assert_eq!(
            history.try_reset(|part| {
                visits.push(part);
                if part == 3 {
                    Err("getter")
                } else {
                    Ok(matrix(part))
                }
            }),
            Err(SkeletonPhysicsHistoryError::LivePart {
                part: 3,
                error: "getter"
            })
        );
        assert_eq!(visits, [0, 1, 2, 3]);
        for part in 0..24 {
            let expected = if part < 3 {
                matrix(part)
            } else {
                FRESH_VOLUME_TRANSFORM
            };
            assert_eq!(
                history.record.transforms[part].map(f32::to_bits),
                expected.map(f32::to_bits)
            );
            assert_eq!(
                history.record.velocities[part].map(f32::to_bits),
                if part < 3 {
                    [0; 4]
                } else {
                    [41.0_f32.to_bits(); 4]
                }
            );
        }
    }

    #[test]
    fn every_column_is_validated_before_the_failed_row_matrix_copy() {
        for column in 0..4 {
            let mut history = seeded();
            let error = history.try_reset(|part| {
                let mut value = matrix(part);
                if part == 2 {
                    value[column * 4 + 1] = f32::NAN;
                }
                Ok::<_, ()>(value)
            });
            assert_eq!(
                error,
                Err(SkeletonPhysicsHistoryError::InvalidVector {
                    part: Some(2),
                    field: SkeletonPhysicsVectorField::TransformColumn(column),
                })
            );
            assert_eq!(
                history.record.transforms[1].map(f32::to_bits),
                matrix(1).map(f32::to_bits)
            );
            assert_eq!(
                history.record.transforms[2].map(f32::to_bits),
                FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
            );
            assert_eq!(
                history.record.positions[2].map(f32::to_bits),
                [31.0_f32.to_bits(); 4]
            );
            assert_eq!(
                history.record.velocities[2].map(f32::to_bits),
                [41.0_f32.to_bits(); 4]
            );
        }
    }

    #[test]
    fn reset_accepts_xyz_infinity_and_preserves_nan_w_bits() {
        let mut history = seeded();
        let mut value = matrix(0);
        for column in 0..4 {
            value[column * 4] = f32::INFINITY;
            value[column * 4 + 3] = f32::from_bits(0x7fc0_0123 + u32::try_from(column).unwrap());
        }
        history.try_reset(|_| Ok::<_, ()>(value)).unwrap();
        assert_eq!(
            history.record.transforms[23].map(f32::to_bits),
            value.map(f32::to_bits)
        );
        assert_eq!(
            history.record.positions[23].map(f32::to_bits),
            position(value).map(f32::to_bits)
        );
        assert_eq!(history.record.velocities[23].map(f32::to_bits), [0; 4]);
    }
}
