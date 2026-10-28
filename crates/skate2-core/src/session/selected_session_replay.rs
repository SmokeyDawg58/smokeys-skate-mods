//! `DataSysMan`'s replay half: each simulation update's presentation submission kept for the
//! challenge replay recorder (`DataMan::GatherAllData` `0x82997bf8`), and a replay-controlled
//! NIS's restore of the recorded frames (`SceneReplayTimeController`).
use super::{AudioSkater, AudioPedestrian, AudioVehicle, AudioDynamicObject, SelectedLiveError, SelectedLiveSession, SelectedLiveStage, career::CareerSession};
use crate::{
    challenge::replay::ReplaySystem,
    presentation_restore::{PresentationRestorer, PresentationSubmission},
};
use std::{collections::BTreeMap, sync::Arc};

/// The AudioPresentation and VDWriter data submitted beside the visual frame.
/// Audio is kept before FillSkaterData's deterministic packing, so the audio
/// host uses the same material/vault conversion for live and recorded frames.
#[derive(Clone)]
pub struct AudioFrame {
    pub frame: u32,
    pub camera_at: [f32; 3],
    pub camera_position: [f32; 3],
    pub skaters: Vec<AudioSkater>,
    pub reverb_key: u64,
    pub ambience_key: u64,
    pub emitter_key: u64,
    pub out_of_boundary_amount: Option<f32>,
    pub fade_amount: Option<f32>,
    pub pedestrians: Vec<AudioPedestrian>,
    pub vehicles: Vec<AudioVehicle>,
    pub dynamic_objects: Vec<AudioDynamicObject>,
    pub local_race_ranking: i32,
    /// GlobalData challenge key/mode and player bits, resolved through the
    /// immutable challenge attributes like AudioSystem::UpdatePresentationData.
    pub challenge: (i32, Option<i32>, bool, i32),
    pub challenge_names: (bool, bool, bool),
    pub player_flags: (bool, bool),
    pub security: SecurityPresentation,
}

/// ChaseMan.GetAudioState8270fe90, packed by AudioPresentation.DoSimSubmit,
/// plus VDWriter._PrepareTazerData827fc768. Kept with the replay frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SecurityPresentation {
    pub chasers: u32,
    /// Native tChaseAudio_Data flags (NSZ, chase, warning, guard, street,
    /// capture and NSZ capture in bits7..1).
    pub flags: u8,
    pub tazer: bool,
    pub bodyguard: bool,
}

/// VDWriter::_PrepareBailData 827fcd40 / VDReader::ReadData 827fdd78.
#[derive(Clone, Copy, Debug)]
pub struct BailVisual {
    pub time: f32,
    pub score: u32,
    pub effect_enabled: bool,
    pub tazer: bool,
}

impl AudioFrame {
    fn bail_visual(&self) -> Option<BailVisual> {
        let local = &self.skaters.iter().rev().find(|s| s.local)?.phys_out;
        if local.overall_wipeout_duration <= 0.0 && !self.security.tazer { return None; }
        Some(BailVisual { time: local.overall_wipeout_duration,
            score: local.hom_score, effect_enabled: local.hom_effect_enabled, tazer: self.security.tazer })
    }
}

pub(super) struct ReplayFrame {
    pub(super) visual: PresentationSubmission,
    pub(super) audio: Option<AudioFrame>,
    pub(super) world: RecordedWorld,
    pub(super) local_skater:usize,
    pub(super) players_recipe:Vec<bool>,
}

pub(super) struct RecordedWorld {
    pub dynamic_objects:Vec<crate::SelectedDynamicObjectVisual>,
    pub vehicles:Vec<crate::SelectedVehicleVisual>,
    pub pedestrians:Vec<crate::SelectedPedestrianVisual>,
    pub traffic_lights:[[f32;4];2],
    /// PC Object Dropper: the local board's physics deck and every dropped copy of it this
    /// frame, which the replay draws the copies from as the live game does.
    pub local_board_deck:Option<[f32;16]>,
    pub dropped_boards:Vec<[f32;16]>,
}

