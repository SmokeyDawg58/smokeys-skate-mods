//! Native `ChooseShot`, `SetShot` and ordered recursive blend admission.
use crate::{
    camera_man::CameraMan,
    camera_rig::speed::SubjectSpeedError,
    camera_shot::CameraShot,
    camera_shot_manager::attributes::{ShotAttributeError, copy_name, terminated_bytes},
    camera_shot_manager::{CameraShotDatabase, CameraShotManager},
    reckoning_numeric::{ReckoningNumeric, try_length},
};
use std::{
    cmp::Ordering,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub(crate) enum ShotSelectionError<E> {
    OwnerLock,
    MissingSubject,
    UnknownNameByte(usize),
    UnterminatedName,
    NonAsciiName,
    ChoiceStorageBoundary(u32),
    BlendCountUnknown,
    BlendCapacityAssertion(u32),
    BlendAliasBoundary,
    BlendTypeAssertion(i32),
    PendingBlendValue(i32),
    MissingApexTime,
    Attribute(ShotAttributeError),
    Speed(SubjectSpeedError<E>),
    Numeric(E),
}

pub(super) fn name_bytes<E>(value: &[Option<u8>; 32]) -> Result<Vec<u8>, ShotSelectionError<E>> {
    let mut result = Vec::new();
    for (index, &byte) in value.iter().enumerate() {
        let byte = byte.ok_or(ShotSelectionError::UnknownNameByte(index))?;
        if byte == 0 {
            return Ok(result);
        }
        result.push(byte);
    }
    Err(ShotSelectionError::UnterminatedName)
}

pub(super) fn name_text<E>(bytes: &[u8]) -> Result<&str, ShotSelectionError<E>> {
    if !bytes.is_ascii() {
        return Err(ShotSelectionError::NonAsciiName);
    }
    Ok(std::str::from_utf8(bytes).expect("checked ASCII"))
}

impl CameraShotManager {
    /// `ClearShotChoices82e3d798` does not clear retained choice payloads.
    pub(crate) fn clear_choices(&mut self) {
        self.choice_count = 0;
    }

    pub(crate) fn try_add_choice<E>(&mut self, name: &[u8]) -> Result<(), ShotSelectionError<E>> {
        let name = terminated_bytes(name);
        // Native asserts length before computing its unchecked destination.
        if name.len() >= 32 {
            return Err(ShotSelectionError::Attribute(
                ShotAttributeError::NameLengthAssertion(name.len()),
            ));
        }
        let destination = self
            .choices
            .get_mut(self.choice_count as usize)
            .ok_or(ShotSelectionError::ChoiceStorageBoundary(self.choice_count))?;
        copy_name(destination, Some(name)).map_err(ShotSelectionError::Attribute)?;
        self.choice_count += 1;
        Ok(())
    }
}

impl CameraMan {
    /// `MakeShotChoice82e46ee0` always selects the first choice; there is no RNG.
    pub(crate) fn try_make_shot_choice<N: ReckoningNumeric>(
        &mut self,
        transition_in: f32,
        transition_out: f32,
        numeric: &mut N,
    ) -> Result<(), ShotSelectionError<N::Error>> {
        if self.shots.choice_count == 0 {
            return Ok(());
        }
        let name = name_bytes(&self.shots.choices[0])?;
        self.try_set_shot(name_text(&name)?, false, numeric)?;
        if transition_in < 0.0 {
            if self.shots.transition_out_time.partial_cmp(&0.0) != Some(Ordering::Less) {
                self.shots.current.transition_time = self.shots.transition_out_time;
            }
        } else {
            self.shots.current.transition_time = transition_in;
        }
        self.shots.transition_out_time = if transition_out >= 0.0 {
            transition_out
        } else {
            -1.0
        };
        Ok(())
    }

    pub(crate) fn try_set_shot<N: ReckoningNumeric>(
        &mut self,
        name: &str,
        force: bool,
        numeric: &mut N,
    ) -> Result<(), ShotSelectionError<N::Error>> {
        let incoming = terminated_bytes(name.as_bytes());
        let _ = name_text::<N::Error>(incoming)?;
        if !force {
            let old = name_bytes(&self.shots.current.name)?;
            let _ = name_text::<N::Error>(&old)?;
            if incoming.eq_ignore_ascii_case(&old) {
                return Ok(());
            }
        }
        // operator=82e3d390 copies every represented field, but only shallowly
        // copies its three native child pointers. No mutex is held or recopied.
        self.shots.previous = self.shots.interpolated.clone();
        self.shots.previous.shot_type = 0;
        self.shots
            .database
            .lock()
            .map_err(|_| ShotSelectionError::OwnerLock)?
            .try_write_to_shot(&mut self.shots.current, name)
            .map_err(ShotSelectionError::Attribute)?;
        if self.shots.current.use_previous {
            let old = &self.shots.interpolated;
            let current = &mut self.shots.current;
            current.pitch = old.pitch;
            current.yaw = old.yaw;
            current.roll = old.roll;
            current.lens_length = old.lens_length;
            current.heading = old.heading;
            current.elevation = old.elevation;
            current.distance = old.distance;
            current.world_arm_orientation = old.world_arm_orientation;
        }
        if self.shots.instant_transition {
            self.shots.teleport();
        } else {
            self.shots.time_since_transition = 0.0;
        }
        if self.shots.current.shot_type == 1 {
            self.shots.active_blend_count = Some(0);
            let values = BlendValueSource {
                rig: &self.rig,
                incline: self.incline_angle,
                subject: self.subject.as_ref(),
                mirrored: self.flags & 2 != 0,
                was_on_ground: self.flags & 0x20 != 0,
                maximum_height: self.maximum_prediction_height,
                maximum_height_time: self.maximum_prediction_height_time,
                absolute_incline: self.absolute_incline_angle,
                grind_incline: self.grind_incline,
                previous_distance: self.shots.previous.distance,
                grind: &self.grind,
            };
            let database = Arc::clone(&self.shots.database);
            setup_blends(
                &mut self.shots.current,
                &self.shots.blends,
                &mut self.shots.active_blend_count,
                &database,
                &values,
                None,
                numeric,
            )?;
        }
        let position = self.rig.positioner.position;
        let desired = self.rig.desired_camera_position(
            self.shots.current.elevation,
            self.shots.current.heading,
            self.shots.current.distance,
        );
        let delta = crate::skeleton_root_transforms::SkeletonRootVector {
            xyz: std::array::from_fn(|i| desired.xyz[i] - position.xyz[i]),
            w: desired.w.zip(position.w).map(|(a, b)| a - b),
        };
        self.shots.shot_distance =
            try_length(delta, numeric).map_err(ShotSelectionError::Numeric)?;
        Ok(())
    }
}

use super::blend_value::{BlendValueSource, raw};

fn setup_blends<N: ReckoningNumeric>(
    shot: &mut CameraShot,
    pool: &[Arc<Mutex<CameraShot>>],
    count: &mut Option<u32>,
    database: &Arc<Mutex<CameraShotDatabase>>,
    values: &BlendValueSource<'_>,
    parent_index: Option<usize>,
    numeric: &mut N,
) -> Result<(), ShotSelectionError<N::Error>> {
    for slot in 0..3 {
        let first = shot.blend_names[slot][0].ok_or(ShotSelectionError::UnknownNameByte(0))?;
        if first == 0 {
            break;
        }
        let index = count.ok_or(ShotSelectionError::BlendCountUnknown)?;
        if index >= 64 {
            return Err(ShotSelectionError::BlendCapacityAssertion(index));
        }
        let child = Arc::clone(&pool[index as usize]);
        // Source allocation is monotonically increasing: each new descendant
        // has a larger pool index. Refuse malformed host aliases before locking.
        if parent_index.is_some_and(|parent| index as usize <= parent) {
            return Err(ShotSelectionError::BlendAliasBoundary);
        }
        *count = Some(index + 1);
        shot.blend_shots[slot] = Some(Arc::clone(&child));
        let mut child = child.lock().map_err(|_| ShotSelectionError::OwnerLock)?;
        let name = name_bytes(&shot.blend_names[slot])?;
        database
            .lock()
            .map_err(|_| ShotSelectionError::OwnerLock)?
            .try_write_to_shot(&mut child, name_text(&name)?)
            .map_err(ShotSelectionError::Attribute)?;
        if child.shot_type == 1 {
            setup_blends(
                &mut child,
                pool,
                count,
                database,
                values,
                Some(index as usize),
                numeric,
            )?;
        }
    }
    shot.blend_value = raw(values, shot, 0.0, numeric)?;
    Ok(())
}

#[cfg(test)]
#[path = "camera_shot_selection_tests.rs"]
mod tests;

impl<E: std::fmt::Display> std::fmt::Display for ShotSelectionError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("shot owner lock"),
            Self::MissingSubject => f.write_str("shot Subject missing"),
            Self::UnknownNameByte(index) => write!(f, "unknown shot name byte {index}"),
            Self::UnterminatedName => f.write_str("unterminated native shot name"),
            Self::NonAsciiName => f.write_str("non-ASCII shot case conversion pending"),
            Self::ChoiceStorageBoundary(count) => {
                write!(f, "native unchecked choice storage boundary {count}")
            }
            Self::BlendCountUnknown => f.write_str("blend count has not been produced"),
            Self::BlendCapacityAssertion(count) => {
                write!(f, "native blend capacity assertion {count}")
            }
            Self::BlendAliasBoundary => f.write_str("blend pool descendant alias boundary"),
            Self::BlendTypeAssertion(kind) => {
                write!(f, "native blended shot assertion, type {kind}")
            }
            Self::PendingBlendValue(kind) => write!(f, "pending shot blend value type {kind}"),
            Self::MissingApexTime => f.write_str("shot blend needs an unwritten apex time"),
            Self::Attribute(error) => write!(f, "shot attributes: {error}"),
            Self::Speed(error) => write!(f, "shot speed: {error}"),
            Self::Numeric(error) => write!(f, "shot numeric: {error}"),
        }
    }
}
