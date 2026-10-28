//! The `PhysOut` values `PhysOutModule_Hud::Update` (`0x827c79b8`) reads for its statistics.
use super::{
    super::scoring::{AirPeriod, Scorable, Scorables},
    Conditioners, PhysicalPlayerPhysOut,
    scoring_attributes::{ScoreError, scalar},
};
use crate::{
    physics::FilteredPhysicsState,
    physics_state_ids::PHYSICS_STATE_SKITCHING,
    reckoning_numeric::ReckoningVector,
    skater_manager_local_identity::stats::{
        StatThresholds,
        hud::{EndedTrick, HudAir, HudSequence, HudStatsFrame},
    },
};

/// `Sk8::Score::IsLateFlipTrick` (`0x82dee1d8`): a valid scorable whose base is another one, the
/// late flip (128).
fn is_late_flip(id: i32) -> bool {
    id != 128 && super::scorable_id_base(id) == Some(128)
}

/// A `Scorables` set's used entries.
fn entries(set: &Scorables) -> Result<&[Scorable], ScoreError> {
    usize::try_from(set.num_entries)
        .ok()
        .and_then(|count| set.elements.get(..count))
        .ok_or(ScoreError::Invalid("HUD stats scorable count"))
}

/// `UpdateTrickCompletion`'s ended trick: `GetTrickType` 2 is a flip trick, a late one by
/// `IsLateFlipTrick`; invalid settings read scorable -1.
fn ended(trick: &Scorable) -> EndedTrick {
    let settings = &trick.trick_scoring_settings;
    let id = if settings.is_valid { settings.scorable_id } else { -1 };
    if super::scorable_id_trick_type(id).unwrap_or(0) != 2 {
        EndedTrick::Other
    } else if is_late_flip(id) {
        EndedTrick::LateFlip
    } else {
        EndedTrick::Flip
    }
}

/// `UpdateAirFinished`'s landing and `UpdateAirContextData` (`0x827c6c40`)'s context: the
/// accumulated context less the take-off's and the landing's, whose group 1 and group 0 first banks
/// count the vehicles and pedestrians jumped over, and `GetEntityTypeCount(3)` (`0x82de8ae8`) of
/// group 0's the security guards.
fn air(air: &AirPeriod) -> Result<HudAir, ScoreError> {
    let mut over = air.accumulated_context_data.clone();
    over.remove(&air.initial_context_data)?;
    over.remove(&air.end_context_data)?;
    let pedestrians = &over.context_sensitive_data[0].uid_data[0];
    let vehicles = &over.context_sensitive_data[1].uid_data[0];
    let security = usize::try_from(pedestrians.count)
        .ok()
        .and_then(|count| pedestrians.uids.get(..count))
        .ok_or(ScoreError::Invalid("HUD stats context count"))?
        .iter()
        .filter(|uid| uid.entity_type == 3)
        .count();
    Ok(HudAir {
        landed: air.landed_succesfully,
        total_height_gain: air.total_height_gain,
        in_air_frames: air.in_air_frames,
        height_to_peak: air.height_to_peak,
        horizontal_distance: air.horizontal_distance,
        grass_gap: air.air_gap_data[0].accumulated_gap_distance,
        stair_gap: air.air_gap_data[1].accumulated_gap_distance,
        grab_time: air.grab_time,
        flips: air.player_num_flips,
        vehicles_over: vehicles.count,
        pedestrians_over: pedestrians.count,
        security_over: u32::try_from(security).map_err(|_| ScoreError::Invalid("HUD stats security count"))?,
    })
}

/// `mGrabSplineDesc.mObjectID` as the unsigned ID `GetGrabSplineOwnerObjectID` takes.
#[expect(clippy::cast_sign_loss, reason = "the native unsigned object ID")]
const fn object_id(id: i32) -> u32 {
    id as u32
}

fn missing<E>(component: &'static str) -> impl FnOnce(E) -> ScoreError {
    move |_| ScoreError::Missing(component)
}

