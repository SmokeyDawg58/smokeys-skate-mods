//! Actual same-owner Skeleton.ProcessAnimAttributes caller and collaborators.
use crate::{
    actor_construction::{
        anim_out_phys_in::AnimOutPhysInOwner,
        physical_mount::input_query::{ActorCapturedPhysicalInput, ActorPhysicalInputError},
    },
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::skeleton_normal::SkeletonNormalAssembly,
    input_cvar_runtime::InputCvarError,
    physical_player_anim_out::PhysicalAnimOut,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    reckoning_numeric::ReckoningNumeric,
    skeleton_assembly_construction::constructor::SkeletonConstructorRuntime,
    skeleton_jump_adjustments::SkeletonJumpAdjustmentError,
    skeleton_processed_contact::{
        SkeletonProcessedContact,
        anim_attributes::{AttributeReadError, named, payload_scalar},
        jump_events::{
            SkeletonJumpAttributes, SkeletonJumpError, SkeletonJumpGlobal, SkeletonJumpGlobalToken,
            SkeletonJumpInput, SkeletonJumpPointer, SkeletonJumpVaultError,
        },
    },
};
use std::{convert::Infallible, sync::Arc};

#[derive(Debug)]
pub(crate) enum SkeletonAnimAttributesError<N> {
    Missing(&'static str),
    DebugOverlay(DebugOverlayError),
    Attribute(AttributeReadError),
    Cvar(InputCvarError),
    Numeric(N),
    Jump(SkeletonJumpError<SkeletonAttributeInputError, Infallible>),
    Finish(SkeletonJumpError<SkeletonAttributeInputError, AttributeVaultError>),
    InvalidTrajectory(i32),
}
impl<N: std::fmt::Debug> std::fmt::Display for SkeletonAnimAttributesError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "animation attributes missing {field}"),
            Self::DebugOverlay(error) => write!(f, "animation attributes: {error}"),
            Self::Attribute(error) => write!(f, "animation attributes: {error}"),
            Self::Cvar(error) => write!(f, "animation attribute cvar: {error:?}"),
            Self::Numeric(error) => write!(f, "animation push numeric: {error:?}"),
            Self::Jump(error) => write!(f, "animation jump: {error}"),
            Self::Finish(error) => write!(f, "animation attribute suffix: {error}"),
            Self::InvalidTrajectory(index) => {
                write!(f, "animation trajectory bone {index} unavailable")
            }
        }
    }
}
#[derive(Debug)]
pub(crate) enum SkeletonAttributeInputError {
    MissingQuery,
    Input(ActorPhysicalInputError),
    Adjustment(SkeletonJumpAdjustmentError),
}
impl std::fmt::Display for SkeletonAttributeInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingQuery => f.write_str("Skeleton Actor query unavailable"),
            Self::Input(error) => write!(f, "Skeleton Actor input: {error}"),
            Self::Adjustment(error) => write!(f, "Skeleton adjustment: {error}"),
        }
    }
}
struct Input<'a> {
    normal: &'a mut SkeletonNormalAssembly,
    bindings:
        Option<&'a crate::skeleton_assembly_construction::constructor::SkeletonConstructorBindings>,
}
impl SkeletonJumpInput for Input<'_> {
    type Input = ActorCapturedPhysicalInput;
    type Error = SkeletonAttributeInputError;
    fn capture_input(&mut self) -> Result<Option<Self::Input>, Self::Error> {
        self.bindings
            .ok_or(Self::Error::MissingQuery)?
            .actor_query
            .as_ref()
            .ok_or(Self::Error::MissingQuery)?
            .try_capture_input()
            .map_err(Self::Error::Input)
    }
    fn value(&mut self, input: &Self::Input, id: u32) -> Result<f32, Self::Error> {
        input.try_value(id).map_err(Self::Error::Input)
    }
    fn store_adjustment_x(&mut self, value: f32) -> Result<(), Self::Error> {
        self.normal
            .store_jump_adjustment_x(value)
            .map_err(Self::Error::Adjustment)
    }
    fn store_adjustment_z(&mut self, value: f32) -> Result<(), Self::Error> {
        self.normal
            .store_jump_adjustment_z(value)
            .map_err(Self::Error::Adjustment)
    }
    fn adjustment_x(&mut self) -> Result<f32, Self::Error> {
        self.normal
            .jump_adjustment_x()
            .map_err(Self::Error::Adjustment)
    }
    fn adjustment_z(&mut self) -> Result<f32, Self::Error> {
        self.normal
            .jump_adjustment_z()
            .map_err(Self::Error::Adjustment)
    }
}
#[derive(Debug)]
pub(crate) enum AttributeVaultError {
    Poisoned,
    Vault(SkeletonJumpVaultError),
}
impl std::fmt::Display for AttributeVaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("animation attribute Vault lock poisoned"),
            Self::Vault(error) => write!(f, "animation attribute Vault: {error}"),
        }
    }
}
struct Vault<'a>(&'a PhysicalPlayerAssemblyOwner);
impl SkeletonJumpAttributes for Vault<'_> {
    type Global = SkeletonJumpGlobalToken;
    type Pointer = SkeletonJumpPointer;
    type Error = AttributeVaultError;
    fn capture(&mut self, kind: SkeletonJumpGlobal) -> Result<Self::Global, Self::Error> {
        self.0
            .jump_attributes()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .capture(kind)
            .map_err(Self::Error::Vault)
    }
    fn lookup(&mut self, global: &Self::Global, key: u64) -> Result<Self::Pointer, Self::Error> {
        self.0
            .jump_attributes()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .lookup(global, key)
            .map_err(Self::Error::Vault)
    }
    fn byte(&mut self, pointer: &Self::Pointer) -> Result<u8, Self::Error> {
        self.0
            .jump_attributes()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .byte(pointer)
            .map_err(Self::Error::Vault)
    }
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        self.0
            .jump_attributes()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .float(pointer)
            .map_err(Self::Error::Vault)
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "borrow actual native owners without duplicate frame storage"
)]
#[expect(
    clippy::float_cmp,
    reason = "native push-start transition compares exactly with one"
)]
pub(crate) fn try_process_skeleton_anim_attributes<N: ReckoningNumeric>(
    normal: &mut SkeletonNormalAssembly,
    constructor: &mut SkeletonConstructorRuntime,
    input: &AnimOutPhysInOwner,
    processed: &mut SkeletonProcessedContact,
    anim_out: &mut PhysicalAnimOut,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    debug_overlay: &DebugOverlaySlot,
    numeric: &mut N,
) -> Result<(), SkeletonAnimAttributesError<N::Error>> {
    use SkeletonAnimAttributesError as E;
    // 59e18..48 requires the actual singleton even for an empty attribute range.
    let _debug = debug_overlay.require().map_err(E::DebugOverlay)?;
    let dt = constructor
        .outer
        .as_ref()
        .ok_or(E::Missing("Skeleton dt"))?
        .time_delta;
    let count = input.attribute_count;
    let data = Arc::clone(&normal.session().template().collision().body().data);
    let mut runtime_input = Input {
        normal,
        bindings: constructor.bindings.as_ref(),
    };
    let mut reverting = false;
    let mut revert_direction = 1.0;
    processed.begin_jump_events();
    for index in 0..count {
        let attribute = input
            .attribute(index)
            .ok_or(E::Missing("active attribute record"))?;
        if let Some(transition) = processed
            .try_apply_animation_contact_prefix(attribute, &data)
            .map_err(E::Attribute)?
        {
            // Source always calculates this length, even when transition!=1.
            let reciprocal_dt = 1.0 / dt;
            let trajectory = constructor.init.trajectory_node;
            let pose = input
                .global_pose
                .as_ref()
                .ok_or(E::Missing("animation poseBuffer"))?;
            let matrix = usize::try_from(trajectory)
                .ok()
                .and_then(|node| pose.get(node))
                .ok_or(E::InvalidTrajectory(trajectory))?;
            let speed = push_speed([matrix[12], matrix[13], matrix[14]], reciprocal_dt, numeric)
                .map_err(E::Numeric)?;
            processed.store_animation_push_speed(if transition == 1.0 { speed } else { 0.0 });
        }
        if attribute.kind != 0 {
            continue;
        }
        if processed
            .try_apply_typed_jump_event(attribute, &mut runtime_input)
            .map_err(E::Jump)?
        {
            continue;
        }
        if named(attribute, "Revert") {
            reverting = true;
        } else if named(attribute, "RevertDir") {
            revert_direction = payload_scalar(attribute).map_err(E::Attribute)?;
        } else if named(attribute, "DangerZone") {
            processed
                .animation_attribute_flags(0x8c8, 0x0001_0000, 0x0001_0000)
                .map_err(E::Attribute)?;
            // Both outcomes converge: enabled native call82b5af48 targets BLR825187c0.
            let _enabled = owner
                .inputs()
                .cvars
                .try_show_danger_zone()
                .map_err(E::Cvar)?;
        } else {
            processed
                .try_apply_animation_attribute_store(attribute, anim_out)
                .map_err(E::Attribute)?;
        }
    }
    if reverting {
        processed.physics_state_fields_mut().start_revert = Some(revert_direction);
    }
    // The Vault adapter takes its first lock only at the first source lookup.
    processed
        .try_finish_jump_events(&mut runtime_input, &mut Vault(owner))
        .map_err(E::Finish)?;
    processed
        .try_finish_animation_attribute_flags(input)
        .map_err(E::Attribute)
}

