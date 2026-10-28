//! Actual `AttribShotDatabase::WriteToShot82d617a0` on its retained Vault.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, refspec::RefSpecError},
    camera_shot::{CameraShot, TransitionUnits},
    camera_shot_manager::CameraShotDatabase,
};
use skate2_formats::{attrib::string_hash64, state_graph::state_graph_string_hash32};

pub(crate) const CAMERA_SHOTS_CLASS: u64 = 0xf27d_d93e_059e_f6cb;
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);

#[derive(Debug)]
pub(crate) enum ShotAttributeError {
    Runtime(AttributeRuntimeError),
    Reference(RefSpecError),
    NameLengthAssertion(usize),
    NonAsciiName,
}
impl std::fmt::Display for ShotAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(error) => write!(f, "shot attribute: {error}"),
            Self::Reference(error) => write!(f, "shot reference: {error}"),
            Self::NameLengthAssertion(length) => {
                write!(f, "shot name length {length} exceeds native buffer")
            }
            Self::NonAsciiName => {
                f.write_str("non-ASCII native shot case conversion is not admitted")
            }
        }
    }
}
impl From<AttributeRuntimeError> for ShotAttributeError {
    fn from(value: AttributeRuntimeError) -> Self {
        Self::Runtime(value)
    }
}

pub(crate) fn terminated_bytes(value: &[u8]) -> &[u8] {
    &value[..value
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(value.len())]
}

/// `Shot.SetName` and `SetBlendShot` copy through the NUL, retaining the old tail.
pub(crate) fn copy_name(
    output: &mut [Option<u8>; 32],
    name: Option<&[u8]>,
) -> Result<(), ShotAttributeError> {
    let Some(name) = name else {
        output[0] = Some(0);
        return Ok(());
    };
    let name = terminated_bytes(name);
    if name.len() >= output.len() {
        return Err(ShotAttributeError::NameLengthAssertion(name.len()));
    }
    for (slot, &byte) in output.iter_mut().zip(name) {
        *slot = Some(byte);
    }
    output[name.len()] = Some(0);
    Ok(())
}

struct Layout<'a>(&'a AttributeRuntimeInstance);
impl Layout<'_> {
    fn word(&self, offset: usize) -> Result<u32, ShotAttributeError> {
        Ok(self
            .0
            .try_cached_layout_field(offset, 4)?
            .map_or(0, |value| {
                u32::from_be_bytes(value.bytes().try_into().expect("four bytes"))
            }))
    }
    fn float(&self, offset: usize) -> Result<f32, ShotAttributeError> {
        self.word(offset).map(f32::from_bits)
    }
    fn integer(&self, offset: usize) -> Result<i32, ShotAttributeError> {
        self.word(offset)
            .map(|value| i32::from_ne_bytes(value.to_ne_bytes()))
    }
    fn boolean(&self, offset: usize) -> Result<bool, ShotAttributeError> {
        Ok(self
            .0
            .try_cached_layout_field(offset, 1)?
            .is_some_and(|data| data.bytes()[0] != 0))
    }
}

impl CameraShotDatabase {
    /// Collection absence leaves the destination entirely unchanged. Each field
    /// publication precedes the next potentially failing live source read.
    pub(crate) fn try_write_to_shot(
        &self,
        shot: &mut CameraShot,
        name: &str,
    ) -> Result<(), ShotAttributeError> {
        let source_name = terminated_bytes(name.as_bytes());
        if !source_name.is_ascii() {
            return Err(ShotAttributeError::NonAsciiName);
        }
        let name = source_name
            .iter()
            .map(u8::to_ascii_lowercase)
            .collect::<Vec<_>>();
        let name_text = std::str::from_utf8(&name).expect("checked ASCII");
        let instance = self
            .vault
            .instance(CAMERA_SHOTS_CLASS, string_hash64(name_text));
        if instance.is_null() {
            return Ok(());
        }
        copy_name(&mut shot.name, Some(&name))?;
        shot.name_hash = state_graph_string_hash32(&name);
        let layout = Layout(&instance);
        shot.shot_type = layout.integer(0x6c)?;
        shot.transition_time = layout.float(0x54)?;
        shot.transition_units = TransitionUnits::from_native(layout.integer(0x50)?);
        shot.distance = layout.float(0xa0)?;
        shot.lens_length = layout.float(0xd0)?;
        shot.smoothing_direction = layout.float(0x64)?;
        shot.smoothing_elevation = layout.float(0x60)?;
        shot.smoothing_yaw = layout.float(0x58)?;
        shot.smoothing_pitch = layout.float(0x5c)?;
        for (slot, offset) in shot
            .reference_weights
            .iter_mut()
            .zip([0x78, 0x74, 0x7c, 0x88, 0x8c, 0x70, 0x90, 0x80, 0x84])
        {
            *slot = layout.float(offset)?;
        }
        shot.board_offset = layout.float(0x94)?;
        shot.heading = layout.float(0x98)? * DEGREES_TO_RADIANS;
        shot.elevation = layout.float(0x9c)? * DEGREES_TO_RADIANS;
        shot.roll = layout.float(0xc8)? * DEGREES_TO_RADIANS;
        shot.yaw = layout.float(0xc4)? * DEGREES_TO_RADIANS;
        shot.pitch = layout.float(0xcc)? * DEGREES_TO_RADIANS;
        shot.follow_subject_in_air = layout.boolean(0xa9)?;
        shot.mirror_for_stance = layout.boolean(0xa8)?;
        shot.snap_to_reference = layout.boolean(0xa7)?;
        shot.use_previous = layout.boolean(0xa4)?;
        shot.use_drop_predictor = layout.boolean(0xa6)?;
        shot.use_free_stick = layout.boolean(0xa5)?;
        shot.blur_shot = layout.float(0xc0)?;
        shot.blur_transition = layout.float(0xbc)?;
        shot.subject_opacity = layout.float(0xb8)?;
        shot.collision_hint = layout.integer(0xb0)?;
        shot.anchor = layout.word(0xb4)?;
        shot.compass_north = layout.integer(0xac)?;
        for (destination, offset) in shot.blend_names.iter_mut().zip([0x30, 0x18, 0]) {
            let reference = instance
                .try_cached_layout_reference(offset)
                .map_err(ShotAttributeError::Reference)?;
            let name = reference
                .as_ref()
                .map(|reference| reference.try_cached_layout_text(0x48))
                .transpose()?
                .flatten();
            copy_name(
                destination,
                name.as_ref()
                    .map(crate::attribute_runtime::AttributeRuntimeData::bytes),
            )?;
        }
        shot.blend_points[0] = layout.float(0xe4)?;
        shot.blend_points[1] = layout.float(0xe0)?;
        shot.blend_points[2] = layout.float(0xdc)?;
        shot.blend_value = layout.float(0xd4)?;
        shot.blend_type = layout.integer(0xd8)?;
        shot.blend_smoothing = layout.float(0x68)?;
        shot.world_heading = layout.float(0x4c)? * DEGREES_TO_RADIANS;
        Ok(())
    }
}

#[cfg(test)]
#[path = "camera_shot_attributes_tests.rs"]
mod tests;
