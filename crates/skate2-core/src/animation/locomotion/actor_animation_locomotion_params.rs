//! Native physical locomotion parameter publications, retained FootControl owners.
use super::{AnimationFootData, AnimationLocomotionTrajectory};
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_root_transforms::SkeletonRootVector as V,
};
#[path = "actor_animation_locomotion_air_params.rs"]
mod air;
#[derive(Debug)]
pub(super) enum LocomotionParamsError<N, S> {
    Index(i32),
    Assertion(&'static str),
    Numeric(N),
    SinCos(S),
    Cvar(InputCvarError),
}
#[derive(Debug)]
pub(super) enum PhysicsParamsError<N, S, D> {
    Params(LocomotionParamsError<N, S>),
    Debug(D),
}
fn index<N, S>(v: i32) -> Result<usize, LocomotionParamsError<N, S>> {
    usize::try_from(v)
        .ok()
        .filter(|v| *v < 4)
        .ok_or(LocomotionParamsError::Index(v))
}
fn add(a: V, b: V) -> V {
    a.zip(b, |a, b| a + b)
}
fn sub(a: V, b: V) -> V {
    a.zip(b, |a, b| a - b)
}
fn half(a: V) -> V {
    a.map(|v| v * 0.5)
}
fn horizontal(mut a: V) -> V {
    a.xyz[1] = 0.0;
    a
}
fn require_positive<N, S>(
    v: f32,
    minimum: f32,
    label: &'static str,
) -> Result<(), LocomotionParamsError<N, S>> {
    // Native BO12/BI25 skips these assertions only for ordered GT.
    if v.partial_cmp(&minimum) != Some(std::cmp::Ordering::Greater) {
        return Err(LocomotionParamsError::Assertion(label));
    }
    Ok(())
}
// Exact VNMSUB graph: outer negation retains native fused zero signs.
fn sqrt_from_seed(value: f32, mut estimate: f32) -> f32 {
    for _ in 0..2 {
        let squared = estimate * estimate;
        let half = estimate * 0.5;
        let residual = -value.mul_add(squared, -1.0);
        estimate = half.mul_add(residual, estimate);
    }
    let product = value * estimate;
    if value == 0.0 { 0.0 } else { product }
}
impl AnimationLocomotionTrajectory {
    ///82c8bca8: destination vector stores precede duration and length assertions.
    fn compute_contact_to_pass<N: ReckoningNumeric, S>(
        &mut self,
        i: usize,
        current: usize,
        numeric: &mut N,
    ) -> Result<(), LocomotionParamsError<N::Error, S>> {
        use LocomotionParamsError as E;
        let previous = (i + 3) % 4;
        if self.physical_feet[previous].initialized {
            let prior = self.physical_feet[previous];
            let base = sub(
                add(
                    add(prior.states[2].vectors[0], prior.states[2].vectors[2]),
                    prior.data.position,
                ),
                self.physical_feet[i].data.position,
            );
            self.physical_feet[i].states[0].vectors[0] = base;
            let ratio =
                (prior.states[2].time / self.input_feet[(current + 3) % 4].states[2].time) * 0.5;
            let complement = 1.0 - ratio;
            let first = self.input_feet[current].states[0].vectors[1].map(|v| v * complement);
            let blended = prior.states[2].vectors[3].zip(first, |v, first| v.mul_add(ratio, first));
            self.physical_feet[i].states[0].vectors[1] = half(blended);
        } else {
            self.physical_feet[i].states[0].vectors[0] =
                self.input_feet[current].states[0].vectors[0];
            self.physical_feet[i].states[0].vectors[1] =
                self.input_feet[current].states[0].vectors[1];
        }
        let input = self.input_feet[current];
        let midpoint = half(add(
            add(input.states[0].vectors[2], input.states[1].vectors[0]),
            input.states[0].vectors[0],
        ));
        self.physical_feet[i].states[0].vectors[2] =
            sub(midpoint, self.physical_feet[i].states[0].vectors[0]);
        self.physical_feet[i].states[0].vectors[3] = input.states[0].vectors[3];
        require_positive(input.states[0].time, 0.0, "contact duration")?;
        let original = horizontal(input.states[0].vectors[2]);
        let adjusted = horizontal(self.physical_feet[i].states[0].vectors[2]);
        //bf00/bf08 both dots before bf0c/bf14 estimates.
        let a = numeric.dot_vector(original, original).map_err(E::Numeric)?;
        let b = numeric.dot_vector(adjusted, adjusted).map_err(E::Numeric)?;
        let ea = numeric.reciprocal_sqrt_estimate(a).map_err(E::Numeric)?;
        let eb = numeric.reciprocal_sqrt_estimate(b).map_err(E::Numeric)?;
        let original = sqrt_from_seed(a, ea);
        let adjusted = sqrt_from_seed(b, eb);
        if !(original > f32::from_bits(0x38d1b717)) && std::env::var_os("SKATE2_TRACE_SELECTED_STATES").is_some() {
            eprintln!("locomotion contact failure: physical={i} input={current} locked={} dot={a:?} estimate={ea:?} input_feet={:?}", self.foot_locked, self.input_feet);
        }
        require_positive(
            original,
            f32::from_bits(0x38d1b717),
            "contact horizontal length",
        )?;
        self.physical_feet[i].states[0].time = (adjusted / original) * input.states[0].time;
        Ok(())
    }
    ///82c8ba10: current contact end becomes pass start before source assertions.
    fn compute_pass_to_air<N: ReckoningNumeric, S>(
        &mut self,
        i: usize,
        current: usize,
        numeric: &mut N,
    ) -> Result<(), LocomotionParamsError<N::Error, S>> {
        use LocomotionParamsError as E;
        self.physical_feet[i].states[1].vectors[0] = add(
            self.physical_feet[i].states[0].vectors[0],
            self.physical_feet[i].states[0].vectors[2],
        );
        self.physical_feet[i].states[1].vectors[1] = self.physical_feet[i].states[0].vectors[3];
        self.physical_feet[i].states[1].vectors[2] = sub(
            self.physical_feet[i].states[2].vectors[0],
            self.physical_feet[i].states[1].vectors[0],
        );
        let input = self.input_feet[current];
        require_positive(input.states[2].time, 0.0, "input air duration")?;
        let ratio = (self.physical_feet[i].states[2].time / input.states[2].time) * 0.5;
        let first = input.states[1].vectors[3].map(|v| v * (1.0 - ratio));
        self.physical_feet[i].states[1].vectors[3] = half(
            self.physical_feet[i].states[2].vectors[1]
                .zip(first, |v, first| v.mul_add(ratio, first)),
        );
        require_positive(input.states[1].time, 0.0, "pass duration")?;
        let original = horizontal(input.states[1].vectors[2]);
        let adjusted = horizontal(self.physical_feet[i].states[1].vectors[2]);
        //Unlike contact, first estimate occurs before second dot (bb7c..bb88).
        let a = numeric.dot_vector(original, original).map_err(E::Numeric)?;
        let ea = numeric.reciprocal_sqrt_estimate(a).map_err(E::Numeric)?;
        let b = numeric.dot_vector(adjusted, adjusted).map_err(E::Numeric)?;
        let eb = numeric.reciprocal_sqrt_estimate(b).map_err(E::Numeric)?;
        let original = sqrt_from_seed(a, ea);
        let adjusted = sqrt_from_seed(b, eb);
        require_positive(
            original,
            f32::from_bits(0x38d1b717),
            "pass horizontal length",
        )?;
        self.physical_feet[i].states[1].time = (adjusted / original) * input.states[1].time;
        Ok(())
    }
    /// UpdatePhysicsData82ca89e0. Debug callback is the actual shared DebugDraw.
    pub(super) fn try_update_physics_data<N: ReckoningNumeric, S: ReckoningSinCosNumeric, D>(
        &mut self,
        next: AnimationFootData,
        nextnext: AnimationFootData,
        cvars: &InputCvarReferences,
        numeric: &mut N,
        sin_cos: &mut S,
        debug: impl FnOnce(&Self, usize, &mut N) -> Result<(), D>,
    ) -> Result<(), PhysicsParamsError<N::Error, S::Error, D>> {
        use PhysicsParamsError as E;
        let i = index(self.physical_foot_index).map_err(E::Params)?;
        let locked = index(self.foot_locked).map_err(E::Params)?;
        self.physical_feet[i].data = self.input_feet[locked].data;
        let next_i = (i + 1) % 4;
        let next_locked = (locked + 1) % 4;
        let after = (i + 2) % 4;
        self.physical_feet[next_i].data = next;
        self.physical_feet[after].data = nextnext;
        self.compute_air(
            next_i,
            next_locked,
            next.position,
            nextnext.position,
            cvars,
            numeric,
            sin_cos,
        )
        .map_err(E::Params)?;
        self.compute_air(
            i,
            locked,
            self.physical_feet[i].data.position,
            next.position,
            cvars,
            numeric,
            sin_cos,
        )
        .map_err(E::Params)?;
        self.compute_contact_to_pass(next_i, next_locked, numeric)
            .map_err(E::Params)?;
        self.compute_pass_to_air(next_i, next_locked, numeric)
            .map_err(E::Params)?;
        self.compute_contact_to_pass(i, locked, numeric)
            .map_err(E::Params)?;
        self.compute_pass_to_air(i, locked, numeric)
            .map_err(E::Params)?;
        self.physical_feet[next_i].stride = sub(
            self.physical_feet[next_i].data.position,
            self.physical_feet[i].data.position,
        );
        self.physical_feet[after].stride = sub(
            self.physical_feet[after].data.position,
            self.physical_feet[next_i].data.position,
        );
        self.physical_feet[next_i].stride_initialized = true;
        self.physical_feet[after].stride_initialized = true;
        self.physical_feet[after].initialized = true;
        self.first_update = false;
        debug(self, i, numeric).map_err(E::Debug)
    }
}

#[cfg(test)]
#[path = "actor_animation_locomotion_params_tests.rs"]
mod tests;
