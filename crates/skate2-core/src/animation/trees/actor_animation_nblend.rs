//! ACS `NBlend82a26348` and `ACSsqtBlend_job` case2, on the existing owners.
//! Checked host failures stop the caller; native error recovery/EndCommands
//! rewind is not claimed by this admitted path.
use super::{ActorFaceError, AndaleSqt, BindPoseError, Command, TreeCommands};

#[cfg(test)]
#[path = "actor_animation_nblend_tests.rs"]
mod tests;

#[derive(Default)]
pub(crate) struct NBlendState {
    pub(crate) maximum_lists: usize,
    pub(crate) list_count: usize,
    pub(crate) maximum_other: u16,
    pub(crate) other_count: u16,
    pub(crate) maximum_commands: usize,
    pub(crate) error: u32,
}
impl NBlendState {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "admitted native descriptors use f32 conversion/fctidz then low16 usage limit"
    )]
    pub(crate) fn configure(&mut self, setup: &crate::actor_face_playback::AnimationCommandSetup) {
        self.maximum_lists = (setup.max_nblend * setup.characters) as usize;
        self.maximum_other = ((((setup.capacities[0] - setup.capacities[1]) as f32
            - setup.averages[2])
            * setup.characters as f32) as i64 as u64
            & 0xffff) as u16;
        self.maximum_commands =
            ((setup.capacities[0] + setup.capacities[2] + setup.max_nblend + setup.capacities[4])
                * setup.characters) as usize;
    }
    pub(crate) fn begin_commands(&mut self) {
        self.error = 0;
    }
    pub(crate) fn end_command_set(&mut self) {
        self.other_count = 0;
    }
    pub(crate) fn synchronized(&mut self) {
        self.list_count = 0;
    }
    pub(crate) fn count_other(&mut self, commands: usize) {
        self.other_count = self.other_count.wrapping_add(1);
        if self.other_count > self.maximum_other || commands >= self.maximum_commands {
            self.error |= 128;
        }
    }
    fn reserve_list(&mut self) -> Option<usize> {
        // Source accepts equality, publishes count, then would read an invalid
        // row. Preserve count but reject missing host storage at its use.
        if self.list_count <= self.maximum_lists {
            let index = self.list_count;
            self.list_count += 1;
            Some(index)
        } else {
            self.error |= 64;
            None
        }
    }
}

#[derive(Clone)]
pub(crate) struct NBlendRecord {
    pub(crate) native_index: u16,
    pub(crate) inputs: Vec<u16>,
    pub(crate) coefficients: Vec<f32>,
}

#[expect(
    clippy::assign_op_pattern,
    reason = "retain native coefficient-before-sum operand order"
)]
fn coefficient_errors(values: &[f32]) -> u32 {
    let epsilon = f32::from_bits(0x38d1_b717);
    let upper = f32::from_bits(0x3f80_0347);
    let mut errors = u32::from(values.len() >= 6) * 4;
    let mut sum = 0.0;
    for &value in values {
        if (value + epsilon)
            .partial_cmp(&0.0)
            .is_none_or(std::cmp::Ordering::is_lt)
            || (upper - value)
                .partial_cmp(&0.0)
                .is_none_or(std::cmp::Ordering::is_lt)
        {
            errors |= 4;
        }
        sum = value + sum;
    }
    if (upper - sum)
        .partial_cmp(&0.0)
        .is_none_or(std::cmp::Ordering::is_lt)
        || ((sum + epsilon) - 1.0)
            .partial_cmp(&0.0)
            .is_none_or(std::cmp::Ordering::is_lt)
    {
        errors |= 4;
    }
    errors
}

impl TreeCommands {
    pub(crate) fn count_other_sqt(&self) -> Result<(), BindPoseError> {
        let mut owner = self.owner.lock().map_err(|_| ActorFaceError::Lock)?;
        let commands = owner.commands.len();
        owner.nblend.count_other(commands);
        // Existing producers keep their current checked failure boundaries;
        // NBlend observes this actual accumulated usage/error state.
        Ok(())
    }
    pub(crate) fn emit_n_blend(&mut self, coefficients: &[f32]) -> Result<(), BindPoseError> {
        let (native_index, maximum_lists) = {
            let mut owner = self.owner.lock().map_err(|_| ActorFaceError::Lock)?;
            let count = owner.commands.len();
            let active = owner.in_commands != 0 && owner.in_command_set != 0;
            let state = &mut owner.nblend;
            state.error |= coefficient_errors(coefficients);
            if !active || coefficients.len() > count {
                state.error |= 2;
            }
            state.count_other(count);
            let index = state.reserve_list();
            if state.error != 0 {
                return Err(BindPoseError::NBlend(state.error));
            }
            (
                index.ok_or(BindPoseError::UnsupportedSetup)?,
                state.maximum_lists,
            )
        };
        if native_index >= maximum_lists {
            return Err(BindPoseError::NBlendListBounds);
        }
        let native_index =
            u16::try_from(native_index).map_err(|_| BindPoseError::NBlendListBounds)?;
        if native_index > 0x0fff {
            return Err(BindPoseError::NBlendListBounds);
        }
        // Native copies coefficients BEFORE reverse stack pops.
        self.nblends.push(NBlendRecord {
            native_index,
            inputs: vec![0; coefficients.len()],
            coefficients: coefficients.to_vec(),
        });
        let record = self.nblends.last_mut().expect("retained coefficient list");
        let mut reusable = None;
        for index in (0..coefficients.len()).rev() {
            let input = self
                .stack
                .pop()
                .ok_or(BindPoseError::TreeStackUnderflow("NBlend input"))?;
            record.inputs[index] = input;
            if input & super::fixed::FIXED_SQT_BIT == 0 {
                reusable = Some(index);
            }
        }
        let pending = record.inputs.clone();
        let prior_segments = self.reuse.segments.len();
        let output = match reusable {
            Some(index) if index > 0 => record.inputs[index],
            _ => self.allocate_dynamic()?,
        };
        let count =
            u16::try_from(coefficients.len()).map_err(|_| BindPoseError::UnsupportedSetup)?;
        let command = Command::NBlend {
            list: native_index,
            count,
            output,
        };
        self.owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .commands
            .push(command);
        self.operations.push(command);
        // NBlend82a26774 stores input IDs BEFORE GetNextLoc82a267a0.
        // Keep those encoded IDs; only graph-root consumption follows the
        // relocated output table (native graph links are set at82a26918 onward).
        let consumed: Vec<u16> = if self.reuse.segments.len() != prior_segments {
            pending
                .iter()
                .map(|input| {
                    self.reuse.last_remap.iter()
                        .find(|(_, source)| source == input)
                        .map_or(*input, |(destination, _)| *destination)
                })
                .collect()
        } else {
            pending
        };
        self.produced(&consumed, output);
        self.stack.push(output);
        Ok(())
    }

