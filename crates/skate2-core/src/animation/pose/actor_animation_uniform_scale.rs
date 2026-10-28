//! UniformScale82a27038 and ACSsqtBlend_job case10 82a2c0d0..c23c.
use super::{ActorFaceError, AndaleHierarchy, AndaleSqt, BindPoseError, Command, TreeCommands};
impl TreeCommands {
    pub(crate) fn uniform_scale(&mut self, scale: f32, scale_trajectory: bool, root_bone: u8)
        -> Result<(), BindPoseError> {
        self.count_other_sqt()?;
        if !scale.is_finite() || !(0.0..=10.0).contains(&scale) {
            return Err(BindPoseError::TreeCoefficientBits(scale.to_bits()));
        }
        let input = self.stack.pop().ok_or(BindPoseError::TreeStackUnderflow("UniformScale"))?;
        let output = if input & super::fixed::FIXED_SQT_BIT != 0 { self.allocate_dynamic()? } else { input };
        let coefficient = ((scale * 819.09998_f32).floor() as u32 & 0x1fff) as u16;
        let command = Command::UniformScale { input, output, coefficient, scale_trajectory, root_bone };
        self.owner.lock().map_err(|_| ActorFaceError::Lock)?.commands.push(command);
        self.operations.push(command);
        self.produced(&[input], output);
        self.stack.push(output);
        Ok(())
    }
}
pub(super) fn execute(input: &[AndaleSqt], hierarchy: &AndaleHierarchy,
    coefficient: u16, scale_trajectory: bool, root_bone: u8) -> Result<Vec<AndaleSqt>, BindPoseError> {
    if input.len() != hierarchy.parents.len() { return Err(BindPoseError::InvalidHierarchy); }
    // Original decode is coefficient * 0x3aa00500, including scale1's
    // quantization. All four scale/translation lanes participate.
    let scale = f32::from(coefficient) * f32::from_bits(0x3aa0_0500);
    let mut output = input.to_vec();
    for (index, value) in output.iter_mut().enumerate() {
        if index == 0 {
            if scale_trajectory { value.translation = input[index].translation.map(|v| v * scale); continue; }
        } else if index == usize::from(root_bone) {
            value.translation = input[index].translation.map(|v| v * scale);
            continue;
        }
        if hierarchy.parents[index] == i32::from(root_bone) {
            value.scale = input[index].scale.map(|v| v * scale);
            value.translation = input[index].translation.map(|v| v * scale);
        }
    }
    Ok(output)
}
