use super::*;
use crate::physical_player_phys_out::simple::Animation;
use std::sync::Arc;

fn pending() -> ScoreHolder {
    let vault = Arc::new(crate::attribute_runtime::tests::graph(0, vec![]));
    let mut holder = ScoreHolder::before_sequence(Arc::new(()), &vault);
    holder.air_sequence_reward_pending = true;
    holder.air_sequence_reward_pending_frame_count = 2;
    holder
}
#[test]
fn delayed_landing_analysis_is_retained_until_reward_is_due() {
    let mut holder = pending();
    let mut animation = Animation::reset_values();
    assert!(!holder.check_air_reward(&animation, 0.5, false).unwrap());
    animation.just_analyzed_sketchyness = true;
    animation.sketchyness_anim_result = 1;
    animation.sketchyness_side_speed = 1.; // absent Vault threshold is native zero
    animation.sketchyness_anim_scalar = -0.6;
    assert!(!holder.check_air_reward(&animation, 0.5, false).unwrap());
    assert!(holder.pending_air_period.just_landed);
    assert!(holder.pending_air_period.landed_succesfully);
    assert!(holder.pending_air_period.landing_was_sketchy);
    animation.just_analyzed_sketchyness = false;
    assert!(holder.check_air_reward(&animation, 0.5, false).unwrap());
    assert!(holder.pending_air_period.landing_was_sketchy);
}
#[test]
fn equality_and_spin_landings_do_not_receive_sketchy_penalty() {
    let mut holder = pending();
    let mut animation = Animation::reset_values();
    animation.just_analyzed_sketchyness = true;
    animation.sketchyness_anim_result = 1;
    animation.sketchyness_side_speed = 1.;
    animation.sketchyness_anim_scalar = 0.5;
    holder.check_air_reward(&animation, 0.5, false).unwrap();
    assert!(!holder.pending_air_period.landing_was_sketchy);
    animation.sketchyness_anim_result = 2;
    animation.sketchyness_anim_scalar = 1.;
    holder.check_air_reward(&animation, 0.5, false).unwrap();
    assert!(!holder.pending_air_period.landing_was_sketchy);
    holder.check_air_reward(&animation, 0.5, true).unwrap();
    assert!(holder.pending_air_period.landing_was_sketchy);
}
#[test]
fn no_pending_air_does_not_consume_landing_or_countdown() {
    let mut holder = pending();
    holder.air_sequence_reward_pending = false;
    let mut animation = Animation::reset_values();
    animation.just_analyzed_sketchyness = true;
    animation.sketchyness_anim_result = 99;
    assert!(!holder.check_air_reward(&animation, 0.5, true).unwrap());
    assert_eq!(holder.air_sequence_reward_pending_frame_count, 2);
    assert!(!holder.pending_air_period.just_landed);
}