    pub(crate) fn execute_n_blend(
        &self,
        list: u16,
        count: u16,
        output: u16,
        dynamic: &mut [Option<Vec<AndaleSqt>>],
        bones: usize,
    ) -> Result<(), BindPoseError> {
        let record = self
            .nblends
            .iter()
            .find(|r| r.native_index == list)
            .ok_or(BindPoseError::NBlendListBounds)?;
        if record.inputs.len() != usize::from(count)
            || record.coefficients.len() != usize::from(count)
            || count == 0
        {
            return Err(BindPoseError::UnsupportedSetup);
        }
        let trace_inputs = if std::env::var_os("SKATE2_TRACE_ADJUSTED_POSE").is_some() {
            Some(
                record
                    .inputs
                    .iter()
                    .map(|&input| {
                        self.operand(input, dynamic)
                            .map(|pose| (input, pose.first().copied()))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )
        } else {
            None
        };
        // Host storage-safety deviation: the native CPU worker writes the first
        // weighted pose before reading later operands, even when its selected
        // output aliases one of them. Preserve that one operand so [0,1,0,0]
        // does not destroy its only contributing pose. Arithmetic/pass order is
        // unchanged. See city-moving-view-2026-09-25.md for the source audit.
        let preserved_output = if record.inputs.iter().skip(1).any(|&id| id == output) {
            Some(self.operand(output, dynamic)?.into_owned())
        } else {
            None
        };
        for (pass, (&input, &coefficient)) in
            record.inputs.iter().zip(&record.coefficients).enumerate()
        {
            for bone in 0..bones {
                let source_pose = if input == output {
                    match &preserved_output {
                        Some(pose) => std::borrow::Cow::Borrowed(pose.as_slice()),
                        None => self.operand(input, dynamic)?,
                    }
                } else {
                    self.operand(input, dynamic)?
                };
                let source = *source_pose
                    .get(bone)
                    .ok_or(BindPoseError::InvalidHierarchy)?;
                let slot = dynamic
                    .get_mut(usize::from(output))
                    .ok_or(BindPoseError::UnsupportedSetup)?;
                if pass == 0 && bone == 0 && slot.is_none() {
                    *slot = Some(Vec::new());
                }
                let values = slot.as_mut().ok_or(BindPoseError::MissingBindPose)?;
                if pass == 0 && values.len() == bone {
                    values.push(AndaleSqt {
                        scale: source.scale.map(|v| coefficient * v),
                        rotation: source.rotation.map(|v| coefficient * v),
                        translation: source.translation.map(|v| coefficient * v),
                    });
                    continue;
                }
                let destination = values
                    .get_mut(bone)
                    .ok_or(BindPoseError::InvalidHierarchy)?;
                if pass == 0 {
                    destination.scale = source.scale.map(|v| coefficient * v);
                    destination.rotation = source.rotation.map(|v| coefficient * v);
                    destination.translation = source.translation.map(|v| coefficient * v);
                } else {
                    for lane in 0..4 {
                        destination.scale[lane] =
                            coefficient.mul_add(source.scale[lane], destination.scale[lane]);
                        destination.rotation[lane] =
                            coefficient.mul_add(source.rotation[lane], destination.rotation[lane]);
                        destination.translation[lane] = coefficient
                            .mul_add(source.translation[lane], destination.translation[lane]);
                    }
                }
            }
        }
        let destination = dynamic
            .get_mut(usize::from(output))
            .and_then(Option::as_mut)
            .ok_or(BindPoseError::MissingBindPose)?;
        for bone in destination.iter_mut().take(bones) {
            bone.rotation = super::numeric::normalize(bone.rotation);
        }
        if let Some(inputs) = trace_inputs {
            if destination
                .iter()
                .take(bones)
                .any(|bone| bone.rotation.iter().any(|v| !v.is_finite()))
            {
                eprintln!(
                    "nonfinite NBlend: list={list} output={output} coefficients={:?} inputs={inputs:?}",
                    record.coefficients
                );
            }
        }
        Ok(())
    }
}
