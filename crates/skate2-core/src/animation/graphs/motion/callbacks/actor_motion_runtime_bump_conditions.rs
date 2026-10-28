//! `IsBumped82c9cd68` and `SurfaceSlope82c9a610` on the same Actor `PhysOut`.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError};
use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    generated_action_load::construct::Comparison,
    input_cvar_runtime::InputCvarReferences,
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningNumeric, length_from_squared_and_estimate, try_asin,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;
const X_SCALE: u64 = 0x796a_2241_00aa_f608;
const THRESHOLD: u64 = 0x2eda_fe74_f922_c817;
pub(crate) struct BumpMotionAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl BumpMotionAttributes {
    /// Global member158: generated `anim_motion` ctor828b1590, caller828b3070.
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(0x90c3_decf_0105_631e, 0x7823_ada8_e7c8_96aa);
        instance.try_cached_layout(0x5c0)?;
        Ok(Self {
            vault: Arc::clone(vault),
            instance,
        })
    }
}
fn attribute_error(error: AttributeRuntimeError) -> MotionUpdateError {
    MotionUpdateError::Load(crate::generated_motion_load::MotionLoadError::AttributeRuntime(error))
}
trait BumpSource {
    type Error;
    type Pointer;
    fn enabled(&mut self) -> Result<bool, Self::Error>;
    fn acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn value(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn squared(&mut self, vector: SkeletonRootVector) -> Result<f32, Self::Error>;
    fn estimate(&mut self, squared: f32) -> Result<f32, Self::Error>;
}
fn try_bumped<S: BumpSource>(source: &mut S) -> Result<bool, S::Error> {
    if !source.enabled()? {
        return Ok(false);
    }
    let mut vector = source.acceleration()?;
    let unscaled = vector;
    let scale = source.lookup(X_SCALE)?;
    let scale = source.value(&scale)?;
    // Native vrlimi mask8 changes X only, preserving Y/Z/W.
    vector.xyz[0] *= scale;
    let threshold = source.lookup(THRESHOLD)?;
    let squared = source.squared(vector)?;
    let threshold = source.value(&threshold)?;
    let estimate = source.estimate(squared)?;
    let magnitude = length_from_squared_and_estimate(squared, estimate);
    let bumped = magnitude > threshold;
    if std::env::var_os("SKATE2_TRACE_PUSH_INTERRUPTION").is_some() {
        eprintln!(
            "push bump condition: acceleration={:?} x_scale={scale} scaled={:?} magnitude={magnitude} threshold={threshold} bumped={bumped}",
            unscaled.xyz, vector.xyz,
        );
    }
    Ok(bumped)
}
fn slope<N: ReckoningNumeric>(
    normal: SkeletonRootVector,
    comparison: Comparison,
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let radians = try_asin(normal.xyz[1], numeric)?;
    let degrees = radians * f32::from_bits(0x4265_2ee1);
    Ok(crate::generated_action_load::conditions::compare(
        comparison,
        90.0 - degrees,
    ))
}
struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    cvars: &'a InputCvarReferences,
    vault: Option<Arc<AttributeRuntimeVault>>,
}
impl BumpSource for Source<'_> {
    type Error = MotionUpdateError;
    type Pointer = Option<AttributeRuntimeData>;
    fn enabled(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_motion_use_bump()
            .map_err(MotionUpdateError::Cvar)
    }
    fn acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        use MotionUpdateError as E;
        self.actor.check_motion_runtime_context(self.context)?;
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(E::Physical)?;
        self.vault = Some(Arc::clone(&physical.owner().inputs().vault));
        physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_motion_bump_acceleration()
            .map_err(E::PhysOut)
    }
    fn lookup(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        use MotionUpdateError as E;
        let services = self.actor.action_runtime_services().map_err(E::Actor)?;
        let attributes = &services
            .as_ref()
            .ok_or(E::MissingOwner("global anim_motion_bumps+158"))?
            .bumps;
        if !Arc::ptr_eq(
            &attributes.vault,
            self.vault
                .as_ref()
                .ok_or(E::MissingOwner("bump physical Vault"))?,
        ) {
            return Err(E::ForeignOwner("global anim_motion_bumps Vault"));
        }
        attributes
            .instance
            .try_get_data(key, 0)
            .map_err(attribute_error)
    }
    fn value(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        let Some(data) = pointer else {
            return Ok(0.0);
        };
        let word = data.bytes().try_into().map_err(|_| {
            attribute_error(AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            })
        })?;
        Ok(f32::from_be_bytes(word))
    }
    fn squared(&mut self, vector: SkeletonRootVector) -> Result<f32, Self::Error> {
        HostReckoningNumeric
            .dot_vector(vector, vector)
            .map_err(MotionUpdateError::ConditionNumeric)
    }
    fn estimate(&mut self, squared: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(MotionUpdateError::ConditionNumeric)
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_is_bumped(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        cvars: &InputCvarReferences,
    ) -> Result<bool, MotionUpdateError> {
        try_bumped(&mut Source {
            actor: self,
            context,
            cvars,
            vault: None,
        })
    }
    pub(crate) fn motion_runtime_surface_slope(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        comparison: Comparison,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let normal = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_ground_output()
            .map_err(E::PhysOut)?
            .below_deck_ground_data_normal;
        slope(normal, comparison, &mut HostReckoningNumeric).map_err(E::ConditionNumeric)
    }
}
#[cfg(test)]
#[path = "actor_motion_runtime_bump_conditions_tests.rs"]
mod tests;

#[path = "actor_motion_set_bump_coefficients.rs"]
mod coefficients;
