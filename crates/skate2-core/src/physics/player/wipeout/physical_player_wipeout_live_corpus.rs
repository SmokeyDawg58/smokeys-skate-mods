//! Independent original-data bail probe; explicit transition selection is test policy.
use super::*;

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_wipeout_ticks_retain_actual_prediction_and_post_flags() {
    with_original_inputs(ground_startup::check_wipeout_ticks);
}

#[derive(Clone, Copy)]
struct Prior {
    over: bool,
    slow: bool,
    settled: f32,
    time: f32,
    marker: Option<[f32; 4]>,
    estimate: f32,
}
#[derive(Default)]
pub(super) struct WipeoutProbe {
    prior: Option<Prior>,
    harvests: usize,
    consumptions: usize,
    completed: usize,
}
impl WipeoutProbe {
    pub(super) fn observe(
        &mut self,
        frame: usize,
        boundary: crate::actor_construction::selected_tick::SelectedTickStage,
        world: &PhysicalAssemblyWorld,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
    ) {
        use crate::{
            actor_construction::selected_tick::SelectedTickStage as S,
            physical_player_physics_state::CurrentPhysicalState as State,
        };
        let state = owner.physics_state().lock().unwrap();
        // Wait for an actual source-selected transition; ordinary Ground ticks
        // must retain their native scheduling and producers.
        if state.current != State::WipeoutGround {
            assert_eq!(state.current, State::Ground);
            assert_eq!(self.completed, 0);
            assert!(self.prior.is_none());
            return;
        }
        match boundary {
            S::BeforeActor => {
                let f = &state.wipeout_ground.as_ref().unwrap().fields;
                assert!(f.prediction.ready);
                self.prior = Some(Prior {
                    over: f.over,
                    slow: f.slow,
                    settled: f.time_settled,
                    time: f.time_in_state,
                    marker: f.prediction.collision_time,
                    estimate: f.estimated_time_to_collision,
                });
            }
            S::BeforeSolver => {
                assert_eq!(
                    state.current,
                    State::WipeoutGround,
                    "controlled source transition must reach Wipeout Update"
                );
                let f = &state.wipeout_ground.as_ref().unwrap().fields;
                assert!(!f.prediction.ready, "submission cannot publish readiness");
                assert!(world.prediction_owner().unwrap().pending_count().unwrap() > 0);
                assert!(f.time_in_state > 0.0);
                if let Some(prior) = self.prior {
                    assert_eq!(
                        (f.over, f.slow),
                        (prior.over, prior.slow),
                        "post flags retained into next Update"
                    );
                    let settled = if prior.over {
                        prior.settled + f32::from_bits(0x3c88_8889)
                    } else {
                        0.0
                    };
                    assert_eq!(
                        f.time_settled.to_bits(),
                        settled.to_bits(),
                        "native recovery consumes previous over flag"
                    );
                    assert!(f.time_in_state > prior.time);
                    let marker = prior.marker.expect("previous actual harvested marker");
                    let expected = if marker.iter().all(|t| *t >= 0.0) {
                        marker[0]
                    } else {
                        prior.estimate
                    };
                    assert_eq!(
                        f.estimated_time_to_collision.to_bits(),
                        expected.to_bits(),
                        "same prior future consumed before resubmission"
                    );
                    self.consumptions += 1;
                    eprintln!(
                        "wipeout frame {frame}: consumed prior over={} slow={}, settled {} -> {}",
                        prior.over, prior.slow, prior.settled, f.time_settled
                    );
                }
            }
            S::AfterSolver => {
                assert_eq!(state.current, State::WipeoutGround);
                assert!(
                    state
                        .wipeout_ground
                        .as_ref()
                        .unwrap()
                        .fields
                        .prediction
                        .ready,
                    "actual submitted Wipeout future must be harvested before solver returns"
                );
                self.harvests += 1;
            }
            S::BeforePreSolve => {}
        }
    }
    pub(super) fn after_tick(&mut self, frame: usize, owner: &Arc<PhysicalPlayerAssemblyOwner>) {
        let state = owner.physics_state().lock().unwrap();
        if state.current == crate::physical_player_physics_state::CurrentPhysicalState::Ground {
            assert_eq!(self.completed, 0);
            assert!(self.prior.is_none());
            return;
        }
        assert_eq!(
            state.current,
            crate::physical_player_physics_state::CurrentPhysicalState::WipeoutGround
        );
        let f = &state.wipeout_ground.as_ref().unwrap().fields;
        eprintln!(
            "wipeout frame {frame}: selected Update -> solver -> selected PostPhysics -> PostWipeout/Update5 returned; over={} slow={} time={}",
            f.over, f.slow, f.time_in_state
        );
        self.completed += 1;
    }
    pub(super) fn is_complete(&self) -> bool {
        self.completed == 3
    }
    pub(super) fn finish(&self) {
        assert_eq!(
            (self.completed, self.harvests, self.consumptions),
            (3, 3, 2)
        );
    }
}