impl Conditioners {
    pub(crate) fn try_hud_stats_frame(&self, output: &PhysicalPlayerPhysOut) -> Result<HudStatsFrame, ScoreError> {
        output.check_current_guard().map_err(ScoreError::Access)?;
        if !self.matches_output(output) {
            return Err(ScoreError::Invalid("HUD stats foreign PhysOut"));
        }
        let retained = &self.scoring.published_value().ok_or(ScoreError::Missing("HUD stats scoring owner"))?.payload;
        let module = retained.module.published_value().ok_or(ScoreError::Missing("HUD stats score module"))?;
        let out = output.concrete.scoring.as_deref().ok_or(ScoreError::Missing("HUD stats Scoring2"))?;
        let state = output.try_state_output().map_err(missing("HUD stats State"))?;
        let anim = output.try_animation_output().map_err(missing("HUD stats Animation"))?;
        let filtered = output.try_filtered_output().map_err(missing("HUD stats Filtered"))?.filtered_physics_state;
        let wipeout_over = output.try_skeleton_output().map_err(missing("HUD stats Skeleton"))?.is_wipeout_over;
        let grabbing = output.try_offboard_output().map_err(missing("HUD stats OffBoard"))?.is_grabbing_object;
        let (centre, velocity) =
            output.try_system_centre_and_damped_velocity().map_err(missing("HUD stats SystemReckoning"))?;
        let begun = entries(&out.tricks_begun)?.first().map(|trick| trick.trick_scoring_settings.is_valid);
        let converted = if begun.is_none() {
            entries(&out.tricks_converted_to)?.first().map(|trick| trick.trick_scoring_settings.is_valid)
        } else {
            None
        };
        let ended_tricks = entries(&out.tricks_ended)?;
        let cancelled = entries(&out.tricks_cancelled)?.iter().filter(|trick| !trick.was_a_tap_trick).count();
        Ok(HudStatsFrame {
            wiping_out: state.is_wiping_out,
            physics_state: state.physics_state,
            filtered_ground: filtered == FilteredPhysicsState::Ground,
            filtered_offboard: filtered == FilteredPhysicsState::OffBoard,
            wipeout_over,
            push_mongo: anim.push_mongo,
            push_regular: anim.push_regular,
            centre_of_mass_height: centre.operand()[1],
            damped_velocity: velocity.operand(),
            distance: out.distance_travelled_this_frame,
            offboard_not_hippy_jumping: out.is_offboard_and_not_hippy_jumping,
            running_out: out.is_running_out,
            offboard_duration: out.current_offboard_duration_in_seconds,
            attempting_body_flip: out.is_attempting_a_body_flip,
            cancelled: u32::try_from(cancelled).map_err(|_| ScoreError::Invalid("HUD stats cancelled count"))?,
            tricks_ended: !ended_tricks.is_empty(),
            ended: ended_tricks.iter().filter(|trick| !trick.was_a_tap_trick).map(ended).collect(),
            begun,
            converted,
            aerial: out.is_in_state_air || out.is_hippy_jumping,
            spins: if out.is_hippy_jumping { out.hippy_jump180_count } else { out.current_air_data.player_spin180_count },
            sequence_begun: out.sequence_begun,
            wipeout_interrupted_trick: out.wipeout_interrupted_trick,
            just_wiped_out: out.just_wiped_out,
            handplant: out.just_finished_handplant.then_some(out.finished_handplant_duration_in_seconds),
            manual: out.just_finished_manual.then_some(out.finished_manual_length),
            grind: out.just_finished_grind.then_some(out.finished_grind_info.length),
            powerslide: out.just_finished_powerslide.then_some(out.finished_powerslide_length),
            air: if out.air_finished { Some(air(&out.air_finished_data)?) } else { None },
            sequence: out.sequence_completed.then_some(HudSequence {
                successful: out.sequence_completed_successfully,
                score: out.finished_sequence_score,
                tricks: out.finished_sequence_trick_count,
                tapped: out.finished_sequence_tapped_trick_count,
                duration: out.finished_sequence_duration_in_seconds,
                objects: out.finished_sequence_dynamic_object_set.is_set,
            }),
            line_score: out.line_score,
            line_timer_points: out.line_timer_points,
            grab_spline: (state.physics_state != PHYSICS_STATE_SKITCHING && state.is_offboard && grabbing)
                .then_some(object_id(state.grab_spline_desc.object_id)),
            thresholds: StatThresholds::try_read(|key| scalar(&module.general, key))?,
        })
    }
}
