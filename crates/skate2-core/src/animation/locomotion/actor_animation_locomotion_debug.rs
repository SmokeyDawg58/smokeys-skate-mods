//! DebugDraw82c98538 and its source numeric/no-op draw children.
use super::{AnimationLocomotionTrajectory, locomotion_producer::TrajectoryProducerError};
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector as V,
};
const DT: f32 = f32::from_bits(0x3c888889);
#[derive(Debug)]
pub(super) enum LocomotionDebugError<N> {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Numeric(N),
    Ratio(TrajectoryProducerError<std::convert::Infallible>),
    Index(usize),
    UnknownBezierStartW,
    BezierNan,
    NonProgressingLoop,
}
fn add(a: V, b: V) -> V {
    a.zip(b, |a, b| a + b)
}
fn bezier<N: ReckoningNumeric>(
    p0: V,
    s0: V,
    p1: V,
    s1: V,
    length: f32,
    cvars: &InputCvarReferences,
    overlay: &DebugOverlaySlot,
    numeric: &mut N,
) -> Result<(), LocomotionDebugError<N::Error>> {
    use LocomotionDebugError as E;
    overlay.require().map_err(E::Overlay)?;
    // Only native-overwritten handle W lanes are made known. Original P0.W is
    // used by Bezier4 duration arithmetic; retain its undefined-value boundary.
    let p0 = p0.known_lanes().ok_or(E::UnknownBezierStartW)?;
    let s0 = [s0.xyz[0], s0.xyz[1], s0.xyz[2], DT];
    let p1 = [p1.xyz[0], p1.xyz[1], p1.xyz[2], length - DT];
    let s1 = [s1.xyz[0], s1.xyz[1], s1.xyz[2], length];
    let limit = (length - DT) + f32::from_bits(0x3727c5ac);
    let mut time = DT;
    if limit > DT {
        let inverse_length = 1.0 / length;
        loop {
            let influence = cvars.try_bezier_influence_percentage().map_err(E::Cvar)?;
            let result = super::physics_update_data::trajectory::bezier4(
                p0,
                s0,
                p1,
                s1,
                inverse_length * time,
                influence,
            );
            if result.iter().any(|v| v.is_nan()) {
                return Err(E::BezierNan);
            }
            if result[3] != 0.0 {
                let mut inverse = numeric.reciprocal_estimate(result[3]).map_err(E::Numeric)?;
                for _ in 0..2 {
                    let residual = -inverse.mul_add(result[3], -1.0);
                    inverse = inverse.mul_add(residual, inverse);
                }
                let _corrected: [f32; 4] = std::array::from_fn(|lane| {
                    inverse.mul_add((result[lane] - p0[lane]) * time, p0[lane])
                });
            }
            let next = time + DT;
            if next.to_bits() == time.to_bits() {
                return Err(E::NonProgressingLoop);
            }
            time = next;
            if !(time < limit) {
                break;
            }
        }
    }
    let influence = cvars.try_bezier_influence_percentage().map_err(E::Cvar)?;
    let end = super::physics_update_data::trajectory::bezier4(p0, s0, p1, s1, 1.0, influence);
    if end.iter().any(|v| v.is_nan()) {
        return Err(E::BezierNan);
    }
    Ok(())
}
fn parabola<N>(duration: f32, overlay: &DebugOverlaySlot) -> Result<(), LocomotionDebugError<N>> {
    use LocomotionDebugError as E;
    overlay.require().map_err(E::Overlay)?;
    let limit = (duration - DT) + f32::from_bits(0x3727c5ac);
    let mut time = DT;
    if limit > DT {
        loop {
            let next = time + DT;
            if next.to_bits() == time.to_bits() {
                return Err(E::NonProgressingLoop);
            }
            time = next;
            if !(time < limit) {
                break;
            }
        }
    }
    Ok(())
}
impl AnimationLocomotionTrajectory {
    /// Source color choices only feed folded draw leaves; caller color schema
    /// has no retained effect. Two feet, two Beziers then parabola, then ratio.
    pub(super) fn try_debug_draw_locomotion<N: ReckoningNumeric>(
        &self,
        base: usize,
        physical: bool,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), LocomotionDebugError<N::Error>> {
        use LocomotionDebugError as E;
        if !cvars.try_loco_display_debug().map_err(E::Cvar)? {
            return Ok(());
        }
        overlay.require().map_err(E::Overlay)?;
        if base >= 4 {
            return Err(E::Index(base));
        }
        let feet = if physical {
            &self.physical_feet
        } else {
            &self.input_feet
        };
        for offset in 0..2 {
            let foot = feet[(base + offset) % 4];
            for state in &foot.states[..2] {
                let start = add(state.vectors[0], foot.data.position);
                let end = add(state.vectors[2], start);
                bezier(
                    start,
                    state.vectors[1],
                    end,
                    state.vectors[3],
                    state.time,
                    cvars,
                    overlay,
                    numeric,
                )?;
            }
            let _start = add(foot.states[2].vectors[0], foot.data.position);
            parabola(foot.states[2].time, overlay)?;
        }
        self.try_current_time_ratio().map_err(E::Ratio)?;
        //82c98828 calls825187c0 (empty AIProcess::OnEnd debug render leaf).
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{input_cvar_runtime::InputCvarManager, reckoning_numeric::HostReckoningNumeric};
    #[test]
    fn disabled_debug_skips_uninitialized_ratio_and_enabled_requires_real_overlay() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let mut trajectory = AnimationLocomotionTrajectory::new();
        trajectory.first_update = false;
        let overlay = DebugOverlaySlot::default();
        trajectory
            .try_debug_draw_locomotion(0, true, &refs, &overlay, &mut HostReckoningNumeric)
            .unwrap();
        manager
            .try_deserialize_startup(Some(b"[Config]\nkDisplayLocomotionDebug=1\n"))
            .unwrap();
        assert!(matches!(
            trajectory.try_debug_draw_locomotion(
                0,
                true,
                &refs,
                &overlay,
                &mut HostReckoningNumeric
            ),
            Err(LocomotionDebugError::Overlay(DebugOverlayError::Missing))
        ));
    }
    #[test]
    fn enabled_debug_executes_children_before_current_ratio_assertion() {
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        manager
            .try_deserialize_startup(Some(b"[Config]\nkDisplayLocomotionDebug=1\n"))
            .unwrap();
        let mut trajectory = AnimationLocomotionTrajectory::new();
        let overlay = DebugOverlaySlot::default();
        overlay.try_construct().unwrap();
        trajectory
            .try_debug_draw_locomotion(0, true, &refs, &overlay, &mut HostReckoningNumeric)
            .unwrap();
        trajectory.first_update = false;
        assert!(matches!(
            trajectory.try_debug_draw_locomotion(
                0,
                true,
                &refs,
                &overlay,
                &mut HostReckoningNumeric
            ),
            Err(LocomotionDebugError::Ratio(_))
        ));
    }
}