impl<T, C> SelectedLiveSession<T, C> {
    /// RealTimeController::GetTickCount 8298fd30 and the replay controller's
    /// GetTickCount 82872720. Rate is signed here; AudioPresentation takes abs.
    pub fn audio_playback(&self) -> (bool, f32) {
        if let Some(editor)=&self.replay_editor {return (true,editor.rate);}
        if let Some(rate) = self.career.as_ref().and_then(|c| c.nis.replay_speed()) {
            return (true, rate);
        }
        let rate = self.inner.world_handle().and_then(|world| {
            let world = world.lock().ok()?;
            let rate = world.sim_rate.lock().ok()?.current_sim_update_rate();
            Some(if rate > 0.0 { crate::simulation_sim_rate::UPDATE_TIME / rate } else { 0.0 })
        }).unwrap_or(1.0);
        (false, rate)
    }

    /// RestoreData.mIsRewinding is the controller's direction, independent of
    /// its playback rate (a photo selection may jump while rate is zero).
    pub fn audio_rewinding(&self) -> bool {
        self.audio_playback().0 && self.replay_clock.is_some_and(|clock| clock.rewinding)
    }

    fn live_audio_frame(&self) -> Option<AudioFrame> {
        let (frame, camera_at, camera_position) = self.audio_camera()?;
        Some(AudioFrame {
            frame, camera_at, camera_position, skaters: self.audio_roster()?,
            reverb_key: self.audio_reverb_key().unwrap_or(0),
            ambience_key: self.audio_ambience_key().unwrap_or(0),
            emitter_key: self.audio_emitter_key().unwrap_or(0),
            out_of_boundary_amount: self.out_of_boundary_amount(),
            fade_amount: self.audio_fade_amount(),
            pedestrians: self.audio_pedestrians(), vehicles: self.audio_vehicles(),
            dynamic_objects: self.audio_dynamic_objects(),
            local_race_ranking: self.audio_local_race_ranking(),
            challenge: self.audio_challenge(), challenge_names: self.audio_challenge_names(),
            player_flags: self.audio_player_flags(),
            security: self.live_security_presentation().unwrap_or_default(),
        })
    }

    /// AudioPresentation::DoSimRestore 827f5c60: copy the selected frame's
    /// records; interpolate only the four wheel positions by player ID.
    /// CameraPresentation still uses the scene's current camera in replays.
    pub fn audio_frame(&self) -> Option<AudioFrame> {
        if !self.audio_playback().0 { return self.live_audio_frame(); }
        let clock = self.replay_clock.as_ref()?;
        let mut out = self.recorded_frames().get(&clock.current)?.audio.clone()?;
        if let Some(previous) = self.replay_index().previous_frame(clock.current)
            .and_then(|f| self.recorded_frames().get(&f)).and_then(|f| f.audio.as_ref()) {
            for skater in &mut out.skaters {
                if let Some(before) = previous.skaters.iter().find(|s| s.player_index == skater.player_index) {
                    for (wheel, previous) in skater.phys_out.wheel_pos.iter_mut().zip(before.phys_out.wheel_pos) {
                        for (value, previous) in wheel.iter_mut().zip(previous) {
                            *value = (*value - previous).mul_add(clock.fraction, previous);
                        }
                    }
                }
            }
        }
        if let Some(matrix) = self.replay_editor.as_ref().and_then(|editor|editor.camera.matrix)
            .or_else(||self.career.as_ref().and_then(|career|career.nis.view().camera).and_then(|camera|camera.matrix)) {
            out.camera_at = [matrix[8], matrix[9], matrix[10]];
            out.camera_position = [matrix[12], matrix[13], matrix[14]];
        } else if let Some(camera) = self.replay_restorer.camera() {
            out.camera_at = camera.forward;
            out.camera_position = camera.position;
        }
        Some(out)
    }

    pub(super) fn bail_visual(&self) -> Option<BailVisual> {
        if self.audio_playback().0 {
            let clock = self.replay_clock.as_ref()?;
            return self.recorded_frames().get(&clock.current)?.audio.as_ref()?.bail_visual();
        }
        let actor = self.audio_local_actor()?;
        let physical = actor.cached_physical_allocation().ok()?;
        let output = physical.owner().phys_out().lock().ok()?;
        let local = output.try_audio_phys_out().ok()?;
        drop(output);
        let tazer = self.live_security_presentation().is_some_and(|state| state.tazer);
        if local.overall_wipeout_duration <= 0.0 && !tazer { return None; }
        Some(BailVisual { time: local.overall_wipeout_duration,
            score: local.hom_score, effect_enabled: local.hom_effect_enabled, tazer })
    }

