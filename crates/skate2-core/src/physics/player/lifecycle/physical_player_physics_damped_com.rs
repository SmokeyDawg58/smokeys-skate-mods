//! Actual retained `Physics.Update` fourth call: `UpdateDampedComPos`82e2a720.
use super::{
    Conditioners, PhysicalPlayerPhysOut, SkeletonRootVector as V, ZERO,
    physics_update::PhysicsBeforeDampedCom, simple::Physics,
};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences, damped_com::DampedComCvars},
    physical_player_phys_out::construct::prior_input::PriorInputReadError,
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum DampedComError<E> {
    ForeignOutput,
    MissingPhysics,
    Output(PriorInputReadError),
    Cvar(InputCvarError),
    Numeric(E),
    Overlay(DebugOverlayError),
}
impl<E: std::fmt::Debug> std::fmt::Display for DampedComError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignOutput => f.write_str("DampedCOM continuation belongs to another output"),
            Self::MissingPhysics => {
                f.write_str("DampedCOM requires the published Physics conditioner")
            }
            Self::Output(error) => write!(f, "DampedCOM output access: {error:?}"),
            Self::Cvar(error) => write!(f, "DampedCOM live cvar: {error:?}"),
            Self::Numeric(error) => write!(f, "DampedCOM dot product: {error:?}"),
            Self::Overlay(error) => write!(f, "DampedCOM overlay: {error}"),
        }
    }
}
pub(crate) struct PhysicsBeforeFootSideOfDeck {
    output: Arc<()>,
}
impl PhysicsBeforeFootSideOfDeck {
    pub(crate) fn matches_output(&self, output: &PhysicalPlayerPhysOut) -> bool {
        Arc::ptr_eq(&self.output, output.occurrence())
    }
}
impl Conditioners {
    pub(crate) fn try_update_physics_damped_com<N: ReckoningNumeric>(
        &mut self,
        before: PhysicsBeforeDampedCom,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<PhysicsBeforeFootSideOfDeck, DampedComError<N::Error>> {
        if !before.matches_output(output) || !self.matches_output(output) {
            return Err(DampedComError::ForeignOutput);
        }
        drop(before);
        let physics = self
            .physics
            .published_value_mut()
            .ok_or(DampedComError::MissingPhysics)?;
        if !Arc::ptr_eq(&physics.output, output.occurrence()) {
            return Err(DampedComError::ForeignOutput);
        }
        physics
            .payload
            .update_damped_com(output, &DampedComCvars(cvars), overlay, numeric)?;
        Ok(PhysicsBeforeFootSideOfDeck {
            output: Arc::clone(output.occurrence()),
        })
    }
}
impl Physics {
    fn update_damped_com<N: ReckoningNumeric>(
        &mut self,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &DampedComCvars<'_>,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), DampedComError<N::Error>> {
        use DampedComError as E;
        let (valid, current) = output
            .damped_com_initial(|| self.last_smoothed_com_position_valid)
            .map_err(E::Output)?;
        if valid {
            let guard = output
                .sample_write_guard()
                .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
            let velocity = cvars.capture(196).map_err(E::Cvar)?;
            let velocity_y = cvars.capture(198).map_err(E::Cvar)?;
            let blend = cvars.capture(195).map_err(E::Cvar)?;
            let blend_y = cvars.capture(197).map_err(E::Cvar)?;
            let velocity = DampedComCvars::float(&velocity).map_err(E::Cvar)?;
            let velocity_y = DampedComCvars::float(&velocity_y).map_err(E::Cvar)?;
            let blend = DampedComCvars::float(&blend).map_err(E::Cvar)?;
            let blend_y = DampedComCvars::float(&blend_y).map_err(E::Cvar)?;
            let velocity = coefficient(velocity, velocity_y);
            let blend = coefficient(blend, blend_y);
            let ones = coefficient(1.0, 1.0);
            let velocity_complement = zip(ones, velocity, |a, b| a - b);
            let blend_complement = zip(ones, blend, |a, b| a - b);
            output.damped_com_check(&guard).map_err(E::Output)?;
            let weighted_position = zip(blend, current, |a, b| a * b);
            let actual_velocity = output.damped_com_velocity_after_guard();
            let weighted_velocity = zip(velocity, actual_velocity, |a, b| a * b);
            self.heavily_damped_com_velocity = fused(
                velocity_complement,
                self.heavily_damped_com_velocity,
                weighted_velocity,
            );
            let predicted = fused(
                self.heavily_damped_com_velocity,
                splat(f32::from_bits(0x3c88_8889)),
                self.last_damped_com_position,
            );
            let old_error = self.accumulated_com_error;
            let mut smoothed = fused(blend_complement, predicted, weighted_position);
            let error = zip(current, smoothed, |a, b| a - b);
            if numeric.dot_vector(error, old_error).map_err(E::Numeric)? > 0.0 {
                let blend = DampedComCvars::float(&cvars.capture(199).map_err(E::Cvar)?)
                    .map_err(E::Cvar)?;
                smoothed = fused(old_error, splat(blend), smoothed);
            }
            if cvars.draw().map_err(E::Cvar)? {
                overlay.require().map_err(E::Overlay)?;
            }
            self.last_damped_com_position = smoothed;
            let old_error = self.accumulated_com_error;
            let error = zip(current, smoothed, |a, b| a - b);
            let iteration =
                DampedComCvars::float(&cvars.capture(200).map_err(E::Cvar)?).map_err(E::Cvar)?;
            let complement = 1.0 - iteration;
            let weighted_error = zip(error, splat(iteration), |a, b| a * b);
            self.accumulated_com_error = fused(old_error, splat(complement), weighted_error);
        } else {
            self.last_damped_com_position = current;
            self.last_smoothed_com_position_valid = true;
            self.heavily_damped_com_velocity = ZERO;
        }
        output
            .damped_com_store(|| self.last_damped_com_position)
            .map_err(E::Output)
    }
}
fn coefficient(xz: f32, y: f32) -> V {
    V {
        xyz: [xz, y, xz],
        w: Some(0.0),
    }
}
fn splat(x: f32) -> V {
    V {
        xyz: [x; 3],
        w: Some(x),
    }
}
fn zip(a: V, b: V, f: impl Fn(f32, f32) -> f32) -> V {
    V {
        xyz: std::array::from_fn(|i| f(a.xyz[i], b.xyz[i])),
        w: a.w.zip(b.w).map(|(a, b)| f(a, b)),
    }
}
fn fused(a: V, b: V, c: V) -> V {
    V {
        xyz: std::array::from_fn(|i| a.xyz[i].mul_add(b.xyz[i], c.xyz[i])),
        w: a.w.zip(b.w).zip(c.w).map(|((a, b), c)| a.mul_add(b, c)),
    }
}
#[cfg(test)]
#[path = "physical_player_physics_damped_com_tests.rs"]
mod tests;
