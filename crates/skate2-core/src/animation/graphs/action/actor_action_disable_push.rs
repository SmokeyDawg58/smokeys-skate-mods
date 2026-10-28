//! Actual DisablePushBrake.ActivationFunc82c9a8f0, proposed intent-bridge child.
use super::{ActionMotionIntentError, ActorConstruction, ActorIdentity};
use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    generated_motion_load::{MotionLoadError, runtime::MotionUpdateError},
    motion_graph_entity_authority::BoundMotionGraphContext,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::PhysOutAccessError,
    physics::PointGraph,
    reckoning_numeric::{ReckoningNumeric, try_asin},
};
use std::sync::Arc;

pub(crate) trait DisablePushSource {
    type Error;
    type Attribute;
    type Output;
    fn ground_up_y(&mut self) -> Result<f32, Self::Error>;
    fn capture_slope_limit(&mut self) -> Result<Self::Attribute, Self::Error>;
    fn read_slope_limit(&mut self, pointer: &Self::Attribute) -> Result<f32, Self::Error>;
    fn fresh_output(&mut self) -> Result<Self::Output, Self::Error>;
    fn tell_anim_dont_push_or_brake(&mut self, output: &Self::Output) -> Result<bool, Self::Error>;
}
#[derive(Debug, PartialEq)]
pub(crate) enum DisablePushError<S, N> {
    Source(S),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for DisablePushError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DisablePushBrake: {self:?}")
    }
}
pub(crate) fn try_disable_push<S: DisablePushSource, N: ReckoningNumeric>(
    source: &mut S,
    numeric: &mut N,
) -> Result<bool, DisablePushError<S::Error, N::Error>> {
    use DisablePushError as E;
    let y = source.ground_up_y().map_err(E::Source)?;
    let limit = source.capture_slope_limit().map_err(E::Source)?; //aa28
    let degrees = try_asin(y, numeric).map_err(E::Numeric)? * f32::from_bits(0x4265_2ee1); //aa6c/70
    let limit = source.read_slope_limit(&limit).map_err(E::Source)?; //aa78 delayed
    let slope = 90.0 - degrees; //aab0 separate subtraction
    let steep = slope > limit; //aac0, ordered compare; no NaN normalization
    let output = source.fresh_output().map_err(E::Source)?; //aad8 ALWAYS before result test
    Ok(steep
        || source
            .tell_anim_dont_push_or_brake(&output)
            .map_err(E::Source)?)
}

