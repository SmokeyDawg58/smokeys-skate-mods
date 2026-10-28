//! Actual `WipeoutGround::UpdateWipeoutControl` 0x82d8edb0.
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::{
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
            wipeout::WipeoutSkeletonError,
        },
        skeleton_normal::part_transform_query::SkeletonPartTransformQueryError,
    },
    input_cvar_runtime::{
        InputCvarError, InputCvarReferences, wipeout_control::WipeoutControlFloat as Float,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::{
        ReckoningClampError, ReckoningNumeric, cross_typed, try_clamp, try_normalize_safe,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector as Vector},
};
use std::sync::Arc;
#[cfg(test)]
#[path = "physical_player_wipeout_control_tests.rs"]
mod tests;
#[derive(Debug)]
pub(crate) enum WipeoutControlError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Part(SkeletonPartTransformQueryError),
    Numeric(N),
    Clamp(ReckoningClampError<N>),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Torque(WipeoutSkeletonError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for WipeoutControlError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(e) => write!(f, "wipeout control missing: {e}"),
            Self::Poisoned => f.write_str("wipeout control poisoned lock"),
            Self::ForeignState => f.write_str("wipeout control foreign state"),
            Self::Pair(e) => write!(f, "wipeout control pair: {e}"),
            Self::Part(e) => write!(f, "wipeout control part: {e}"),
            Self::Numeric(e) => write!(f, "wipeout control numeric: {e:?}"),
            Self::Clamp(e) => write!(f, "wipeout control clamp: {e:?}"),
            Self::Cvar(e) => write!(f, "wipeout control cvar: {e:?}"),
            Self::Overlay(e) => write!(f, "wipeout control overlay: {e:?}"),
            Self::Torque(e) => write!(f, "wipeout control torque: {e}"),
        }
    }
}

