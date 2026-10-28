//! `TrajectorySelector::AdjustStartingVelOnVerts`0x82dbfbc0 on the retained owner.
use crate::physics_state_ids::PHYSICS_STATE_PHYSICS_AIR;
use super::OnboardTrajectoryOwner;
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    reckoning_attributes::{ReckoningAttributeRead, ReckoningAttributeScope},
    reckoning_numeric::{
        ReckoningNumeric, length_from_squared_and_estimate, try_length, try_normalize_safe,
    },
    skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};
use std::sync::{Arc, Mutex};

const BLEND: u64 = 0x0b42_dfbb_7f77_56a0;
const GROUND_Y_LIMIT: u64 = 0x9492_3d13_6ea2_da37;
const VELOCITY_Y_LIMIT: u64 = 0x58bf_c8d4_1216_c7ee;
const TRANSITION_SCALE: u64 = 0x723f_f102_0686_22cf;
const ZERO: Vector = Vector::known([0.0; 4]);

#[cfg(test)]
#[path = "physical_player_trajectory_adjust_start_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum AdjustStartError<S, N> {
    Source(S),
    Numeric(N),
}
impl<S: std::fmt::Display, N: std::fmt::Display> std::fmt::Display for AdjustStartError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => e.fmt(f),
            Self::Numeric(e) => write!(f, "trajectory adjustment numeric: {e}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum AdjustLiveError<A> {
    Missing(&'static str),
    ForeignOwner,
    Poisoned,
    Attribute(A),
    Layout(AttributeRuntimeError),
    Overlay(DebugOverlayError),
}
impl<A: std::fmt::Display> std::fmt::Display for AdjustLiveError<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "trajectory adjustment missing {s}"),
            Self::ForeignOwner => f.write_str("trajectory adjustment owner mismatch"),
            Self::Poisoned => f.write_str("trajectory adjustment processed lock poisoned"),
            Self::Attribute(e) => e.fmt(f),
            Self::Layout(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
        }
    }
}
trait AdjustSource {
    type Error;
    fn ground_normal(&mut self) -> Result<Vector, Self::Error>;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn transition(&mut self) -> Result<f32, Self::Error>;
    fn physics_state(&mut self) -> Result<i32, Self::Error>;
    fn trajectory_scalar(&mut self) -> Result<f32, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
fn transition_factor(value: f32) -> f32 {
    let shifted = value - 0.25;
    // Two source FSELs, including unordered selection (NaN ends at +1).
    let lower = if -1.0 - shifted >= 0.0 { -1.0 } else { shifted };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

/// Only the source mOnVert/startVel fields mutate. Private Launch is not run.
fn adjust<S: AdjustSource, N: ReckoningNumeric>(
    on_vert: &mut bool,
    start: &mut Vector,
    player_jumped: bool,
    source: &mut S,
    numeric: &mut N,
) -> Result<(), AdjustStartError<S::Error, N::Error>> {
    use AdjustStartError::{Numeric, Source};
    *on_vert = false; //fbf4, before even the first processed read
    let ground_y = source.ground_normal().map_err(Source)?.xyz[1];
    let blend = source.scalar(BLEND).map_err(Source)?;
    let ground_y_limit = source.scalar(GROUND_Y_LIMIT).map_err(Source)?;
    let velocity_y_limit = source.scalar(VELOCITY_Y_LIMIT).map_err(Source)?;
    let original = *start;
    let original_unit = try_normalize_safe(original, ZERO, numeric).map_err(Numeric)?;
    let normal = source.ground_normal().map_err(Source)?;
    let dot = numeric.dot_vector(original, normal).map_err(Numeric)?;
    let mut projected = normal.map(|lane| lane * dot);
    projected.xyz[1] = 0.0; //fd90; only this lane overwritten
    let vertical = Vector::known([0.0, original.xyz[1], 0.0, 0.0]);
    let component = vertical.zip(projected, |a, b| a + b); //fd98 VA61 from raw17ddd83d
    let component_unit = try_normalize_safe(component, ZERO, numeric).map_err(Numeric)?;
    let component_squared = numeric.dot_vector(component, component).map_err(Numeric)?;
    let remainder = original.zip(component, |a, b| a - b);
    let transition = source.transition().map_err(Source)?;
    let estimate = numeric
        .reciprocal_sqrt_estimate(component_squared)
        .map_err(Numeric)?;
    let factor = transition_factor(transition);
    let component_speed = length_from_squared_and_estimate(component_squared, estimate);
    let scale = source.scalar(TRANSITION_SCALE).map_err(Source)?;
    let normal = source.ground_normal().map_err(Source)?;
    let force = normal.map(|lane| (lane * factor) * scale);
    let desired = Vector::known([0.0, 1.0, 0.0, 0.0]).zip(force, |a, b| a - b);
    let desired_unit = try_normalize_safe(desired, ZERO, numeric).map_err(Numeric)?;
    // Raw BGE is LT-clear, including unordered; all prefix work still ran.
    if ground_y.partial_cmp(&ground_y_limit) != Some(std::cmp::Ordering::Less) {
        return Ok(());
    }
    // Raw BLE takes GT-clear, including unordered, to the non-vert arm.
    if component_unit.xyz[1] > velocity_y_limit {
        *on_vert = true; //ff4c before the remaining numeric operations
        let redirected = desired_unit.zip(remainder, |a, b| a.mul_add(component_speed, b));
        let old_weight = 1.0 - blend;
        let old_direction = original_unit.map(|lane| lane * old_weight); //ffD8 VA40
        let mixed = redirected.zip(old_direction, |a, b| a.mul_add(blend, b));
        let speed = try_length(*start, numeric).map_err(Numeric)?;
        let direction = try_normalize_safe(mixed, ZERO, numeric).map_err(Numeric)?;
        *start = direction.map(|lane| lane * speed); //0058 single final full store
    } else if !player_jumped && source.physics_state().map_err(Source)? != PHYSICS_STATE_PHYSICS_AIR {
        let velocity = *start;
        let normal = source.ground_normal().map_err(Source)?;
        let dot = numeric.dot_vector(velocity, normal).map_err(Numeric)?;
        let projection = normal.map(|lane| lane * dot);
        let tangent = velocity.zip(projection, |a, b| a - b);
        let normal_scale = source.trajectory_scalar().map_err(Source)?;
        *start = projection.zip(tangent, |a, b| a.mul_add(normal_scale, b)); //00f8
        source.require_overlay().map_err(Source)?; //00fc, unconditional after store
    }
    Ok(())
}

struct LiveSource<'a, A> {
    processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    attributes: &'a mut A,
    trajectory: &'a WallRideTrajectoryAttributes,
    overlay: &'a DebugOverlaySlot,
}
impl<A: ReckoningAttributeRead> AdjustSource for LiveSource<'_, A> {
    type Error = AdjustLiveError<A::Error>;
    fn ground_normal(&mut self) -> Result<Vector, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .ground_normal()
            .map(|v| v.value)
            .ok_or(Self::Error::Missing("groundNormal1d0"))
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        let pointer = self
            .attributes
            .lookup(ReckoningAttributeScope::Global, key)
            .map_err(Self::Error::Attribute)?;
        self.attributes
            .float(&pointer)
            .map_err(Self::Error::Attribute)
    }
    fn transition(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .input_controls()
            .transition
            .ok_or(Self::Error::Missing("transition95c"))
    }
    fn physics_state(&mut self) -> Result<i32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .physics_state_fields()
            .physics
            .ok_or(Self::Error::Missing("physicsState8e4"))
    }
    fn trajectory_scalar(&mut self) -> Result<f32, Self::Error> {
        let layout = self.trajectory.capture().map_err(Self::Error::Layout)?;
        let bytes = layout
            .as_ref()
            .map_or(&[0; 0x24c][..], AttributeRuntimeData::bytes);
        Ok(f32::from_be_bytes(
            bytes[0x1e4..0x1e8]
                .try_into()
                .expect("checked trajectory layout"),
        ))
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
}
impl OnboardTrajectoryOwner {
    pub(crate) fn try_adjust_starting_velocity<A: ReckoningAttributeRead, N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        attributes: &mut A,
        trajectory: &WallRideTrajectoryAttributes,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), AdjustStartError<AdjustLiveError<A::Error>, N::Error>> {
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| AdjustStartError::Source(AdjustLiveError::Poisoned))?
                .processed(),
        );
        if !self.bindings.matches(owner, &processed) {
            return Err(AdjustStartError::Source(AdjustLiveError::ForeignOwner));
        }
        adjust(
            &mut self.on_vert,
            &mut self.launch.start_velocity,
            self.launch.player_jumped,
            &mut LiveSource {
                processed: &processed,
                attributes,
                trajectory,
                overlay,
            },
            numeric,
        )
    }
}
