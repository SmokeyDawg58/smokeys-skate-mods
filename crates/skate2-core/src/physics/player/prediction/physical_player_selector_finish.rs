//! Selector.Update82dc6804..6910 on the same completed winner.
use super::{
    OnboardTrajectoryOwner,
    selector_numeric::{adjust, collision_position},
    selector_update::SelectorError,
};
use crate::{
    reckoning_numeric::{ReckoningNumeric, try_unsigned_angle},
    skeleton_root_transforms::SkeletonRootVector as V,
};

pub(crate) trait FinishSource {
    type Error;
    type Layout;
    fn layout(&mut self) -> Result<Self::Layout, Self::Error>;
    fn scalar(&mut self, layout: &Self::Layout, offset: usize) -> Result<f32, Self::Error>;
    fn graph(&mut self, layout: &Self::Layout, offset: usize, x: f32) -> Result<f32, Self::Error>;
    fn up(&mut self) -> Result<V, Self::Error>;
    fn ground_normal(&mut self) -> Result<V, Self::Error>;
    fn show_wall(&mut self) -> Result<bool, Self::Error>;
    fn show_trajectories(&mut self) -> Result<bool, Self::Error>;
    fn show_select(&mut self) -> Result<bool, Self::Error>;
    fn overlay(&mut self) -> Result<(), Self::Error>;
}
impl OnboardTrajectoryOwner {
    /// `CalculateCOMTrajectory82dc02f8`. Keep p+(COM-p), including rounding.
    fn calculate_com<S: FinishSource, N: ReckoningNumeric>(
        &mut self,
        src: &mut S,
        n: &mut N,
    ) -> Result<(), SelectorError<S::Error, N::Error>> {
        use SelectorError::{Numeric, Source};
        let y = self.winning_data.landing_normal.xyz[1];
        let layout = src.layout().map_err(Source)?;
        let blend = src.graph(&layout, 0x60, y).map_err(Source)?;
        if self.winning_result.is_none() {
            return Err(SelectorError::Missing("winning result buffer"));
        }
        self.winning_com = self.winning_buffer.trajectory;
        let delta = self
            .start_trajectory_position
            .zip(self.launch.com_position, |a, b| a - b);
        let up = src.up().map_err(Source)?;
        let height = n.dot_vector(up, delta).map_err(Numeric)?.abs();
        let offset = self
            .launch
            .com_position
            .zip(self.winning_com.position, |a, b| a - b);
        let desired = self
            .winning_data
            .landing_normal
            .map(|v| v * (height * blend));
        let layout = src.layout().map_err(Source)?;
        let frame = self.winning_buffer.frame;
        // Native lfs0478 precedes BLE047c, including frame<=0.
        let maximum = src.scalar(&layout, 0x1f4).map_err(Source)?;
        if frame > 0 {
            let adjustment = desired.zip(offset, |a, b| a - b);
            self.winning_com.position = self.winning_com.position.zip(offset, |a, b| a + b);
            adjust(&mut self.winning_com, frame, adjustment, maximum, n)?;
        }
        if src.show_wall().map_err(Source)? {
            src.overlay().map_err(Source)?;
        }
        if src.show_trajectories().map_err(Source)? {
            src.overlay().map_err(Source)?;
            src.overlay().map_err(Source)?;
        }
        Ok(())
    }
    /// `IsSecondPassRequired82db98a0`. AI return precedes targeting-grind mutation.
    fn second_pass<S: FinishSource, N: ReckoningNumeric>(
        &mut self,
        src: &mut S,
        n: &mut N,
    ) -> Result<bool, SelectorError<S::Error, N::Error>> {
        use SelectorError::{Numeric, Source};
        if self.ai_trajectory_following {
            return Ok(false);
        }
        let layout = src.layout().map_err(Source)?;
        let minimum = src.scalar(&layout, 0x1f0).map_err(Source)?;
        if self.best_trajectory < 0 {
            return Err(SelectorError::Missing("nonnegative winning trajectory"));
        }
        let ground = src.ground_normal().map_err(Source)?;
        let angle =
            try_unsigned_angle(self.winning_data.landing_normal, ground, n).map_err(Numeric)?;
        let grind = self.grind_bonus_awarded && self.best_trajectory == 0;
        self.targeting_grind = grind;
        // Native BLT9978: unordered angle does not reject a second pass.
        Ok(self.pass == 1
            && self.num_trajectories >= 2
            && angle.partial_cmp(&minimum) != Some(std::cmp::Ordering::Less)
            && !grind)
    }
    /// Returns whether runtime must run existing private Launch and then set pass2.
    pub(crate) fn try_finish_completed_query<S: FinishSource, N: ReckoningNumeric>(
        &mut self,
        src: &mut S,
        n: &mut N,
    ) -> Result<bool, SelectorError<S::Error, N::Error>> {
        use SelectorError::Source;
        self.calculate_com(src, n)?;
        let second = self.second_pass(src, n)?;
        if src.show_select().map_err(Source)? {
            // GetWinningTrajectory82db6338 and Update's debug tail. Native
            // stripped drawing bodies retain only assertions/overlay lookups.
            if self.best_trajectory < 0 {
                return Err(SelectorError::Missing("nonnegative winning trajectory"));
            }
            let _ = collision_position(&self.winning_buffer)?;
            src.overlay().map_err(Source)?;
            for _ in 0..self.num_trajectories {
                src.overlay().map_err(Source)?;
            }
            if self.best_trajectory < 0 {
                return Err(SelectorError::Missing("nonnegative winning trajectory"));
            }
            // DebugDrawTrajectory82dc0b18: full time gate, overlay then position.
            if self.winning_buffer.time.iter().all(|t| *t >= 0.0) {
                src.overlay().map_err(Source)?;
                let _ = collision_position(&self.winning_buffer)?;
            }
        }
        self.ready = true; //68fc, before LaunchSecondPass and its fallible work
        if second {
            self.launch.start_velocity = self.winning_data.velocity; //4010
            let layout = src.layout().map_err(Source)?;
            self.launch.cone_angle_x = src.scalar(&layout, 0x244).map_err(Source)?;
            let layout = src.layout().map_err(Source)?;
            self.launch.cone_angle_z = src.scalar(&layout, 0x23c).map_err(Source)?;
        }
        Ok(second)
    }
}
#[cfg(test)]
#[path = "physical_player_selector_finish_tests.rs"]
mod tests;
