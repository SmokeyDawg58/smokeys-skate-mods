//! Shared setup/live subset of `GetBlendRawValue82e44898`.
use crate::{
    camera_drop::CameraGrindDetector,
    camera_man::preupdate::{animation_input_lean, trajectory::is_on_ground},
    camera_rig::CameraRig,
    camera_shot::CameraShot,
    camera_shot_manager::selection::ShotSelectionError,
    camera_subject::CameraSubject,
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::{Arc, Mutex};

pub(super) struct BlendValueSource<'a> {
    pub(super) rig: &'a CameraRig,
    pub(super) incline: f32,
    pub(super) subject: Option<&'a Arc<Mutex<CameraSubject>>>,
    pub(super) mirrored: bool,
    /// CameraMan+380 bit 0x20: the previous `UpdateTrajectoryData` on-ground result.
    pub(super) was_on_ground: bool,
    /// CameraMan+348 `mMaximumPredictorHeight` and +34c `mMaximumPredictorHeightTime`.
    pub(super) maximum_height: f32,
    pub(super) maximum_height_time: Option<f32>,
    /// CameraMan+324 `mInclineAngleAbs` and +338 `mGrindInclineAngle`.
    pub(super) absolute_incline: f32,
    pub(super) grind_incline: f32,
    /// ShotManager2+3e0 `mPreviousShot.mPositionDistance` (+3c).
    pub(super) previous_distance: f32,
    /// CameraMan+248 `mGrindDetector`.
    pub(super) grind: &'a Arc<Mutex<CameraGrindDetector>>,
}

/// `GetTrajectoryProgress82e3e3e8`: elapsed fraction of the raw trajectory end
/// time (not gated by validity); 1 on the ground, 0 without a positive apex.
fn trajectory_progress(source: &BlendValueSource<'_>, subject: &CameraSubject) -> f32 {
    if is_on_ground(subject, source.rig.subject_state) {
        return 1.0;
    }
    // e430/e434: only an ordered positive height continues.
    if source.maximum_height.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return 0.0;
    }
    let remaining = subject.trajectory_end_time - subject.trajectory_elapsed;
    let end = subject.trajectory_end_time;
    // e4a0/e4a8: BLE, including unordered, selects one.
    let progress = if end > 0.0 {
        (end - remaining) / end
    } else {
        1.0
    };
    // e4c0/e4c8: fsel clamp to [0, 1].
    let lower = if -progress >= 0.0 { 0.0 } else { progress };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

/// `GetTrajectoryProgressWarped82e3e4f0`: 0..1 up to twice the apex time, then
/// 1..2 towards the predicted collision. On the ground it reports 0 while the
/// previous update was also grounded, otherwise 1.
fn trajectory_progress_warped<E>(
    source: &BlendValueSource<'_>,
    subject: &CameraSubject,
) -> Result<f32, ShotSelectionError<E>> {
    if is_on_ground(subject, source.rig.subject_state) {
        return Ok(if source.was_on_ground { 0.0 } else { 1.0 });
    }
    // e554/e558: only an ordered positive height continues.
    if source.maximum_height.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return Ok(0.0);
    }
    let apex = source
        .maximum_height_time
        .ok_or(ShotSelectionError::MissingApexTime)?;
    // e570/e574: BLT returns zero; unordered continues.
    if subject.trajectory_collision_time() < apex {
        return Ok(0.0);
    }
    let twice = apex * 2.0;
    let remaining = subject.trajectory_collision_time() - twice;
    let elapsed = subject.trajectory_elapsed;
    Ok(if elapsed > twice {
        let fraction = (elapsed - twice) / remaining + 1.0;
        // e5d0/e5d8: fsel pair, so unordered values select their negative arms.
        let lower = if 1.0 - fraction >= 0.0 { 1.0 } else { fraction };
        if 2.0 - lower >= 0.0 { lower } else { 2.0 }
    } else {
        elapsed / twice
    })
}

#[expect(
    clippy::neg_multiply,
    reason = "native speed sign branch multiplies negative one"
)]
pub(super) fn raw<N: ReckoningNumeric>(
    source: &BlendValueSource<'_>,
    shot: &CameraShot,
    dt: f32,
    numeric: &mut N,
) -> Result<f32, ShotSelectionError<N::Error>> {
    if shot.shot_type != 1 {
        return Err(ShotSelectionError::BlendTypeAssertion(shot.shot_type));
    }
    Ok(match shot.blend_type {
        0 => shot.blend_value,
        1 => {
            let speed = source
                .rig
                .try_subject_speed(numeric)
                .map_err(ShotSelectionError::Speed)?;
            if speed < 0.0 { speed * -1.0 } else { speed }
        }
        5 => {
            //82e44a18: CameraMan::GetTrajectoryProgress.
            let subject = source.subject.ok_or(ShotSelectionError::MissingSubject)?;
            let subject = subject.lock().map_err(|_| ShotSelectionError::OwnerLock)?;
            trajectory_progress(source, &subject)
        }
        6 => source.incline * f32::from_bits(0x4265_2ee1),
        //82e44b4c: the shot manager always exists here, so its previous shot is read.
        7 => source.previous_distance,
        9 => source.absolute_incline * f32::from_bits(0x4265_2ee1),
        20 => source.grind_incline * f32::from_bits(0x4265_2ee1),
        21 | 22 => {
            //82e44cc4 GetPaintedRegionBlendValue / 82e44cec negated GetSkitchGrabPosition.
            let subject = source.subject.ok_or(ShotSelectionError::MissingSubject)?;
            let subject = subject.lock().map_err(|_| ShotSelectionError::OwnerLock)?;
            if shot.blend_type == 21 {
                subject.painted_region_blend
            } else {
                -subject.skitch_grab_position
            }
        }
        8 => {
            //82e44a28: CameraMan::GetTrajectoryProgressWarped.
            let subject = source.subject.ok_or(ShotSelectionError::MissingSubject)?;
            let subject = subject.lock().map_err(|_| ShotSelectionError::OwnerLock)?;
            trajectory_progress_warped::<N::Error>(source, &subject)?
        }
        16 => {
            let subject = source.subject.ok_or(ShotSelectionError::MissingSubject)?;
            let subject = subject.lock().map_err(|_| ShotSelectionError::OwnerLock)?;
            subject.trajectory_collision_time()
        } //82e44bf0: queried at setup (dt=0) and during live shot update
        13 if dt != 0.0 => {
            let subject = source.subject.ok_or(ShotSelectionError::MissingSubject)?;
            let subject = subject.lock().map_err(|_| ShotSelectionError::OwnerLock)?;
            animation_input_lean(&subject, source.mirrored)
        }
        13 | 14 if dt == 0.0 => 0.0,
        //82e44bc0: GrindDetector::GetGrindAnticipation (vtable+1c), the eased anticipation.
        14 => {
            source
                .grind
                .lock()
                .map_err(|_| ShotSelectionError::OwnerLock)?
                .shaped_anticipation
        }
        other => return Err(ShotSelectionError::PendingBlendValue(other)),
    })
}