/// Actual GlobalAttributeCollection+13c selected allocation; distinct from +128.
pub(crate) struct PushingMotionAttributes(AttributeRuntimeInstance, Arc<AttributeRuntimeVault>);
impl PushingMotionAttributes {
    /// Alloc828b2e54/generated ctor828b2e78/member store828b2e84.
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(0x90c3_decf_0105_631e, 0x9dc4_354d_be42_8a7f);
        instance.try_cached_layout(0x5c0)?;
        Ok(Self(instance, Arc::clone(vault)))
    }

    pub(crate) fn check_vault(
        &self,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<(), MotionUpdateError> {
        if Arc::ptr_eq(&self.1, vault) {
            Ok(())
        } else {
            Err(MotionUpdateError::ForeignOwner("Pushing global Vault"))
        }
    }

    /// Fresh `GetAttributePointer` through the retained +13c Pushing instance.
    /// The native generated caller uses `DefaultDataArea(4)` when absent.
    pub(crate) fn first_push_scalar(&self, key: u64) -> Result<f32, MotionUpdateError> {
        let pointer = self.first_push_scalar_pointer(key)?;
        Self::first_push_scalar_value(pointer.as_ref())
    }

    /// `ComputeRepushDeadline::End` 0x82ca1118/0x82ca1170 performs two
    /// independent fresh lookups from this same Pushing collection.
    pub(crate) fn repush_scalar(&self, key: u64) -> Result<f32, MotionUpdateError> {
        if !matches!(key, 0x792c_66a0_ddc0_2df7 | 0x1761_c0ae_4c9e_79c2) {
            return Err(MotionUpdateError::MissingOwner("repush Pushing scalar key"));
        }
        let pointer = self
            .0
            .try_get_data(key, 0)
            .map_err(|error| MotionUpdateError::Load(MotionLoadError::AttributeRuntime(error)))?;
        Self::first_push_scalar_value(pointer.as_ref())
    }

    /// `PushCycle::Update82cd68e0` fetches the generated scalar afresh.
    /// Accessor82c8ad60 inserts 07a7a486 into the HIGH word of r4;
    /// d4c75e46 remains the low word (insrdi at82c8ad80).
    pub(crate) fn max_holding_acc(&self) -> Result<f32, MotionUpdateError> {
        let pointer = self
            .0
            .try_get_data(0x07a7_a486_d4c7_5e46, 0)
            .map_err(|error| MotionUpdateError::Load(MotionLoadError::AttributeRuntime(error)))?;
        Self::first_push_scalar_value(pointer.as_ref())
    }

    /// `Begin82ca0c3c` retains this pointer across the virtual time getter.
    pub(crate) fn first_push_scalar_pointer(
        &self,
        key: u64,
    ) -> Result<Option<AttributeRuntimeData>, MotionUpdateError> {
        if !matches!(key, 0x4c4b_9cdf_5860_f565 | 0x1a4c_a176_9dcc_b3a8) {
            return Err(MotionUpdateError::MissingOwner(
                "first-push Pushing scalar key",
            ));
        }
        self.0
            .try_get_data(key, 0)
            .map_err(|error| MotionUpdateError::Load(MotionLoadError::AttributeRuntime(error)))
    }

    pub(crate) fn first_push_scalar_value(
        pointer: Option<&AttributeRuntimeData>,
    ) -> Result<f32, MotionUpdateError> {
        let Some(pointer) = pointer else {
            return Ok(0.0);
        };
        let word: [u8; 4] = pointer.bytes().try_into().map_err(|_| {
            MotionUpdateError::Load(MotionLoadError::AttributeRuntime(
                AttributeRuntimeError::ScalarSize {
                    size: pointer.bytes().len(),
                },
            ))
        })?;
        Ok(f32::from_be_bytes(word))
    }

    /// `MapButtonPressToChangeVel82c95328` and `SetPushCoefs::Ramp82ca00b0`
    /// use four PointGraphData8 fields of the same cached Pushing layout.
    pub(crate) fn first_push_graph(
        &self,
        offset: usize,
        input: f32,
    ) -> Result<f32, MotionUpdateError> {
        if !matches!(offset, 0x230 | 0x1e0 | 0x280 | 0x2d0) {
            return Err(MotionUpdateError::MissingOwner(
                "first-push Pushing graph offset",
            ));
        }
        let layout = self
            .0
            .try_cached_layout(0x5c0)
            .map_err(|error| MotionUpdateError::Load(MotionLoadError::AttributeRuntime(error)))?;
        // Generated ctor828b2e78 substitutes DefaultDataArea5c0 when absent.
        let word = |at: usize| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[at..at + 4]
                        .try_into()
                        .expect("validated Pushing cached layout"),
                )
            })
        };
        Ok(PointGraph::<8> {
            x: std::array::from_fn(|index| word(offset + 0x10 + index * 4)),
            y: std::array::from_fn(|index| word(offset + 0x30 + index * 4)),
        }
        .evaluate(input))
    }
}
#[derive(Debug)]
pub(crate) enum DisablePushActorError {
    Intent(ActionMotionIntentError),
    Attribute(AttributeRuntimeError),
    AttributeSize(usize),
    MissingPushingGlobal,
    Output(PhysOutAccessError),
    PoisonedOutput,
}
impl std::fmt::Display for DisablePushActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Intent(e) => write!(f, "push condition intent: {e}"),
            Self::Attribute(e) => write!(f, "push condition attribute: {e}"),
            Self::AttributeSize(size) => write!(f, "push condition scalar has {size} bytes"),
            Self::Output(e) => write!(f, "push condition output: {e:?}"),
            Self::MissingPushingGlobal => write!(f, "missing PushingMotion global"),
            Self::PoisonedOutput => write!(f, "push condition output lock poisoned"),
        }
    }
}
struct Source<'a> {
    actor: &'a ActorConstruction,
    attributes: Option<&'a PushingMotionAttributes>,
}
impl DisablePushSource for Source<'_> {
    type Error = DisablePushActorError;
    type Attribute = Option<AttributeRuntimeData>;
    type Output = Arc<PhysicalPlayerAssemblyOwner>;
    fn ground_up_y(&mut self) -> Result<f32, Self::Error> {
        let allocation = self
            .actor
            .cached_physical_allocation()
            .map_err(|e| Self::Error::Intent(ActionMotionIntentError::Physical(e)))?;
        allocation
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| Self::Error::PoisonedOutput)?
            .action_condition_ground_up_y()
            .map_err(Self::Error::Output)
    }
    fn capture_slope_limit(&mut self) -> Result<Self::Attribute, Self::Error> {
        self.attributes
            .ok_or(Self::Error::MissingPushingGlobal)?
            .0
            .try_get_data(0xa40d_f063_c0ac_73df, 0)
            .map_err(Self::Error::Attribute)
    }
    fn read_slope_limit(&mut self, pointer: &Self::Attribute) -> Result<f32, Self::Error> {
        let Some(data) = pointer else { return Ok(0.0) };
        let bytes = data
            .bytes()
            .try_into()
            .map_err(|_| Self::Error::AttributeSize(data.bytes().len()))?;
        Ok(f32::from_be_bytes(bytes))
    }
    fn fresh_output(&mut self) -> Result<Self::Output, Self::Error> {
        let allocation = self
            .actor
            .cached_physical_allocation()
            .map_err(|e| Self::Error::Intent(ActionMotionIntentError::Physical(e)))?;
        Ok(Arc::clone(allocation.owner()))
    }
    fn tell_anim_dont_push_or_brake(&mut self, output: &Self::Output) -> Result<bool, Self::Error> {
        Ok(output
            .phys_out()
            .lock()
            .map_err(|_| Self::Error::PoisonedOutput)?
            .try_skeleton_output()
            .map_err(|e| Self::Error::Intent(ActionMotionIntentError::PhysOut(e)))?
            .tell_anim_dont_push_or_brake)
    }
}
impl ActorConstruction {
    pub(crate) fn action_runtime_disable_push<N: ReckoningNumeric>(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        attributes: Option<&PushingMotionAttributes>,
        numeric: &mut N,
    ) -> Result<bool, DisablePushError<DisablePushActorError, N::Error>> {
        self.check_action_runtime_context(context)
            .map_err(|e| DisablePushError::Source(DisablePushActorError::Intent(e)))?;
        self.runtime_disable_push(attributes, numeric)
    }

    /// Shared native predicate after the calling graph validates its context.
    pub(crate) fn runtime_disable_push<N: ReckoningNumeric>(
        &self,
        attributes: Option<&PushingMotionAttributes>,
        numeric: &mut N,
    ) -> Result<bool, DisablePushError<DisablePushActorError, N::Error>> {
        try_disable_push(
            &mut Source {
                actor: self,
                attributes,
            },
            numeric,
        )
    }
}
#[cfg(test)]
#[path = "actor_action_disable_push_tests.rs"]
mod tests;
