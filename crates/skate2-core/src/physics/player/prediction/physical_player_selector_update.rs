//! Actual retained selector.Update82dc6528; no parallel selector state.
use super::OnboardTrajectoryOwner;
use super::selector_scoring::{add_wall_scores, calculate_scores};
use crate::{
    physical_player_trajectory_owners::RetainedPredictionResult,
    reckoning_numeric::ReckoningNumeric, skeleton_root_transforms::SkeletonRootVector as V,
};

/// Required host choice. Exclusion is never represented as a native query miss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectorGrindPolicy {
    Native,
    ParkedForFirstSkatingTest,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectorProgress {
    Retained(bool),
    NearLanding,
    BeforeCom,
}
#[derive(Debug)]
pub(crate) enum SelectorError<E, N> {
    Source(E),
    Numeric(N),
    Missing(&'static str),
    FutureNotReady(usize),
    CollisionTime(u32),
    NegativeMaximum(u32),
    DebugDrawPending,
}
impl<E: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for SelectorError<E, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "selector source: {e:?}"),
            Self::Numeric(e) => write!(f, "selector numeric: {e:?}"),
            Self::Missing(name) => write!(f, "selector missing {name}"),
            Self::FutureNotReady(i) => write!(f, "selector future {i} is not ready"),
            Self::CollisionTime(bits) => {
                write!(f, "selector collision time assertion ({bits:08x})")
            }
            Self::NegativeMaximum(bits) => write!(f, "selector negative maximum ({bits:08x})"),
            Self::DebugDrawPending => f.write_str("selector raw trajectory drawing pending"),
        }
    }
}
/// Cached layout values are captured once where native retains its layout pointer;
/// separate scalar/layout calls preserve source sampling elsewhere.
pub(crate) trait SelectorSource {
    type Error;
    type Layout;
    fn minimum_frames(&mut self) -> Result<i32, Self::Error>;
    fn layout(&mut self) -> Result<Self::Layout, Self::Error>;
    fn scalar(&mut self, layout: &Self::Layout, offset: usize) -> Result<f32, Self::Error>;
    fn graph(
        &mut self,
        layout: &Self::Layout,
        offset: usize,
        count: usize,
        x: f32,
    ) -> Result<f32, Self::Error>;
    fn feet(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn central(&mut self) -> Result<bool, Self::Error>;
    fn raw_debug(&mut self) -> Result<bool, Self::Error>;
    fn wall_debug(&mut self) -> Result<bool, Self::Error>;
    fn overlay(&mut self) -> Result<(), Self::Error>;
    fn up(&mut self) -> Result<V, Self::Error>;
    fn transition(&mut self) -> Result<f32, Self::Error>;
    fn ground_normal(&mut self) -> Result<V, Self::Error>;
    fn ground_velocity(&mut self) -> Result<V, Self::Error>;
    fn line(&mut self, start: V, end: V) -> Result<Option<(V, V)>, Self::Error>;
    /// `ProcessedPhysIn` +8c8 bit 30 (`mIsAIPathFollowing`) and the AI
    /// trajectory flags +6b0: whether the central candidate is grind scored.
    fn grind_scoring_allowed(&mut self) -> Result<bool, Self::Error>;
    /// +8cc bit 27: grinds are avoided, so only the penalty applies.
    fn avoid_grinds(&mut self) -> Result<bool, Self::Error>;
    /// `CalculateGrindScore` 0x82dc5c38.
    fn grind_score(
        &mut self,
        selector: &mut OnboardTrajectoryOwner,
        index: usize,
        landing_velocity: V,
    ) -> Result<f32, Self::Error>;
    /// `CalculateGrindPenalty` 0x82db9ee0.
    fn grind_penalty(
        &mut self,
        selector: &OnboardTrajectoryOwner,
        index: usize,
    ) -> Result<f32, Self::Error>;
}
pub(super) fn count<E, N>(s: &OnboardTrajectoryOwner) -> Result<usize, SelectorError<E, N>> {
    if s.num_trajectories <= 0 {
        return Ok(0);
    }
    let n = usize::try_from(s.num_trajectories).expect("positive signed count");
    if n > 7 {
        return Err(SelectorError::Missing("native candidate capacity"));
    }
    Ok(n)
}
pub(super) fn ready<E, N>(
    s: &OnboardTrajectoryOwner,
    i: usize,
) -> Result<RetainedPredictionResult, SelectorError<E, N>> {
    if !s.queries[i].ready {
        return Err(SelectorError::FutureNotReady(i));
    }
    Ok(s.queries[i].result)
}
impl OnboardTrajectoryOwner {
    pub(crate) fn try_complete_query_prefix<S: SelectorSource, N: ReckoningNumeric>(
        &mut self,
        source: &mut S,
        numeric: &mut N,
        policy: SelectorGrindPolicy,
    ) -> Result<SelectorProgress, SelectorError<S::Error, N::Error>> {
        use SelectorError::Source;
        let started = self.just_started_query;
        self.has_just_changed = false;
        if !started || !self.queries[0].ready {
            return Ok(SelectorProgress::Retained(self.ready));
        }
        if self.pass == 2 {
            self.has_just_changed = true;
        }
        if source.raw_debug().map_err(Source)? && self.pass == 0 {
            return Err(SelectorError::DebugDrawPending);
        }
        self.just_started_query = false;
        self.best_trajectory = 0;
        self.winning_result = None;
        self.pass = self.pass.wrapping_add(1);
        calculate_scores(self, source, numeric, policy)?;
        self.grind_trajectory_results.clear();
        add_wall_scores(self, source)?;
        let mut best = -100_000_000.0;
        for i in 0..count(self)? {
            if self.scores[i] > best {
                best = self.scores[i];
                self.best_trajectory = i16::try_from(i).expect("seven candidates");
            }
        }
        let i = usize::try_from(self.best_trajectory)
            .map_err(|_| SelectorError::Missing("signed winner"))?;
        self.winning_buffer = ready(self, i)?;
        // Native pointer targets winning_buffer; i is only provenance metadata.
        self.winning_result = Some(i);
        self.winning_data = self.data[i];
        self.targeting_wallride = self.is_wallride[i];
        let layout = source.layout().map_err(Source)?;
        let minimum = source.scalar(&layout, 0x1e8).map_err(Source)?;
        let t = self.winning_buffer.time[0];
        if std::env::var_os("SKATE2_TRACE_TRAJECTORY").is_some() {
            eprintln!(
                "trajectory selector: pass={} candidates={} winner={} time={} minimum={} landing={:?} start_com={:?} start_velocity={:?} scores={:?} times={:?}",
                self.pass,
                self.num_trajectories,
                i,
                t,
                minimum,
                self.winning_data.landing_position.xyz,
                self.launch.com_position.xyz,
                self.launch.start_velocity.xyz,
                self.scores,
                self.queries
                    .iter()
                    .map(|q| (q.ready, q.result.time[0], q.result.frame))
                    .collect::<Vec<_>>()
            );
        }
        if t.abs().partial_cmp(&100.0) != Some(std::cmp::Ordering::Less) {
            return Err(SelectorError::CollisionTime(t.to_bits()));
        }
        if t < minimum {
            self.suggested_landing_normal = self.winning_buffer.normal;
            self.ready = false;
            self.suggested_landing_normal_valid = true;
            self.has_just_changed = false;
            return Ok(SelectorProgress::NearLanding);
        }
        // Source continues into COM/second-pass. No premature ready publication.
        Ok(SelectorProgress::BeforeCom)
    }
}
#[cfg(test)]
#[path = "physical_player_selector_update_tests.rs"]
pub(super) mod tests;
