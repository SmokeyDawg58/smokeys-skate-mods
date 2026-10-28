//! Retained `PIDFilter<Vector3>` used by actual Reckoning filters.
//! Constructor 0x82e48e50..0x82e48ea8; Update 0x82dc8f20..0x82dc90bc.
//! Despite the source Vector3 name, every operation/store includes all four
//! lanes. This is the original filter graph, not a conventional PID controller:
//! no dt, normalization, clamp, estimate, callback or coefficient scaling exists.
//! Exclusive state ownership and a copied target exclude native concurrent/alias
//! writes between repeated loads. Host f32/FMA does not establish Xenon denormal,
//! NaN-payload, signed-zero or floating-point-status parity.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained filters await the complete Reckoning caller"
    )
)]

use crate::skeleton_root_transforms::SkeletonRootVector as Vector;

#[cfg(test)]
#[path = "reckoning_pid_tests.rs"]
mod tests;

/// PDB `PhysicsControllerData`, 16 bytes: P/I/D/integral-blend at0/4/8/c.
/// The PID constructor COPIES these values; it retains no Vault pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PhysicsControllerData {
    pub(crate) proportional: f32,
    pub(crate) integral: f32,
    pub(crate) derivative: f32,
    pub(crate) integral_blend: f32,
}

/// Named state corresponding to native offsets00/10/20/30/40/50.
/// A value override does not reset the histories. Reckoning uses that distinction
/// after normalization and in `MaintainFilters` 0x82dd8248.
#[derive(Debug)]
pub(crate) struct VectorPidFilter {
    control: PhysicsControllerData,
    value: Vector,
    last_target: Vector,
    integral: Vector,
    derivative: Vector,
    derivative_input: Vector,
}

impl VectorPidFilter {
    pub(crate) const fn new(control: PhysicsControllerData, initial: [f32; 4]) -> Self {
        Self {
            control,
            value: Vector::known(initial),
            last_target: Vector::known(initial),
            integral: Vector::known([0.0; 4]),
            derivative: Vector::known([0.0; 4]),
            derivative_input: Vector::known([0.0; 4]),
        }
    }

    pub(crate) const fn control(&self) -> PhysicsControllerData {
        self.control
    }
    pub(crate) const fn value(&self) -> Vector {
        self.value
    }
    pub(crate) const fn last_target(&self) -> Vector {
        self.last_target
    }
    pub(crate) const fn integral(&self) -> Vector {
        self.integral
    }
    pub(crate) const fn derivative(&self) -> Vector {
        self.derivative
    }
    pub(crate) const fn derivative_input(&self) -> Vector {
        self.derivative_input
    }

    /// Caller-owned coefficient replacement, not automatic Vault refreshing.
    pub(crate) fn replace_control(&mut self, control: PhysicsControllerData) {
        self.control = control;
    }

    /// Override ONLY native mVal. All four lanes copy without normalization.
    pub(crate) fn set_value(&mut self, value: impl Into<Vector>) {
        self.value = value.into();
    }

    pub(crate) fn update(&mut self, target: impl Into<Vector>) -> Vector {
        let target = target.into();
        let blend = self.control.integral_blend;
        let keep = 1.0 - blend;
        let error: Vector = target.zip(self.value, |a, b| a - b);
        // 8fbc rounded product, 8fc4 fused old-history contribution, 8fc8 store.
        let blended_error = error.map(|value| value * blend);
        self.integral = self.integral.zip(blended_error, |a, b| a.mul_add(keep, b));

        // Source reloads coefficient+c at8fcc, then target and old lastTarget.
        let blend = self.control.integral_blend;
        let keep = 1.0 - blend;
        let target_delta: Vector = target.zip(self.last_target, |a, b| a - b);
        let blended_delta = target_delta.map(|value| value * blend);
        self.derivative_input = self
            .derivative_input
            .zip(blended_delta, |a, b| a.mul_add(keep, b));

        let proportional = self.control.proportional;
        let integral = self.control.integral;
        let derivative = self.control.derivative;
        let base: Vector = self.integral.zip(self.value, |a, b| a.mul_add(integral, b));
        let with_proportional: Vector = error.zip(base, |a, b| a.mul_add(proportional, b));
        // +40 is previous OUTPUT delta, not previous filtered target delta+50.
        let correction: Vector = self.derivative_input.zip(self.derivative, |a, b| a - b);
        let next: Vector = correction.zip(with_proportional, |a, b| a.mul_add(derivative, b));
        self.derivative = next.zip(self.value, |a, b| a - b);
        self.value = next;
        self.last_target = target;
        self.value
    }
}
