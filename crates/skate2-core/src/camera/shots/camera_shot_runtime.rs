//! Existing shot-manager runtime producer reached by `CameraMan.UpdateRig`.
use super::{
    blend_selection,
    blend_value::{self, BlendValueSource},
    field_interpolation, runtime_math,
    selection::{ShotSelectionError, name_bytes, name_text},
};
use crate::{
    camera_compass::runtime::math::{camera_angle, unit_fraction},
    camera_man::CameraMan,
    camera_shot::CameraShot,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum ShotUpdateError {
    OwnerLock,
    MissingSubject,
    CompassIndex(i32),
    UnknownReferenceName(usize),
    ChildOutsidePool,
    ChildCycle,
    Numeric(ReckoningHostNumericError),
    Selection(ShotSelectionError<ReckoningHostNumericError>),
    BlendSelection(blend_selection::BlendSelectionError),
    Math(runtime_math::ShotMathError<ReckoningHostNumericError, std::convert::Infallible>),
    Cvar(InputCvarError),
}

impl CameraMan {
    /// `ShotManager2.Update82e472c8`; the embedded manager is mutated directly.
    pub(crate) fn try_update_shots(
        &mut self,
        dt: f32,
        cvars: &InputCvarReferences,
    ) -> Result<(), ShotUpdateError> {
        use ShotUpdateError as E;
        let elapsed = dt + self.shots.time_since_transition;
        self.shots.time_since_transition = elapsed;
        let reload = cvars
            .try_camera_shot_database_live_changes()
            .map_err(E::Cvar)?
            || self
                .shots
                .database
                .lock()
                .map_err(|_| E::OwnerLock)?
                .changed;
        if reload {
            let name = name_bytes(&self.shots.current.name).map_err(E::Selection)?;
            self.try_set_shot(
                name_text(&name).map_err(E::Selection)?,
                true,
                &mut HostReckoningNumeric,
            )
            .map_err(E::Selection)?;
            self.shots.transition_out_time = -1.0;
            self.shots.time_since_transition =
                duration(&self.shots.current, self.shots.shot_distance);
            self.shots
                .database
                .lock()
                .map_err(|_| E::OwnerLock)?
                .changed = false;
        }
        let source = BlendValueSource {
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
        update_shot(
            &mut self.shots.current,
            dt,
            &source,
            &self.shots.blends,
            &mut Vec::new(),
        )?;
        let duration = duration(&self.shots.current, self.shots.shot_distance);
        // Raw47408 branches GT-clear: unordered duration selects one.
        let fraction = if duration > 0.0 {
            self.shots.time_since_transition / duration
        } else {
            1.0
        };
        field_interpolation::publish(
            &self.shots.previous,
            &self.shots.current,
            &mut self.shots.interpolated,
            unit_fraction(fraction),
            true,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
        .map_err(E::Math)
    }
}

fn duration(shot: &CameraShot, distance: f32) -> f32 {
    match shot.transition_units.native_value() {
        1 => shot.transition_time * f32::from_bits(0x3c88_8889),
        2 => distance / shot.transition_time,
        _ => shot.transition_time,
    }
}

fn update_shot(
    shot: &mut CameraShot,
    dt: f32,
    source: &BlendValueSource<'_>,
    pool: &[Arc<Mutex<CameraShot>>],
    ancestry: &mut Vec<usize>,
) -> Result<(), ShotUpdateError> {
    use ShotUpdateError as E;
    if shot.shot_type == 0 {
        if !shot.use_previous {
            let north = match shot.compass_north {
                0 | 1 | 3 => {
                    let subject = source
                        .subject
                        .ok_or(E::MissingSubject)?
                        .lock()
                        .map_err(|_| E::OwnerLock)?;
                    *subject
                        .compass_norths
                        .get(usize::try_from(shot.compass_north).expect("matched positive id"))
                        .ok_or(E::CompassIndex(shot.compass_north))?
                }
                _ => 0.0,
            };
            let mirror = source.rig.heading_mirror;
            let mut heading = shot.heading;
            let target = -shot.heading;
            let amount = (1.0 - mirror) * 0.5;
            crate::camera_compass::runtime::math::interpolate_angle(
                &mut heading,
                target,
                amount,
                &mut HostReckoningNumeric,
            )
            .map_err(E::Numeric)?;
            heading += north;
            heading = camera_angle(heading, &mut HostReckoningNumeric).map_err(E::Numeric)?;
            shot.world_arm_orientation = runtime_math::normal_orientation(
                shot.elevation,
                heading,
                &mut HostReckoningSinCosNumeric,
            )
            .unwrap_or_else(|never| match never {});
        }
        return Ok(());
    }
    let first_point = shot.blend_points[0];
    let mut last_point = first_point;
    for index in 0..3 {
        let first = shot.blend_names[index][0].ok_or(E::UnknownReferenceName(index))?;
        if first == 0 {
            break;
        }
        last_point = shot.blend_points[index];
    }
    let raw =
        blend_value::raw(source, shot, dt, &mut HostReckoningNumeric).map_err(E::Selection)?;
    shot.blend_value = blend_selection::smooth_value(
        raw,
        shot.blend_value,
        shot.blend_smoothing,
        first_point,
        last_point,
    );
    let selected = blend_selection::select(shot).map_err(E::BlendSelection)?;
    let amount = runtime_math::ease(selected.fraction, &mut HostReckoningSinCosNumeric)
        .unwrap_or_else(|never| match never {});
    // Native A and B can alias. Perform BOTH updates, ending each lock before
    // acquiring the other; afterward both source reads see the final payload.
    let Some((first, second)) = update_endpoints(selected, |child| {
        update_child(child, dt, source, pool, ancestry)
    })?
    else {
        return Ok(());
    };
    field_interpolation::publish(
        &first,
        &second,
        shot,
        amount,
        false,
        &mut HostReckoningNumeric,
        &mut HostReckoningSinCosNumeric,
    )
    .map_err(E::Math)
}

fn update_endpoints(
    selected: blend_selection::BlendSelection,
    mut update: impl FnMut(&Arc<Mutex<CameraShot>>) -> Result<(), ShotUpdateError>,
) -> Result<Option<(CameraShot, CameraShot)>, ShotUpdateError> {
    let Some(first) = selected.first else {
        return Ok(None);
    };
    update(&first)?;
    let Some(second) = selected.second else {
        return Ok(None);
    };
    update(&second)?;
    let first = first
        .lock()
        .map_err(|_| ShotUpdateError::OwnerLock)?
        .clone();
    let second = second
        .lock()
        .map_err(|_| ShotUpdateError::OwnerLock)?
        .clone();
    Ok(Some((first, second)))
}

fn update_child(
    child: &Arc<Mutex<CameraShot>>,
    dt: f32,
    source: &BlendValueSource<'_>,
    pool: &[Arc<Mutex<CameraShot>>],
    ancestry: &mut Vec<usize>,
) -> Result<(), ShotUpdateError> {
    let index = pool
        .iter()
        .position(|entry| Arc::ptr_eq(entry, child))
        .ok_or(ShotUpdateError::ChildOutsidePool)?;
    if ancestry.contains(&index) {
        return Err(ShotUpdateError::ChildCycle);
    }
    ancestry.push(index);
    let result = update_shot(
        &mut *child.lock().map_err(|_| ShotUpdateError::OwnerLock)?,
        dt,
        source,
        pool,
        ancestry,
    );
    ancestry.pop();
    result
}

impl std::fmt::Display for ShotUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("shot runtime owner lock"),
            Self::MissingSubject => f.write_str("shot runtime Subject missing"),
            Self::CompassIndex(index) => write!(f, "Subject compass index {index}"),
            Self::UnknownReferenceName(index) => write!(f, "unknown shot reference name {index}"),
            Self::ChildOutsidePool => f.write_str("shot child is not in retained pool"),
            Self::ChildCycle => f.write_str("shot child cycle"),
            Self::Numeric(error) => write!(f, "{error}"),
            Self::Selection(error) => write!(f, "{error}"),
            Self::BlendSelection(error) => write!(f, "{error}"),
            Self::Math(error) => write!(f, "{error}"),
            Self::Cvar(error) => write!(f, "{error}"),
        }
    }
}

#[cfg(test)]
#[path = "camera_shot_runtime_tests.rs"]
mod tests;