    fn live_security_presentation(&self) -> Option<SecurityPresentation> {
        let owner = self.inner.world.as_ref()?;
        let identity = owner.world.lock().ok()?.local_skater_identity.clone()?;
        let actor = identity.try_local_actor().ok()??;
        let (count, captured) = {
            let prefix = actor.prefix().ok()?;
            let chasee = prefix.chasee.as_ref()?;
            (chasee.chasers.len() as u32, chasee.group_end_reason == 1)
        };
        let (position, hom_bail) = {
            let physical = actor.cached_physical_allocation().ok()?;
            let output = physical.owner().phys_out().lock().ok()?;
            (output.try_skeleton_output().ok()?.skeleton_root_matrix.columns()[3].xyz,
                output.try_audio_phys_out().ok()?.hom_bail_in_progress)
        };
        let terrain = self.inner.terrain.as_ref()?.terrain_for(&owner.world).ok()?;
        let region = terrain.layers.lock().ok()?.try_region_name(6, [position[0], position[2]]).ok()?;
        let world = owner.world.lock().ok()?;
        let living = world.grind_services.living_world().lock().ok()?;
        let chase = living.chase.as_ref()?;
        let roster = living.pedestrian_entities.as_ref()?;
        let mut flags = 0;
        if count > 0 {
            flags |= 0x40;
            if captured { flags |= 4; }
            for id in chase.chasers() {
                let Some(pedestrian) = (id.kind == 1).then(|| roster.get(id.id as u32)).flatten() else { continue };
                if pedestrian.security_guard.is_some() { flags |= 0x10; }
                if pedestrian.core.as_ref().is_some_and(|core| core.entity_key == skate2_formats::attrib::string_hash64("security_street01")) { flags |= 8; }
                // 8270ffb8/8270ffc8 exit on EITHER guard kind, not both.
                if flags & 0x18 != 0 { break; }
            }
        }
        if let Some(region) = region {
            if chase.warners().iter().any(|id| id.kind == 1 && roster.get(id.id as u32).is_some_and(|p| p.security_guard.is_some())) { flags |= 0x20; }
            if flags & 0x10 != 0 || region.contains("Area") { flags |= 0x80; }
            if chase.is_skater_being_jacked() { flags |= 2; }
        }
        Some(SecurityPresentation {
            // Native GlobalData has a TWO-bit count (827fb2f8), not saturation.
            chasers: count & 3, flags,
            tazer: chase.taze_is_on() && hom_bail,
            bodyguard: living.big_black_entity.is_some(),
        })
    }
}

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Visual, e.to_string())
}

/// `Sk8::Scene::SceneReplayTimeController` (constructor `0x82872600`).
#[derive(Clone, Copy, Debug)]
pub(super) struct ReplayClock {
    pub(super) start: u32,
    pub(super) end: u32,
    /// `mCurrentReplayFrame`.
    pub(super) current: u32,
    /// `mFrameUnaccounted`.
    pub(super) fraction: f32,
    /// SceneReplayTimeController keeps its last whole-frame direction.
    pub(super) rewinding: bool,
}

impl ReplayClock {
    /// The constructor: the start frame is current.
    pub(super) const fn new(start: u32, end: u32) -> Self {
        Self { start, end, current: start, fraction: 0.0, rewinding: false }
    }

