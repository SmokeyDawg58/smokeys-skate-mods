//! Native `ShotManager2.GetBlendShots82e3d598` and blend-value clamp.
use crate::camera_shot::CameraShot;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum BlendSelectionError {
    UnknownReferenceName(usize),
    UnproducedSelection,
}

pub(super) struct BlendSelection {
    pub(super) first: Option<Arc<Mutex<CameraShot>>>,
    pub(super) second: Option<Arc<Mutex<CameraShot>>>,
    pub(super) fraction: f32,
}

/// Reads the parent's authored reference strings at B0/D0/F0. Equal
/// endpoints deliberately retain the same Arc for sequential native updates.
pub(super) fn select(shot: &CameraShot) -> Result<BlendSelection, BlendSelectionError> {
    let points = shot.blend_points;
    let mut count = 0;
    for (index, name) in shot.blend_names.iter().enumerate() {
        let first = name[0].ok_or(BlendSelectionError::UnknownReferenceName(index))?;
        if first == 0 {
            break;
        }
        count += 1;
    }
    if count == 0 {
        return Err(BlendSelectionError::UnproducedSelection);
    }
    let value = shot.blend_value;
    // Raw d644 is ordered BGT: unordered points/value continue to the next.
    let upper = points[..count].iter().position(|point| *point > value);
    let (a, b, fraction) = match upper {
        Some(0) => (0, 0, 1.0),
        Some(index) => {
            let difference = value - points[index - 1];
            let width = points[index] - points[index - 1];
            (index - 1, index, difference / width)
        }
        None => (count.saturating_sub(2), count - 1, 1.0),
    };
    Ok(BlendSelection {
        first: shot.blend_shots[a].clone(),
        second: shot.blend_shots[b].clone(),
        fraction,
    })
}

/// Native 45b90..45bb4: rounded square, FMADDS, then two FSEL gates.
pub(super) fn smooth_value(raw: f32, old: f32, smoothing: f32, low: f32, high: f32) -> f32 {
    let difference = raw - old;
    let weight = 1.0 - smoothing;
    let square = weight * weight;
    let value = square.mul_add(difference, old);
    let lower = if low - value >= 0.0 { low } else { value };
    if high - lower >= 0.0 { lower } else { high }
}

impl std::fmt::Display for BlendSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownReferenceName(index) => {
                write!(f, "reference name byte unknown at {index}")
            }
            Self::UnproducedSelection => {
                f.write_str("native empty blend selection scratch is unknown")
            }
        }
    }
}

#[cfg(test)]
#[path = "camera_shot_blend_selection_tests.rs"]
mod tests;
