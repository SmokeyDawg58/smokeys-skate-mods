//! Retained `SkeletonDrives::UpdateRagdoll` 0x82b546c0 and flag4 dynamics.
use super::update::{
    LOCAL_SCALE_KEY, ROOT_SCALE_KEY, SkeletonDriveUpdatePayload, SkeletonDrivesUpdateAttributeRead,
    SkeletonDrivesUpdateCvars, SkeletonDrivesUpdateError as Error, update_dynamics, update_frames,
};
use super::{
    DISPLACEMENT_SCALE, STRENGTH_SCALE, SimulationDriveDynamicsParams, SkeletonBoneDrives,
    SkeletonDriveNumeric, SkeletonDriveType, SkeletonDrives,
};
use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_full4},
    skeleton_attributes::SkeletonDriveAttributes,
    skeleton_root_transforms::SkeletonRootVector,
    skeleton_two_bone::try_inverse_basis_from_cofactors,
};

pub(crate) const ROOT_WEIGHT_KEYS: [u64; 4] = [
    0x8f84_492b_5338_adc0,
    0x0b97_4171_ba60_1791,
    0x9661_de4e_0288_7b77,
    0x1272_733f_2cf1_b5dc,
];

pub(crate) trait RagdollDriveAttributes: SkeletonDrivesUpdateAttributeRead {
    fn lookup_skeleton_drive(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
}
pub(crate) trait RagdollDriveCvars: SkeletonDrivesUpdateCvars {
    fn follow_animation(&mut self) -> Result<bool, Self::Error>;
    fn rigid_ragdoll(&mut self) -> Result<bool, Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct RagdollDriveWeights {
    pub start: f32,
    pub end: f32,
    pub controlled: f32,
}

#[derive(Debug)]
pub(crate) enum RagdollDriveUpdateError<A, C, N, R> {
    Drive(Error<A, C, N>),
    Inverse(R),
}
impl<A, C, N, R> From<Error<A, C, N>> for RagdollDriveUpdateError<A, C, N, R> {
    fn from(value: Error<A, C, N>) -> Self {
        Self::Drive(value)
    }
}

impl SkeletonDrives {
    /// Inverses1..23 precede tuning; drive rows1..22 retain their admitted payloads.
    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::type_complexity,
        reason = "source-ordered partial stores across distinct retained inputs and numeric services"
    )]
    pub(crate) fn try_update_ragdoll<
        A: RagdollDriveAttributes,
        C: RagdollDriveCvars,
        N: SkeletonDriveNumeric,
        R: ReckoningNumeric,
    >(
        &mut self,
        targets: &[[f32; 16]; 24],
        coefficients: &[SkeletonDriveAttributes; 24],
        weights: RagdollDriveWeights,
        attributes: &mut A,
        cvars: &mut C,
        numeric: &mut N,
        inverse_numeric: &mut R,
    ) -> Result<(), RagdollDriveUpdateError<A::Error, C::Error, N::Error, R::Error>> {
        let mut inverses = [None; 24];
        for part in 1..24 {
            inverses[part] = Some(
                inverse(&targets[part], inverse_numeric)
                    .map_err(RagdollDriveUpdateError::Inverse)?,
            );
        }
        let pointer = attributes
            .lookup_global(LOCAL_SCALE_KEY)
            .map_err(Error::<A::Error, C::Error, N::Error>::Attribute)?;
        let local_scale = attributes
            .float(&pointer)
            .map_err(Error::<A::Error, C::Error, N::Error>::Attribute)?;
        let pointer = attributes
            .lookup_global(ROOT_SCALE_KEY)
            .map_err(Error::<A::Error, C::Error, N::Error>::Attribute)?;
        let root_scale = attributes
            .float(&pointer)
            .map_err(Error::<A::Error, C::Error, N::Error>::Attribute)?;
        let ordinary = ((1.0 - weights.start) - weights.end) - weights.controlled;
        self.ragdoll_strength = ordinary;
        for (part, coefficient) in coefficients.iter().enumerate().take(23).skip(1) {
            let bone = &mut self.bones[part];
            bone.flags = 4;
            let coefficient = *coefficient;
            let values = [
                coefficient.ragdoll_strength_start,
                coefficient.ragdoll_strength,
                coefficient.ragdoll_strength_controlled,
                coefficient.ragdoll_strength_end,
            ];
            let mut root_values = [0.0; 4];
            for index in 0..4 {
                let pointer = attributes
                    .lookup_skeleton_drive(ROOT_WEIGHT_KEYS[index])
                    .map_err(Error::<A::Error, C::Error, N::Error>::Attribute)?;
                root_values[index] = attributes
                    .float(&pointer)
                    .map_err(Error::<A::Error, C::Error, N::Error>::Attribute)?
                    * values[index];
            }
            // 54ab4..54ae4: separate products followed by the actual fused sums.
            let local = values[3] * weights.end;
            let local = values[1].mul_add(ordinary, local);
            let local = values[0].mul_add(weights.start, local);
            let local = values[2].mul_add(weights.controlled, local) * local_scale;
            let root = root_values[1] * ordinary;
            let root = root_values[3].mul_add(weights.end, root);
            let root = root_values[0].mul_add(weights.start, root);
            let root = root_values[2].mul_add(weights.controlled, root) * root_scale;
            if self.data.parents()[part].is_none() {
                return Err(Error::<A::Error, C::Error, N::Error>::MissingParent { part }.into());
            }
            let child = bone
                .part
                .ok_or(Error::<A::Error, C::Error, N::Error>::MissingBonePart { part })?;
            if child.index() == 0 {
                continue;
            }
            let active_local = bone.active[0];
            let child_pose = targets[child.index()];
            let child_inverse = inverses[child.index()].expect("nonzero Part has inverse");
            bone.strengths = [local, root];
            for kind in [SkeletonDriveType::Local, SkeletonDriveType::LocalRoot] {
                if !(if kind == SkeletonDriveType::Local {
                    active_local
                } else {
                    bone.active[1]
                }) {
                    continue;
                }
                let parent =
                    bone.parent_parts[kind.index()].ok_or(
                        Error::<A::Error, C::Error, N::Error>::MissingDriveParent { part, kind },
                    )?;
                let inverse_parent =
                    inverses[parent.index()].as_ref().ok_or(Error::<
                        A::Error,
                        C::Error,
                        N::Error,
                    >::UninitializedInverse {
                        part,
                        kind,
                        parent: parent.index(),
                    })?;
                update_frames(
                    child,
                    kind,
                    bone,
                    &child_pose,
                    &child_inverse,
                    inverse_parent,
                    numeric,
                )?;
                ragdoll_dynamics::<A, C, N::Error>(
                    part,
                    kind,
                    bone,
                    if kind == SkeletonDriveType::Local {
                        local
                    } else {
                        root
                    },
                    attributes,
                    cvars,
                )?;
            }
        }
        Ok(())
    }
}