    /// `Update` (`0x828727e8`): the scrub position is the start frame plus the scene's elapsed
    /// time at 60 frames a second, at most the end frame; the current frame moves to its whole
    /// frames through the recorder (`SkipForward` / `SkipBackward`) and keeps the rest.
    pub(super) fn update(&mut self, elapsed: f32, replay: &ReplaySystem) {
        #[expect(clippy::cast_precision_loss, reason = "(float)(__int64) frame numbers")]
        let (start, end, current) = (self.start as f32, self.end as f32, self.current as f32);
        let mut scrub = elapsed * 60.0 + start;
        if scrub > end {
            scrub = end;
        }
        let delta = scrub - current;
        let whole = delta.floor();
        self.fraction = delta - whole;
        #[expect(clippy::cast_possible_truncation, reason = "the floored delta's integer")]
        let steps = whole as i32;
        if steps > 0 {
            self.rewinding = false;
            self.current = replay.skip_forward(self.current, steps.unsigned_abs());
        } else if steps < 0 {
            self.rewinding = true;
            self.current = replay.skip_backward(self.current, steps.unsigned_abs());
        }
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    pub(super) fn replay_index(&self) -> &ReplaySystem {
        self.replay_editor.as_ref().and_then(|e|e.loaded_index.as_ref())
            .unwrap_or_else(||self.career.as_ref().map_or(&self.freeroam_replay, |career|career.replay()))
    }
    pub(super) fn recorded_frames(&self)->&BTreeMap<u32,Arc<ReplayFrame>> {
        self.replay_editor.as_ref().and_then(|e|e.loaded_frames.as_ref()).unwrap_or(&self.replay_frames)
    }
    /// `DataSysMan::SubmitFrame` (`0x8277bca0`) for the challenge replay recorder: in a career,
    /// the completed update's submission joins the recorder under its next frame number, and
    /// the frames the recorder no longer holds are let go.
    pub(super) fn record_replay_frame(&mut self) -> Result<(), SelectedLiveError> {
        if self.replay_editor.is_some() || self.main_menu.is_some() || self.main_menu_pending {
            return Ok(());
        }
        let Some(submission) = self.presentation_submission()? else { return Ok(()) };
        let audio = self.live_audio_frame();
        let local=self.inner.local_party_skater().map_err(error)?;
        let runtime=local.map(|s|&s.presentation_runtime).or(self.inner.presentation_runtime.as_ref()).ok_or_else(||error("replay local presentation absent"))?;
        let identity=runtime.palette().map_err(error)?.identity();
        let local_skater=submission.skaters.iter().position(|s|s.palette_identity()==identity).unwrap_or(0);
        let mut roster=self.inner.party_skater_visuals().map_err(error)?;
        roster.extend(self.inner.ai_skater_visuals().map_err(error)?);
        let players_recipe=submission.skaters.iter().map(|s|roster.iter().find(|p|p.palette.identity()==s.palette_identity()).is_none_or(|p|p.players_recipe)).collect();
        let mut world=self.recorded_world_visuals()?;
        let previous=self.replay_frames.last_key_value().map(|(_,frame)|&frame.world);
        for pedestrian in &mut world.pedestrians {
            let previous=previous.and_then(|w|w.pedestrians.iter().find(|p|p.entity_id==pedestrian.entity_id));
            pedestrian.freeze_replay_pose(previous).map_err(error)?;
        }
        let (frame,stored)=if let Some(career)=self.career.as_mut() {
            let (frame,stored,_)=career.store_replay_frame();(frame,stored)
        } else {
            self.freeroam_replay_frame=self.freeroam_replay_frame.wrapping_add(1);
            let frame=self.freeroam_replay_frame;
            (frame,self.freeroam_replay.store_frame(frame).0)
        };
        if stored {
            self.replay_frames.insert(frame, Arc::new(ReplayFrame { visual: submission, audio, world,local_skater,players_recipe }));
        }
        // Both the tracker and BTreeMap iterate in ascending frame order.
        // Merge them once instead of binary-searching the entire tracker for
        // every retained visual/audio frame on every simulation tick. This
        // keeps the same membership and ascending destruction order, including
        // pinned gaps, resets and EraseAllButLastNFrames (0x82994368).
        let index=self.career.as_ref().map_or(&self.freeroam_replay,|career|career.replay());
        let mut held = index.frames().peekable();
        self.replay_frames.retain(|frame, _| {
            while held.peek().is_some_and(|tracked| tracked < frame) {
                held.next();
            }
            held.peek().is_some_and(|tracked| tracked == frame)
        });
        Ok(())
    }

    /// The presentations a replay-controlled NIS restored from the recorded frames, which its
    /// frames draw; none outside one.
    #[must_use]
    pub fn replay_restorer(&self) -> Option<&PresentationRestorer> {
        self.replay_clock.is_some().then_some(&self.replay_restorer)
    }
}

/// A replay-controlled NIS's frame between the scene's time step and its camera
/// (`DataSysMan::RestoreFrame` `0x8277f040` under the scene's time controller): the clock scrubs
/// the recorder by the scene's time, the recorded frames it picked restore (the recorded game
/// camera too, the scene camera's replay camera), and the scene's camera anchors on the frame's
/// restored local skater. Outside one the clock goes and the presentations restore from the
/// game again.
/// Returns true on leaving replay, so the caller restores shared cloth before drawing.
pub(super) fn update_replay_view(
    career: &mut CareerSession,
    frames: &BTreeMap<u32, Arc<ReplayFrame>>,
    clock: &mut Option<ReplayClock>,
    restorer: &mut PresentationRestorer,
) -> Result<bool, SelectedLiveError> {
    let Some((start, end, elapsed)) = career.nis.replay_clock() else {
        let ended = clock.take().is_some();
        career.frozen_skaters.clear();
        career.nis.set_replay_camera(None);
        return Ok(ended);
    };
    if !clock.is_some_and(|running| running.start == start && running.end == end) {
        *clock = Some(ReplayClock::new(start, end));
    }
    let Some(clock) = clock.as_mut() else { return Ok(false) };
    clock.update(elapsed, career.replay());
    let (frame, fraction, forward) = (clock.current, clock.fraction, !clock.rewinding);
    let Some(next) = frames.get(&frame).cloned() else { return Ok(false) };
    let previous = career.replay().previous_frame(frame).and_then(|before| frames.get(&before)).cloned();
    let camera = career.nis.camera_position().unwrap_or([0.0; 3]);
    // Once a 1/60 s presentation update.
    let elapsed = crate::simulation_sim_rate::UPDATE_TIME;
    restorer.restore_recorded(previous.as_ref().map(|f| &f.visual), &next.visual, frame, fraction, camera, forward, elapsed).map_err(error)?;
    career.nis.set_replay_camera(restorer.replay_matrix());
    // The recording can cross Party Play turns: the frame's own local player, the skater
    // `SkaterPresentation::GetLocalIndex` (0x827d0090) finds flagged local in the restored data.
    let local = next.visual.skaters.get(next.local_skater).map(|skater| skater.palette_identity());
    if let Some(skater) = local.and_then(|identity| restorer.skater(&identity)) {
        career.nis.set_camera_anchor(Some(skater.root));
        career.nis.set_camera_local_hips(Some(skater.hips_position));
        career.nis.set_camera_local_hips_forward(Some(skater.hips_forward));
        career.nis.set_local_bounds(Some(skater.bounds));
        // InstanceFrozenSkater (0x82874f90): a copy without the recipe's renderable is
        // destroyed.
        if career.nis.take_frozen_skater()
            && let Some(materials) = skater.materials
        {
            let materials = materials
                .lock()
                .map_err(|_| error("skater materials lock poisoned"))?
                .try_frozen_copy()
                .map_err(error)?;
            career.frozen_skaters.push(super::career::FrozenSkater {
                palette: skater.palette.clone(),
                hips: skater.hips_position,
                bounds: skater.bounds,
                root: skater.root,
                materials,
            });
            if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
                eprintln!("career: frozen skater {} at replay frame {frame}", career.frozen_skaters.len());
            }
        }
    }
    Ok(false)
}