struct LiveControl<'a, N> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    owner: Arc<PhysicalPlayerAssemblyOwner>,
    numeric: &'a mut N,
}
impl<N: ReckoningNumeric> LiveControl<'_, N> {
    fn transform(
        &mut self,
        part: usize,
    ) -> Result<SkeletonRootMatrix, WipeoutControlError<N::Error>> {
        use WipeoutControlError as E;
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(
                &self.world.backend,
                self.board,
                self.skeleton,
                |normal, _| normal.try_query_part_transform(part),
            )
            .map_err(E::Pair)?
            .map_err(E::Part)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "ordered native control path and failure prefixes"
    )]
    fn run(
        &mut self,
        in_contact: bool,
        overlay: &DebugOverlaySlot,
    ) -> Result<bool, WipeoutControlError<N::Error>> {
        use WipeoutControlError as E;
        let hips = self.transform(23)?;
        let spine = self.transform(11)?;
        let axis = try_normalize_safe(
            spine.columns()[3].zip(hips.columns()[3], |a, b| a - b),
            [0.0; 4],
            self.numeric,
        )
        .map_err(E::Numeric)?;
        let angular = self
            .world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(
                &self.world.backend,
                self.board,
                self.skeleton,
                |normal, _| normal.try_calculate_wipeout_angular_velocity(self.numeric),
            )
            .map_err(E::Pair)?
            .map_err(E::Numeric)?;
        self.owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .wipeout_ground
            .as_mut()
            .ok_or(E::Missing("WipeoutGround"))?
            .fields
            .body_angular_velocity = angular;
        if in_contact {
            return Ok(false);
        }
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let velocity = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .centre_of_mass_velocity()
            .ok_or(E::Missing("Processed COM velocity260"))?
            .value;
        let velocity = try_normalize_safe(velocity, [0.0; 4], self.numeric).map_err(E::Numeric)?;
        let (control_y, control_x) = {
            let p = processed.lock().map_err(|_| E::Poisoned)?;
            let a = p.animation_attribute_fields();
            (
                a.wipeout_y.ok_or(E::Missing("wipeout Y"))?,
                a.wipeout_x.ok_or(E::Missing("wipeout X"))?,
            )
        };
        let hips = self.transform(23)?;
        let auto = try_auto_control(
            axis,
            velocity,
            hips.columns()[1],
            control_x,
            &self.owner.inputs().cvars,
            self.numeric,
        )?;
        // Native reload precedes the two max-angle pointer captures.
        let angular = self
            .owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .wipeout_ground
            .as_ref()
            .ok_or(E::Missing("WipeoutGround"))?
            .fields
            .body_angular_velocity;
        let torque = try_control_torque(
            axis,
            auto,
            angular,
            control_x,
            control_y,
            &self.owner.inputs().cvars,
            self.numeric,
        )?;
        if self
            .owner
            .inputs()
            .cvars
            .try_wipeout_debug_display()
            .map_err(E::Cvar)?
        {
            overlay.require().map_err(E::Overlay)?;
        }
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_apply_pair_wipeout_torque(
                &mut self.world.backend,
                self.board,
                self.skeleton,
                torque,
                self.numeric,
            )
            .map_err(E::Torque)?;
        Ok(true)
    }
}
#[derive(Clone, Copy)]
struct AutoControl {
    side: Vector,
    correction: Vector,
}
fn try_auto_control<N: ReckoningNumeric>(
    axis: Vector,
    velocity: Vector,
    hips_y: Vector,
    control_x: f32,
    refs: &InputCvarReferences,
    n: &mut N,
) -> Result<AutoControl, WipeoutControlError<N::Error>> {
    use WipeoutControlError as E;
    let up = Vector::known([0.0, 1.0, 0.0, 0.0]);
    let mut side = try_normalize_safe(cross_typed(up, axis), [0.0; 4], n).map_err(E::Numeric)?;
    side.xyz[1] = 0.0;
    side = try_normalize_safe(side, [0.0; 4], n).map_err(E::Numeric)?;
    let velocity_cross = cross_typed(up, velocity);
    if 0.0 > n.dot_vector(velocity_cross, side).map_err(E::Numeric)? {
        #[expect(
            clippy::neg_multiply,
            reason = "preserve native floating-point multiply rather than sign-bit operation"
        )]
        {
            side = side.map(|v| v * -1.0);
        }
    }
    let velocity_axis = try_normalize_safe(velocity_cross, [0.0; 4], n).map_err(E::Numeric)?;
    let dot = n.dot_vector(velocity_axis, axis).map_err(E::Numeric)?;
    let projected = axis.zip(velocity_axis.map(|v| v * dot), |a, b| a - b);
    let projected = try_normalize_safe(projected, [0.0; 4], n).map_err(E::Numeric)?;
    let auto_z = refs
        .try_capture_wipeout_control_float(Float::AutoZ)
        .map_err(E::Cvar)?
        .read()
        .map_err(E::Cvar)?;
    let minimum = refs
        .try_capture_wipeout_control_float(Float::MinimumControl)
        .map_err(E::Cvar)?
        .read()
        .map_err(E::Cvar)?;
    let mut correction = cross_typed(axis, projected).map(|v| v * auto_z);
    if control_x.abs() < minimum && hips_y.xyz[1] < 0.0 {
        let dot = n.dot_vector(velocity_axis, hips_y).map_err(E::Numeric)?;
        let projected = hips_y.zip(velocity_axis.map(|v| v * dot), |a, b| a - b);
        let projected = try_normalize_safe(projected, [0.0; 4], n).map_err(E::Numeric)?;
        let auto_y = refs
            .try_capture_wipeout_control_float(Float::AutoY)
            .map_err(E::Cvar)?
            .read()
            .map_err(E::Cvar)?;
        correction =
            cross_typed(hips_y, projected).zip(correction, |v, old| v.mul_add(auto_y, old));
    }
    Ok(AutoControl { side, correction })
}
fn try_control_torque<N: ReckoningNumeric>(
    axis: Vector,
    auto: AutoControl,
    angular: Vector,
    control_x: f32,
    control_y: f32,
    refs: &InputCvarReferences,
    n: &mut N,
) -> Result<Vector, WipeoutControlError<N::Error>> {
    use WipeoutControlError as E;
    let x = refs
        .try_capture_wipeout_control_float(Float::MaxAngleX)
        .map_err(E::Cvar)?;
    let y = refs
        .try_capture_wipeout_control_float(Float::MaxAngleY)
        .map_err(E::Cvar)?;
    let x = x.read().map_err(E::Cvar)?;
    let y = y.read().map_err(E::Cvar)?;
    let radians = f32::from_bits(0x3c8e_fa35);
    let side_angle = (x * radians) * control_y;
    let axis_angle = (y * radians) * control_x;
    let force = refs
        .try_capture_wipeout_control_float(Float::TorqueForce)
        .map_err(E::Cvar)?;
    let side = auto.side.map(|v| v * side_angle);
    let maximum = refs
        .try_capture_wipeout_control_float(Float::MaxTorque)
        .map_err(E::Cvar)?;
    let negative_axis = axis.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
    let desired = negative_axis
        .zip(side, |a, b| a.mul_add(axis_angle, b))
        .zip(auto.correction, |a, b| a + b)
        .map(|v| v * f32::from_bits(0x426f_ffff));
    let difference = desired.zip(angular, |a, b| a - b);
    let force = force.read().map_err(E::Cvar)?;
    let maximum = maximum.read().map_err(E::Cvar)?;
    let torque = difference.map(|v| (v * force) * f32::from_bits(0x3c88_8889));
    try_clamp(torque, maximum, n).map_err(E::Clamp)
}
impl PhysicalAssemblyWorld {
    /// The contact argument is the caller's collision snapshot, sampled by Update.
    pub(crate) fn try_update_pair_wipeout_control<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        in_contact: bool,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<bool, WipeoutControlError<N::Error>> {
        use WipeoutControlError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if state.current != CurrentPhysicalState::WipeoutGround {
                return Err(E::ForeignState);
            }
            let state = state
                .wipeout_ground
                .as_ref()
                .ok_or(E::Missing("WipeoutGround"))?;
            if !state.bindings.board.same_as(board)
                || !state.bindings.skeleton.same_as(skeleton)
                || !state
                    .bindings
                    .owner
                    .upgrade()
                    .is_some_and(|p| Arc::ptr_eq(&p, &owner))
            {
                return Err(E::ForeignState);
            }
        }
        LiveControl {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
        }
        .run(in_contact, overlay)
    }
}
