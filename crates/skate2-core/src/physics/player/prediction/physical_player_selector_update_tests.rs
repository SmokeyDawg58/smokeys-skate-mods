#![expect(clippy::float_cmp, reason = "exact fixture publications")]
use super::*;
use crate::{
    physical_player_trajectory_owners::TrajectoryOwnerBindings,
    reckoning_numeric::HostReckoningNumeric, skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::{Arc, Mutex, Weak};
pub(crate) fn owner() -> OnboardTrajectoryOwner {
    let mut s = OnboardTrajectoryOwner::new(
        TrajectoryOwnerBindings {
            owner: Weak::new(),
            processed: Arc::new(Mutex::new(SkeletonProcessedContact::new())),
        },
        false,
    );
    s.num_trajectories = 2;
    s.just_started_query = true;
    for q in &mut s.queries[..2] {
        q.ready = true;
        q.result.time = [0.25; 4];
        q.result.frame = 15;
        q.result.ground_normal = V::known([0.0, 1.0, 0.0, 0.0]);
        q.result.trajectory.position = V::known([0.0; 4]);
        q.result.trajectory.velocity = V::known([1.0, 0.0, 0.0, 0.0]);
        q.result.trajectory.gravity = V::known([0.0; 4]);
    }
    s
}
pub(crate) struct Source {
    pub(crate) minimum: i32,
    pub(crate) near: f32,
    pub(crate) calls: Vec<String>,
    pub(crate) fail_scalar: Option<usize>,
    pub(crate) hit: Option<(V, V)>,
    pub(crate) scalar_nan: Option<usize>,
    pub(crate) transition: f32,
    pub(crate) grind: f32,
    pub(crate) penalty: f32,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            minimum: 1,
            near: 0.5,
            calls: Vec::new(),
            fail_scalar: None,
            hit: None,
            scalar_nan: None,
            transition: f32::NAN,
            grind: 0.0,
            penalty: 0.0,
        }
    }
}
impl SelectorSource for Source {
    type Error = &'static str;
    type Layout = ();
    fn minimum_frames(&mut self) -> Result<i32, Self::Error> {
        self.calls.push("frames".into());
        Ok(self.minimum)
    }
    fn layout(&mut self) -> Result<(), Self::Error> {
        self.calls.push("layout".into());
        Ok(())
    }
    fn scalar(&mut self, (): &(), o: usize) -> Result<f32, Self::Error> {
        self.calls.push(format!("scalar{o:x}"));
        if self.fail_scalar == Some(o) {
            return Err("scalar");
        }
        if self.scalar_nan == Some(o) {
            return Ok(f32::NAN);
        }
        Ok(match o {
            0x1e8 => self.near,
            0x1a0 => 0.5,
            0x1c4 | 0x1c8 => 2.0,
            0x1a8 | 0x1a4 => 1.0,
            0x1f4 => 100.0,
            _ => 0.0,
        })
    }
    fn graph(&mut self, (): &(), o: usize, _: usize, _: f32) -> Result<f32, Self::Error> {
        self.calls.push(format!("graph{o:x}"));
        Ok(0.0)
    }
    fn feet(&mut self, key: u64) -> Result<f32, Self::Error> {
        self.calls.push(format!("feet{key:x}"));
        Ok(0.5)
    }
    fn central(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("central".into());
        Ok(false)
    }
    fn raw_debug(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("raw".into());
        Ok(false)
    }
    fn wall_debug(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("wall".into());
        Ok(false)
    }
    fn overlay(&mut self) -> Result<(), Self::Error> {
        panic!("disabled debug")
    }
    fn up(&mut self) -> Result<V, Self::Error> {
        self.calls.push("up".into());
        Ok(V::known([0.0, 0.0, 1.0, 8.0]))
    }
    fn transition(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("transition".into());
        Ok(self.transition)
    }
    fn ground_normal(&mut self) -> Result<V, Self::Error> {
        Ok(V::known([1.0, 0.0, 0.0, 0.0]))
    }
    fn ground_velocity(&mut self) -> Result<V, Self::Error> {
        Ok(V::known([1.0, 0.0, 0.0, 0.0]))
    }
    fn line(&mut self, start: V, _: V) -> Result<Option<(V, V)>, Self::Error> {
        self.calls.push(format!("lineY{}", start.xyz[1]));
        Ok(self.hit)
    }
    fn grind_scoring_allowed(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("grindAllowed".into());
        Ok(true)
    }
    fn avoid_grinds(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("avoidGrinds".into());
        Ok(false)
    }
    fn grind_score(
        &mut self,
        selector: &mut OnboardTrajectoryOwner,
        _: usize,
        _: V,
    ) -> Result<f32, Self::Error> {
        self.calls.push("grindScore".into());
        selector.grind_bonus_awarded = self.grind > 0.0;
        Ok(self.grind)
    }
    fn grind_penalty(&mut self, _: &OnboardTrajectoryOwner, _: usize) -> Result<f32, Self::Error> {
        self.calls.push("grindPenalty".into());
        Ok(self.penalty)
    }
}
#[test]
fn not_ready_clears_changed_without_sampling() {
    let mut s = owner();
    s.queries[0].ready = false;
    s.ready = true;
    s.has_just_changed = true;
    let mut src = Source::default();
    assert_eq!(
        s.try_complete_query_prefix(
            &mut src,
            &mut HostReckoningNumeric,
            SelectorGrindPolicy::ParkedForFirstSkatingTest
        )
        .unwrap(),
        SelectorProgress::Retained(true)
    );
    assert!(!s.has_just_changed);
    assert!(src.calls.is_empty());
}
#[test]
fn stable_winner_is_copied_before_near_landing() {
    let mut s = owner();
    s.pass = 2;
    let mut src = Source::default();
    assert_eq!(
        s.try_complete_query_prefix(
            &mut src,
            &mut HostReckoningNumeric,
            SelectorGrindPolicy::ParkedForFirstSkatingTest
        )
        .unwrap(),
        SelectorProgress::NearLanding
    );
    assert_eq!(s.winning_result, Some(0));
    assert!(!s.ready && !s.has_just_changed && s.suggested_landing_normal_valid);
    s.queries[0].result.normal.xyz[0] = 9.0;
    assert_eq!(s.winning_buffer.normal.xyz[0], 0.0);
    assert_eq!(src.calls.iter().filter(|x| *x == "frames").count(), 2);
}
#[test]
fn native_grind_scores_only_the_central_candidate() {
    let mut s = owner();
    let mut src = Source {
        grind: 5.0,
        ..Source::default()
    };
    assert_eq!(
        s.try_complete_query_prefix(
            &mut src,
            &mut HostReckoningNumeric,
            SelectorGrindPolicy::Native
        )
        .unwrap(),
        SelectorProgress::NearLanding
    );
    assert_eq!(s.scores[..2], [6.0, 1.0]);
    assert_eq!(s.winning_result, Some(0));
    assert!(s.grind_bonus_awarded);
    let grinds: Vec<&str> = src
        .calls
        .iter()
        .map(String::as_str)
        .filter(|x| x.starts_with("grind") || x.starts_with("avoid"))
        .collect();
    assert_eq!(
        grinds,
        [
            "grindAllowed",
            "grindScore",
            "avoidGrinds",
            "avoidGrinds",
            "grindPenalty"
        ]
    );
}
#[test]
fn normal_path_never_publishes_ready_before_com() {
    let mut s = owner();
    let mut src = Source {
        near: 0.1,
        ..Source::default()
    };
    assert_eq!(
        s.try_complete_query_prefix(
            &mut src,
            &mut HostReckoningNumeric,
            SelectorGrindPolicy::ParkedForFirstSkatingTest
        )
        .unwrap(),
        SelectorProgress::BeforeCom
    );
    assert!(!s.ready);
}
