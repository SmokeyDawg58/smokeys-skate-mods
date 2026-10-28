//! Registered SAME entry-state continuation, `UpdateSpeedModel` a2e0..aca4.
use super::{SkateboardEntryState, speed_model_prefix::SpeedModelBeforeJumpContribution};
use crate::{
    reckoning_numeric::ReckoningNumeric,
    reckoning_numeric::try_normalize_safe_with_refinement,
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
    skateboard_foot_forces::SkateboardFootForce,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpeedGlobal {
    Conservation,
    Manual,
    Friction,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpeedScalar {
    Balance,
    DeckSpeed,
    Spin,
    SurfaceSpeed,
    TimeSinceInput,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpeedVector {
    SurfaceVelocity,
    GroundNormal,
    /// a558/a56c: Processed+160 `mBodyZAxis`, not the flattened +170 direction.
    BodyZAxis,
    ProcessedEffectiveZ,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpeedGraph {
    IdleFriction,
    ManualFriction,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpeedCvar {
    ManualFrames,
    CheatEnabled,
    CheatSpeed,
    Debug,
}

/// Each method represents the actual source access; captures retain the selected
/// global/surface/cvar object across later reads. No snapshot or new model.
pub(crate) trait SpeedModelSource {
    type Error;
    type Pointer;
    type Surface;
    type Graph;
    type Cvar;
    fn flags_8c8(&mut self) -> Result<u32, Self::Error>;
    fn flags_8cc(&mut self) -> Result<u32, Self::Error>;
    fn attribute(&mut self, global: SpeedGlobal, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn value(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn effective_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
    fn vector(&mut self, field: SpeedVector) -> Result<SkeletonRootVector, Self::Error>;
    fn scalar(&mut self, field: SpeedScalar) -> Result<f32, Self::Error>;
    fn frames_spinning(&mut self) -> Result<i32, Self::Error>;
    fn capture_cvar(&mut self, field: SpeedCvar) -> Result<Self::Cvar, Self::Error>;
    fn cvar_int(&mut self, target: &Self::Cvar) -> Result<i32, Self::Error>;
    fn cvar_bool(&mut self, target: &Self::Cvar) -> Result<bool, Self::Error>;
    fn cvar_float(&mut self, target: &Self::Cvar) -> Result<f32, Self::Error>;
    fn capture_surface(&mut self) -> Result<Self::Surface, Self::Error>;
    fn surface_graph(&mut self, surface: &Self::Surface, speed: f32) -> Result<f32, Self::Error>;
    fn capture_graph(&mut self, graph: SpeedGraph) -> Result<Self::Graph, Self::Error>;
    fn evaluate_graph(&mut self, graph: &Self::Graph, speed: f32) -> Result<f32, Self::Error>;
    fn body_mass(&mut self) -> Result<f32, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SpeedModelError<S, N> {
    Source(S),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for SpeedModelError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "speed model source: {e:?}"),
            Self::Numeric(e) => write!(f, "speed model numeric: {e:?}"),
        }
    }
}
fn attribute<S: SpeedModelSource>(s: &mut S, g: SpeedGlobal, key: u64) -> Result<f32, S::Error> {
    let p = s.attribute(g, key)?;
    s.value(&p)
}
fn select(test: f32, yes: f32, no: f32) -> f32 {
    if test >= 0.0 { yes } else { no }
}
fn scale(v: SkeletonRootVector, s: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: v.xyz.map(|v| v * s),
        w: v.w.map(|v| v * s),
    }
}
fn subtract(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| a.xyz[i] - b.xyz[i]),
        w: a.w.zip(b.w).map(|(a, b)| a - b),
    }
}
fn flip(v: SkeletonRootVector) -> SkeletonRootVector {
    let f = |v: f32| f32::from_bits(v.to_bits() ^ 0x8000_0000);
    SkeletonRootVector {
        xyz: v.xyz.map(f),
        w: v.w.map(f),
    }
}

impl SkateboardEntryState {
    /// Full suffix after the earlier token. Errors after a860 preserve its target
    /// store; errors after aad0/aadc preserve the force initialization as well.
    pub(crate) fn try_speed_model_update<S: SpeedModelSource, N: ReckoningNumeric>(
        &mut self,
        prefix: SpeedModelBeforeJumpContribution,
        source: &mut S,
        numeric: &mut N,
        output: &mut SkateboardFootForce,
    ) -> Result<(), SpeedModelError<S::Error, N::Error>> {
        self.try_speed_model_update_with_refinement(
            prefix,
            source,
            numeric,
            output,
            refine_drive_reciprocal_sqrt,
        )
    }
    #[expect(
        clippy::too_many_lines,
        reason = "one source-ordered UpdateSpeedModel continuation"
    )]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "consume ordered continuation token"
    )]
    #[expect(
        clippy::assign_op_pattern,
        reason = "native floating-point operand order"
    )]
    pub(crate) fn try_speed_model_update_with_refinement<
        S: SpeedModelSource,
        N: ReckoningNumeric,
    >(
        &mut self,
        prefix: SpeedModelBeforeJumpContribution,
        source: &mut S,
        numeric: &mut N,
        output: &mut SkateboardFootForce,
        refine: impl FnOnce(f32, f32) -> f32,
    ) -> Result<(), SpeedModelError<S::Error, N::Error>> {
        use SpeedGlobal::{Conservation, Friction, Manual};
        use SpeedModelError as E;
        let dt = prefix.time_delta;
        let mut target = prefix.starting_target;
        let contribution = if source.flags_8c8().map_err(E::Source)? & 1 != 0 {
            attribute(source, Conservation, 0xb67e_7c6e_1758_9d9d).map_err(E::Source)? * dt
        } else {
            0.0
        };
        let effective = source.effective_transform().map_err(E::Source)?;
        let z = effective.columns()[2];
        let velocity = source
            .vector(SpeedVector::SurfaceVelocity)
            .map_err(E::Source)?;
        let forward = numeric.dot_vector(velocity, z).map_err(E::Numeric)?;
        let normal = source
            .vector(SpeedVector::GroundNormal)
            .map_err(E::Source)?;
        let projected = scale(z, forward);
        let normal_speed = numeric.dot_vector(projected, normal).map_err(E::Numeric)?;
        let direction = try_normalize_safe_with_refinement(
            subtract(projected, scale(normal, normal_speed)),
            SkeletonRootVector::known([0.0; 4]),
            numeric,
            refine,
        )
        .map_err(E::Numeric)?;
        let gravity = attribute(source, Conservation, 0x58ca_8124_b6ca_6d74).map_err(E::Source)?;
        let downhill = f32::from_bits(direction.xyz[1].to_bits() ^ 0x8000_0000) * gravity;
        let limit = attribute(source, Conservation, 0xf4eb_6567_aba4_d0f9).map_err(E::Source)?;
        let negative_limit = -limit;
        let max_frames = source
            .capture_cvar(SpeedCvar::ManualFrames)
            .map_err(E::Source)?;
        let frames = source.frames_spinning().map_err(E::Source)?;
        let low_test = negative_limit - downhill;
        let max_frames = source.cvar_int(&max_frames).map_err(E::Source)?;
        let bounded = select(low_test, negative_limit, downhill);
        let slope = select(limit - bounded, bounded, limit) * dt;
        if frames < max_frames {
            let torque = self.manual_torque;
            let balance = source.scalar(SpeedScalar::Balance).map_err(E::Source)?;
            let torque_active = if torque == 0.0 { 0.0 } else { 1.0 };
            // BO4/BI25: unordered balance takes the -1 branch.
            let mut sign = if balance > 0.0 { 1.0 } else { -1.0 };
            let body_z = source.vector(SpeedVector::BodyZAxis).map_err(E::Source)?;
            let alignment = numeric.dot_vector(body_z, direction).map_err(E::Numeric)?;
            if 0.0 > alignment {
                sign *= -1.0;
            }
            let negative_limit =
                attribute(source, Manual, 0xa98c_e2a9_19e8_0790).map_err(E::Source)?;
            let positive_limit =
                attribute(source, Manual, 0xb4ef_1a99_b05b_abf6).map_err(E::Source)?;
            let deck_speed = source
                .scalar(SpeedScalar::DeckSpeed)
                .map_err(E::Source)?
                .abs();
            // Both BLEs are NOT GT: unordered selects factor1.
            let positive = if deck_speed > positive_limit {
                0.0
            } else {
                1.0
            };
            let negative = if deck_speed > negative_limit {
                0.0
            } else {
                1.0
            };
            let spin = source.scalar(SpeedScalar::Spin).map_err(E::Source)?;
            let spin_squared = spin * spin;
            let coefficient =
                attribute(source, Manual, 0xb52d_8ff0_6890_b800).map_err(E::Source)?;
            let mut manual = ((coefficient * spin_squared) * positive) * sign;
            manual *= torque_active;
            if manual < 0.0 {
                manual *= negative;
            }
            target = manual + target;
        }
        let surface = source.capture_surface().map_err(E::Source)?;
        let speed = source
            .scalar(SpeedScalar::SurfaceSpeed)
            .map_err(E::Source)?;
        let friction = source.surface_graph(&surface, speed).map_err(E::Source)?;
        let mut change = (-friction) * dt;
        if slope > 0.0 {
            change += slope;
        } else {
            change = select(change - slope, slope, change);
        }
        change += contribution;
        target = change + target;
        let idle_limit = attribute(source, Friction, 0xa397_ebf0_1bdd_6c3c).map_err(E::Source)?;
        if source
            .scalar(SpeedScalar::TimeSinceInput)
            .map_err(E::Source)?
            > idle_limit
        {
            let graph = source
                .capture_graph(SpeedGraph::IdleFriction)
                .map_err(E::Source)?;
            let speed = source
                .scalar(SpeedScalar::SurfaceSpeed)
                .map_err(E::Source)?;
            let value = source.evaluate_graph(&graph, speed).map_err(E::Source)?;
            target = -(value.mul_add(dt, -target)); // fnmsubs a7f4
        }
        if source.scalar(SpeedScalar::Balance).map_err(E::Source)? != 0.0 {
            let graph = source
                .capture_graph(SpeedGraph::ManualFriction)
                .map_err(E::Source)?;
            let speed = source
                .scalar(SpeedScalar::SurfaceSpeed)
                .map_err(E::Source)?;
            let value = source.evaluate_graph(&graph, speed).map_err(E::Source)?;
            target = -(value.mul_add(dt, -target)); // fnmsubs a850
        }
        self.target_speed = select(target, target, 0.0); // a860, before cheat target read
        let negative_max = -prefix.maximum_difference;
        let cheat = source
            .capture_cvar(SpeedCvar::CheatEnabled)
            .map_err(E::Source)?;
        let difference = self.target_speed - prefix.absolute_forward_speed;
        let bounded = select(negative_max - difference, negative_max, difference);
        let upper_test = prefix.maximum_difference - bounded;
        let cheat = source.cvar_bool(&cheat).map_err(E::Source)?;
        let mut delta = select(upper_test, bounded, prefix.maximum_difference);
        if cheat && source.flags_8cc().map_err(E::Source)? & 0x0100_0000 != 0 {
            let normal = source
                .vector(SpeedVector::GroundNormal)
                .map_err(E::Source)?;
            if normal.xyz[1] > f32::from_bits(0x3f26_6666) {
                let velocity = source
                    .vector(SpeedVector::SurfaceVelocity)
                    .map_err(E::Source)?;
                let effective_z = source
                    .vector(SpeedVector::ProcessedEffectiveZ)
                    .map_err(E::Source)?;
                let speed = numeric
                    .dot_vector(velocity, effective_z)
                    .map_err(E::Numeric)?;
                let cheat = source
                    .capture_cvar(SpeedCvar::CheatSpeed)
                    .map_err(E::Source)?;
                let mut cheat =
                    source.cvar_float(&cheat).map_err(E::Source)? * f32::from_bits(0x3e8e_38e4);
                if 0.0 > speed {
                    let surface_speed = source
                        .scalar(SpeedScalar::SurfaceSpeed)
                        .map_err(E::Source)?;
                    if 1.0 > surface_speed {
                        cheat *= -1.0;
                    }
                }
                let difference = cheat - prefix.absolute_forward_speed;
                let lower = f32::from_bits(0xbecc_cccd);
                let upper = f32::from_bits(0x3ecc_cccd);
                let bounded = select(lower - difference, lower, difference);
                delta = select(upper - bounded, bounded, upper);
            }
        }
        let coefficient = if prefix.should_apply_force {
            let key = if delta < 0.0 {
                0x5c50_5baa_91bb_f3c6
            } else {
                0xf728_404c_f0e1_c5f1
            };
            attribute(source, Conservation, key).map_err(E::Source)?
        } else {
            0.0
        };
        let mass = source.body_mass().map_err(E::Source)?;
        let mass_coefficient = mass * coefficient;
        let impulse = mass_coefficient * delta;
        output.world_space_force = SkeletonRootVector::known([0.0; 4]);
        output.local_space_position = SkeletonRootVector::known([0.0; 4]);
        let magnitude = impulse / dt;
        output.local_space_position = SkeletonRootVector::known([0.0; 4]); // aae4
        let alignment = numeric.dot_vector(z, direction).map_err(E::Numeric)?;
        let axis = if alignment > 0.0 { z } else { flip(z) };
        output.world_space_force = scale(axis, magnitude);
        let debug = source.capture_cvar(SpeedCvar::Debug).map_err(E::Source)?;
        if source.cvar_bool(&debug).map_err(E::Source)? {
            source.require_overlay().map_err(E::Source)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "skateboard_speed_model_update_tests.rs"]
mod tests;