/// Cofactor inverse82b54724..54810, retaining the native duplicate-Y basis W.
fn inverse<N: ReckoningNumeric>(m: &[f32; 16], numeric: &mut N) -> Result<[f32; 16], N::Error> {
    let x = [m[0], m[1], m[2], m[3]];
    let y = [m[4], m[5], m[6], m[7]];
    let z = [m[8], m[9], m[10], m[11]];
    let cofactors =
        [cross_full4(y, z), cross_full4(z, x), cross_full4(x, y)].map(SkeletonRootVector::known);
    let basis = try_inverse_basis_from_cofactors(SkeletonRootVector::known(x), cofactors, numeric)?;
    let mut result = [0.0; 16];
    for column in 0..3 {
        result[column * 4..column * 4 + 3].copy_from_slice(&basis[column].xyz);
        result[column * 4 + 3] = basis[column].w.expect("known cofactor lanes");
    }
    let negative = [m[12], m[13], m[14]].map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
    for lane in 0..4 {
        let first = negative[0] * result[lane];
        let second = negative[1].mul_add(result[4 + lane], first);
        result[12 + lane] = negative[2].mul_add(result[8 + lane], second);
    }
    Ok(result)
}

fn ragdoll_dynamics<A: RagdollDriveAttributes, C: RagdollDriveCvars, N>(
    part: usize,
    kind: SkeletonDriveType,
    bone: &SkeletonBoneDrives,
    strength: f32,
    attributes: &mut A,
    cvars: &mut C,
) -> Result<(), Error<A::Error, C::Error, N>> {
    if cvars.follow_animation().map_err(Error::Cvar)? {
        return update_dynamics(part, kind, bone, strength, attributes, cvars);
    }
    if cvars.rigid_ragdoll().map_err(Error::Cvar)? {
        let layout = attributes
            .capture_animation_layout(kind)
            .map_err(Error::Attribute)?;
        let strength = attributes
            .animation_float(&layout, 8)
            .map_err(Error::Attribute)?;
        let spring = attributes
            .animation_float(&layout, 12)
            .map_err(Error::Attribute)?;
        let mut dynamics = bone.dynamics[kind.index()]
            .write()
            .map_err(|_| Error::Poisoned {
                part: Some(part),
                kind,
                payload: SkeletonDriveUpdatePayload::Dynamics,
            })?;
        dynamics.angular.spring = spring * DISPLACEMENT_SCALE;
        dynamics.angular.damping = 0.0;
        dynamics.angular.raw_type = 2;
        dynamics.angular.strength = strength * STRENGTH_SCALE;
        return Ok(()); // Native rigid branch does not touch linear dynamics.
    }
    let angular_strength = bone.soft[3] * strength;
    let spring = bone.soft[2] * strength;
    let angular_strength = angular_strength * STRENGTH_SCALE;
    let damping = cvars.soft_damping().map_err(Error::Cvar)?;
    {
        let mut dynamics = bone.dynamics[kind.index()]
            .write()
            .map_err(|_| Error::Poisoned {
                part: Some(part),
                kind,
                payload: SkeletonDriveUpdatePayload::Dynamics,
            })?;
        dynamics.angular = SimulationDriveDynamicsParams {
            raw_type: 1,
            spring,
            damping,
            strength: angular_strength,
        };
    }
    // Raw BO4/BI25 branches when GT is clear, including NaN.
    let linear = if strength > 0.5 {
        let linear_strength = bone.soft[3] * strength;
        let linear_spring = bone.soft[2] * strength;
        let damping = cvars.soft_damping().map_err(Error::Cvar)?;
        SimulationDriveDynamicsParams {
            raw_type: 1,
            spring: linear_spring * 0.5,
            damping,
            strength: linear_strength * STRENGTH_SCALE,
        }
    } else {
        SimulationDriveDynamicsParams {
            raw_type: 1,
            spring: 0.0,
            damping: 0.0,
            strength: 0.0,
        }
    };
    let mut dynamics = bone.dynamics[kind.index()]
        .write()
        .map_err(|_| Error::Poisoned {
            part: Some(part),
            kind,
            payload: SkeletonDriveUpdatePayload::Dynamics,
        })?;
    dynamics.linear = linear;
    Ok(())
}

#[cfg(test)]
#[path = "skeleton_drives_ragdoll_update_tests.rs"]
mod tests;