/// Exact dot3, two VMX RSQRT refinements, zero selection then reciprocal-dt
/// multiplication at82b5a5e4..620. Estimate is sampled even for squared zero.
fn push_speed<N: ReckoningNumeric>(
    position: [f32; 3],
    reciprocal_dt: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let value = [position[0], position[1], position[2], 0.0];
    let squared = numeric.dot3(value, value)?;
    let mut inverse = numeric.reciprocal_sqrt_estimate(squared)?;
    for _ in 0..2 {
        let square = inverse * inverse;
        let half = inverse * 0.5;
        let residual = -squared.mul_add(square, -1.0);
        inverse = half.mul_add(residual, inverse);
    }
    let length = squared * inverse;
    let selected = if squared == 0.0 { 0.0 } else { length };
    Ok(reciprocal_dt * selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Numeric {
        calls: Vec<&'static str>,
        fail: bool,
    }
    impl ReckoningNumeric for Numeric {
        type Error = &'static str;
        fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
            self.calls.push("dot3");
            Ok(crate::reckoning_numeric::host_dot3(a, b))
        }
        fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
            self.calls.push("rsqrt");
            if self.fail {
                Err("estimate")
            } else {
                Ok(1.0 / value.sqrt())
            }
        }
        fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("push uses scalar division")
        }
        fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
            panic!("push has no conversion")
        }
    }
    #[test]
    fn zero_length_still_samples_estimate_before_zero_select() {
        let mut numeric = Numeric {
            calls: vec![],
            fail: true,
        };
        assert_eq!(push_speed([0.0; 3], 2.0, &mut numeric), Err("estimate"));
        assert_eq!(numeric.calls, ["dot3", "rsqrt"]);
        numeric.fail = false;
        assert_eq!(
            push_speed([0.0; 3], 2.0, &mut numeric).unwrap().to_bits(),
            0
        );
        assert_eq!(
            push_speed([3.0, 4.0, 0.0], 0.5, &mut numeric)
                .unwrap()
                .to_bits(),
            2.5_f32.to_bits()
        );
    }
}