/// On a return to game time (Scene::Purge 0x8286a908's non-held branch or
/// EnterRealTime 0x827f1130), RestoreFrame (0x8277f040) restores the game
/// presentation even when the challenge/StartHub still pauses SIM.
/// A new character NIS owns its borrowed entities' skin jobs instead.
pub(super) fn restore_game_cloth<T, C>(
    inner: &mut super::SelectedSession<T, C>,
    career: Option<&CareerSession>,
) -> Result<(), SelectedLiveError> {
    if !career.is_some_and(CareerSession::first_player_borrowed)
        && let Some(runtime) = inner.presentation_runtime.as_mut()
    {
        runtime.restore_game_cloth().map_err(error)?;
    }
    for skater in inner.party.iter_mut().chain(inner.ai_skaters.iter_mut()) {
        if !skater.presentation_runtime.borrowed() {
            skater.presentation_runtime.restore_game_cloth().map_err(error)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_scrubs_whole_frames_and_keeps_the_rest() {
        let mut replay = ReplaySystem::default();
        for frame in 0..100 {
            replay.store_frame(frame);
        }
        let mut clock = ReplayClock::new(10, 40);
        clock.update(0.25, &replay);
        assert_eq!(clock.current, 25);
        assert!(clock.fraction.abs() < 1.0e-4);
        clock.update(0.2583, &replay);
        assert_eq!(clock.current, 25);
        assert!((clock.fraction - 0.498).abs() < 1.0e-3);
        clock.update(5.0, &replay);
        assert_eq!(clock.current, 40, "held at the end frame");
        clock.update(0.0, &replay);
        assert_eq!(clock.current, 10, "back to the start");
    }
}

