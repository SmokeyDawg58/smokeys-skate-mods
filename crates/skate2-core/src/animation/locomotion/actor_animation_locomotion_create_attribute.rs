//! CreateLocomotionTrajectory per-row ingestion after its first-state continuation.
use super::AnimationLocomotionTrajectory;
use super::locomotion_producer::TrajectoryProducerError;
use crate::skeleton_root_transforms::SkeletonRootMatrix as M;
use skate2_formats::andale::{AndaleAttributeValue, AndaleRuntimeAttribute};

#[derive(Debug)]
pub(super) enum CreateAttributeError<E> {
    Index(i32),
    Duration,
    RawTime,
    Vector,
    Trajectory(TrajectoryProducerError<std::convert::Infallible>),
    Tuning(E),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton_root_transforms::SkeletonRootVector as V;

    fn row(name: &str, value: AndaleAttributeValue) -> AndaleRuntimeAttribute {
        AndaleRuntimeAttribute {
            sequence_id: 2,
            begin_seconds: 0.0,
            end_seconds: 1.0,
            name: name.into(),
            status: 5,
            value,
        }
    }
    fn world() -> M {
        M::from_columns([
            V {
                xyz: [1.0, 0.0, 0.0],
                w: None,
            },
            V::known([0.0, 1.0, 0.0, 0.0]),
            V::known([0.0, 0.0, 1.0, 0.0]),
            V::known([100.0; 4]),
        ])
    }
    #[test]
    fn air_foot_vector_requires_state_change_and_uses_mirrored_foot_without_translation() {
        let mut trajectory = AnimationLocomotionTrajectory::new();
        let attr = row(
            "DCOMFOOT",
            AndaleAttributeValue::Vector([2.0, 3.0, 4.0, f32::NAN]),
        );
        let before = trajectory.input_feet[1].states[2].vectors[0];
        trajectory
            .try_ingest_locomotion_attribute(&attr, true, false, &world(), || Err(()))
            .unwrap();
        assert_eq!(trajectory.input_feet[1].states[2].vectors[0], before);
        trajectory
            .try_ingest_locomotion_attribute(&attr, true, true, &world(), || Err(()))
            .unwrap();
        assert_eq!(
            trajectory.input_feet[1].states[2].vectors[0],
            V {
                xyz: [2.0, 3.0, 4.0],
                w: None
            }
        );
    }
    #[test]
    fn serialized_vector_names_reach_the_same_trajectory_fields_as_source_names() {
        use skate2_formats::andale::PackedFastString;
        let mut trajectory = AnimationLocomotionTrajectory::new();
        for (index, name) in ["DCOMFOOT", "SCOMSTART", "DCOMSTARTEND", "SCOMEND"].iter().enumerate() {
            let decoded = PackedFastString::<5>::encode(name).unwrap().decode().unwrap();
            assert_ne!(decoded, *name);
            let value = [index as f32 + 1.0, 2.0, 3.0, 0.0];
            let mut attr = row(&decoded, AndaleAttributeValue::Vector(value));
            attr.sequence_id = 0;
            trajectory.try_ingest_locomotion_attribute(&attr, false, true, &world(), || Ok::<_, ()>(0.0)).unwrap();
            assert_eq!(trajectory.input_feet[0].states[0].vectors[index].xyz, [value[0], value[1], value[2]]);
        }
    }
    #[test]
    fn raw_nan_is_accepted_but_duration_nan_fails_before_store() {
        let mut trajectory = AnimationLocomotionTrajectory::new();
        let attr = row(
            "SequenceStateRawTime",
            AndaleAttributeValue::Float(f32::NAN),
        );
        trajectory
            .try_ingest_locomotion_attribute(&attr, false, true, &world(), || Ok::<_, ()>(0.02))
            .unwrap();
        assert!(trajectory.time.is_nan());
        let old = trajectory.input_feet[0].states[2].time;
        let attr = row("SequenceStateLength", AndaleAttributeValue::Float(f32::NAN));
        assert!(matches!(
            trajectory.try_ingest_locomotion_attribute(&attr, false, true, &world(), || Err(())),
            Err(CreateAttributeError::Duration)
        ));
        assert_eq!(trajectory.input_feet[0].states[2].time, old);
    }
}

impl AnimationLocomotionTrajectory {
    /// Scalar branch82cadafc..dc6c and vector branch82cad7b8..da34.
    /// The caller performs SetState/footDown/turn for the first nonnegative row
    /// before calling this method, preserving the source mutation prefix.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub(super) fn try_ingest_locomotion_attribute<E>(
        &mut self,
        attribute: &AndaleRuntimeAttribute,
        mirrored: bool,
        state_changed: bool,
        world: &M,
        maximum_raw_time: impl FnOnce() -> Result<f32, E>,
    ) -> Result<(), CreateAttributeError<E>> {
        use CreateAttributeError as Error;
        let id = attribute.sequence_id;
        if id == -1 {
            return Ok(());
        }
        let foot =
            usize::try_from((id / 3 + i32::from(mirrored)) % 4).map_err(|_| Error::Index(id))?;
        let state = usize::try_from(id % 3).map_err(|_| Error::Index(id))?;
        match &attribute.value {
            AndaleAttributeValue::Float(value) => {
                if attribute.name.eq_ignore_ascii_case("SequenceStateLength") {
                    // 82cadb04=41990018 branches to store only for GT.
                    if !(*value > 0.0) {
                        return Err(Error::Duration);
                    }
                    self.input_feet[foot].states[state].time = *value;
                }
                if state_changed && attribute.name.eq_ignore_ascii_case("SequenceStateRawTime") {
                    let maximum = maximum_raw_time().map_err(Error::Tuning)?;
                    let clamped = if *value - maximum >= 0.0 {
                        maximum
                    } else {
                        *value
                    };
                    // 82cadc54=40980018 accepts !LT, including unordered.
                    if clamped < 0.0 {
                        return Err(Error::RawTime);
                    }
                    self.time = clamped;
                }
            }
            AndaleAttributeValue::Vector(local) => {
                // Transform is evaluated for every vector row, even an ignored
                // name. Native multiplies X, then FMA Y, then FMA Z; no P term.
                let basis = world.columns();
                let x = basis[0].map(|v| v * local[0]);
                let xy = basis[1].zip(x, |v, x| v.mul_add(local[1], x));
                let transformed = basis[2].zip(xy, |v, xy| v.mul_add(local[2], xy));
                // Native compares FastString words, whose alphabet folds case.
                // Serialized clips decode to lowercase; generated sequence
                // attributes keep their source spelling. Both are equal natively.
                let index = match attribute.name.as_str() {
                    name if name.eq_ignore_ascii_case("DCOMFOOT") && (state_changed || state != 2) => Some(0),
                    name if name.eq_ignore_ascii_case("SCOMSTART") => Some(1),
                    name if name.eq_ignore_ascii_case("DCOMSTARTEND") => Some(2),
                    name if name.eq_ignore_ascii_case("SCOMEND") => {
                        return self
                            .try_set_speed_com_end(transformed, state as i32, foot as i32)
                            .map_err(Error::Trajectory);
                    }
                    _ => None,
                };
                if let Some(index) = index {
                    // Source duplicates XYZ lanes before self-equality compare;
                    // an unknown or NaN W does not fail this assertion.
                    if transformed.xyz.iter().any(|v| v.is_nan()) {
                        return Err(Error::Vector);
                    }
                    self.input_feet[foot].states[state].vectors[index] = transformed;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
