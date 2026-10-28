//! Live CheckForBadLandingWipeout, 82dd9528..9988; COM/deck velocity selected by caller.
//! Child of `ground_wipeout_runtime`: reuse its actual Source.
//! 97ac raw141ff533 is normal times negative dot plus velocity, fused per lane.
//! 97f8/9874/98b0/9924 BO4 BI25 skip when CR6.GT clear, including NaN.
//! Graph9844 is the retained Instance cached layout (no null fallback).
use super::{
    GroundWipeoutRuntimeError, GroundWipeoutSource, ReckoningNumeric, SkateboardAttributeRead,
    SkeletonRootVector, Source,
};
use crate::{physics::PointGraph, skateboard_attributes::SkateboardAttributeClass};

const MIN_SPEED: u64 = 0xcc33_ef98_5fb1_7f0d;
const HEAVY: u64 = 0x3e46_f877_8d1a_97c4;
const STAIR_HEAVY: u64 = 0x0cca_453d_e3f7_25bf;
const HARD_GRIND: u64 = 0x3275_07d7_c7f8_eff6;
const MAX_SPEED: u64 = 0xb1a0_d1ff_6075_3b23;
const MAX_ANGLE: u64 = 0x7699_ad57_d672_86ac;

impl<N: ReckoningNumeric> Source<'_, N> {
    fn landing_attribute(&mut self, key: u64) -> Result<f32, GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_ground_force_board(&self.world.backend, self.board, |_, _, attrs| {
                attrs.read_float(SkateboardAttributeClass::Wipeout, key)
            })
            .map_err(E::BoardAttributes)?
            .map_err(E::Attribute)
    }
    fn landing_graph(&mut self) -> Result<PointGraph<8>, GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_ground_force_board(&self.world.backend, self.board, |_, _, attrs| {
                attrs.try_wipeout_landing_graph()
            })
            .map_err(E::BoardAttributes)?
            .map_err(E::Attribute)?
            .ok_or(E::Missing("physics_wipeout cached PointGraph8"))
    }
    fn landing_request(
        &mut self,
        index: usize,
        value: f32,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_wipeout_request_value(index, value)
            .map_err(E::Owner)
    }
    fn landing_debug(&mut self) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        if crate::input_cvar_runtime::wipeout_decision::WipeoutDecisionCvars::new(
            &self.owner.inputs().cvars,
        )
        .display_debug()
        .map_err(GroundWipeoutRuntimeError::Cvar)?
        {
            // Raw98a8/98c8/9944 only require singleton; no draw call follows.
            self.overlay
                .require()
                .map_err(GroundWipeoutRuntimeError::Overlay)?;
        }
        Ok(())
    }
}

fn ordered_above(value: f32, threshold: f32) -> bool {
    value > threshold
}
fn tangent(
    normal: SkeletonRootVector,
    velocity: SkeletonRootVector,
    downward: f32,
) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| normal.xyz[i].mul_add(downward, velocity.xyz[i])),
        w: normal
            .w
            .zip(velocity.w)
            .map(|(n, v)| n.mul_add(downward, v)),
    }
}

/// The caller retains the validated SAME board/skeleton/owner Source used by
/// Air.CheckForAirWipeout. This is the actual false/use-board-velocity branch.
pub(super) fn try_bad_landing<N: ReckoningNumeric>(
    source: &mut Source<'_, N>,
    use_com: bool,
) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
    use GroundWipeoutRuntimeError as E;
    if source.num_parts_in_contact()? == 0 {
        return Ok(());
    }
    // Preserve all four pointer/read operations even on the grind branch.
    let minimum = source.landing_attribute(MIN_SPEED)?;
    let ordinary = source.landing_attribute(HEAVY)?;
    let stairs = source.landing_attribute(STAIR_HEAVY)?;
    let hard_grind = source.landing_attribute(HARD_GRIND)?;
    let heavy = if source.board_contact_flag(0x8000_0000)? {
        stairs
    } else {
        ordinary
    };
    // Raw82dd9700..971c chooses Processed+260 for true, +190 for false.
    let velocity = source.processed(|p| {
        if use_com {
            p.prior_output().centre_of_mass_velocity().map(|v| v.value)
        } else {
            p.prior_output().velocity().map(|v| v.value)
        }
    })?;
    let targeting_grind = {
        let trajectories = source
            .owner
            .trajectories()
            .lock()
            .map_err(|_| E::Poisoned)?;
        trajectories
            .onboard
            .as_ref()
            .ok_or(E::Missing("onboard selector"))?
            .targeting_grind
    };
    let mut normal = source
        .contacts()?
        .lock()
        .map_err(|_| E::Poisoned)?
        .collision_state
        .average_normal
        .ok_or(E::Missing("board averageGroundNormal"))?;
    if targeting_grind {
        {
            let trajectories = source
                .owner
                .trajectories()
                .lock()
                .map_err(|_| E::Poisoned)?;
            let selector = trajectories
                .onboard
                .as_ref()
                .ok_or(E::Missing("onboard selector"))?;
            if selector.most_vertical_grind_normal_valid {
                normal = SkeletonRootVector::known(selector.most_vertical_grind_normal);
            }
        }
        let downward = -source.dot(normal, velocity)?;
        if ordered_above(downward, hard_grind) {
            source.landing_debug()?;
            source.landing_request(8, 0.0)?;
        }
        return Ok(());
    }
    let downward = -source.dot(normal, velocity)?;
    let speed = source.length(tangent(normal, velocity, downward))?;
    if ordered_above(speed, minimum) {
        let maximum = source.landing_attribute(MAX_SPEED)?;
        let angle = source.landing_attribute(MAX_ANGLE)?;
        let graph = source.landing_graph()?;
        let ratio = (speed - minimum) / (maximum - minimum);
        let sample = graph.evaluate(ratio);
        let surface_angle = source.processed(|p| p.prior_output().landing_surface_angle())?;
        let limit = sample.mul_add(std::f32::consts::FRAC_PI_2 - angle, angle);
        if ordered_above(surface_angle.abs(), limit) {
            source.landing_request(5, 0.0)?; //987c..9890 BEFORE debug read9894.
        }
        source.landing_debug()?;
    }
    if ordered_above(downward, heavy) {
        source.landing_debug()?; //98b4..98c8 BEFORE valued request98cc.
        source.landing_request(6, downward)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "physical_player_air_bad_landing_tests.rs"]
mod tests;
