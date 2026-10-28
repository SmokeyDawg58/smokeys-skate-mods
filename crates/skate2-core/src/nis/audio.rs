//! The `Sk8::Audio::IAudioInterface` calls the scene player (`ScenePlayer::AudioQueueForStart`,
//! the ICE audio user channels, `Scene::Init`, `Scene::Purge`, the audio clock of
//! `SceneManagerImpl::UpdateFrameBegin`) and the challenge audio manager
//! (`ChallengeAudioManager::PlaySpeech` 0x82da1b98) make of the audio system
//! (`SingletonHolder<AudioSystem>::Get`), which the host implements over its audio system.

/// What the audio system tells the game about mission speech.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechMessage {
    /// `cMsgSpeechStarted` to the challenge system.
    Started { character: i32, id: i32 },
    /// `cMsgSpeechEnded` to the challenge system.
    Ended { character: i32, id: i32 },
    /// `cMsgCallSpeechStarted` to the front end.
    CallStarted { character: i32, id: i32 },
    /// `cMsgCallSpeechEnded` to the front end.
    CallEnded { character: i32, id: i32 },
}

/// The audio system behind `IAudioInterface` (`AudioSystem` vtable 0x82022ac0).
pub trait AudioInterface: Send + Sync {
    /// `AudioCore::GetSystemTime` 0x82a7dae0: the audio clock, in seconds.
    fn system_time(&self) -> f64;
    /// `QueueNISPost` 0x824f1158 (+0x58).
    fn queue_nis_post(&self, id: i32);
    /// `PlayNISPost` 0x824f11c0 (+0x5C).
    fn play_nis_post(&self);
    /// `IsPostQueued` 0x824f1228 (+0x60).
    fn is_post_queued(&self) -> bool;
    /// `QueueNISSpeech` 0x824f1080 (+0x64).
    fn queue_nis_speech(&self, character: i32, id: i32);
    /// `PlayNISSpeech` 0x8250c6a0 (+0x68).
    fn play_nis_speech(&self, character: i32, id: i32);
    /// `IsSpeechQueued` 0x824f10e8 (+0x6C).
    fn is_speech_queued(&self, character: i32, id: i32) -> bool;
    /// `QueueNISMusic` (+0x50) and `PlayNISMusic` (+0x54): the empty stub (`blr`, 0x825187c0) in
    /// this build.
    fn queue_nis_music(&self, _id: i32) {}
    fn play_nis_music(&self, _id: i32) {}
    /// `NotifyNISPlaybackStart` 0x824fe540 (+0x74).
    fn notify_nis_playback_start(&self, scene_type: i32, special_flags: i32, is_replay_nis: bool, scene_length: f32);
    /// `NotifyNISPlaybackFinish` 0x824fe558 (+0x78).
    fn notify_nis_playback_finish(&self);
    /// `PlayMissionSpeech` 0x82504ed0 (+0x3C).
    fn play_mission_speech(&self, character: i32, id: i32, param: i32);
    /// cMsgLeaderBoardOverallScreen -> PlayerOverallResultHandler825007b8.
    fn leaderboard_overall(&self, _character: i32) {}
    /// `cMsgPlayCameraManCsisEvent` with unset parameters on the audio broadcaster
    /// (`AudioSystem::ChallengeFailedHandler` 0x82505740 sends it), which
    /// `CameraManCsisEventHandler` 0x824ff7a0 plays.
    fn play_camera_man_csis_event(&self, function: &'static str);
    /// `TriggerMissionMusic` 0x82504f28 (+0x34, `ChallengeAudioManager::PlayMusic`) of the
    /// `eSk8MissionMusicTrigger` for the challenge, with what `TriggerMissionMusicImp` 0x824fee80
    /// reads of it: its `ChallengeType` and its `AudioRaceTuning` collection.
    fn trigger_mission_music(&self, trigger: i32, challenge: u64, challenge_type: i32, race_tuning: u64);
    /// `PlayMissionSFX` 0x82504e70 (+0x38, `ChallengeAudioManager::PlayMissionSFX`) of the
    /// `eSk8MissionSFX` for the challenge, with its `ChallengeType` and `AudioRaceTuning`.
    fn play_mission_sfx(&self, _sfx: i32, _challenge: u64, _challenge_type: i32, _race_tuning: u64) {}
    /// `CacheCrowdPodInfos` 0x824fd6c0 at a countdown (`TriggerMissionMusic` 0x82504f28): the
    /// spectator groups found around the local skater, as centroid and spectators.
    fn cache_crowd_pod_infos(&self, _pods: &[([f32; 4], i32)]) {}
    /// `RestartChallenge` 0x824ff320 (+0x84, `ChallengeAudioManager::RestartChallenge` 0x82da1c48).
    fn restart_challenge(&self, retry: bool);
    /// `QuitChallenge` 0x824ff478, the audio system's `cMsgAbandonChallenge` handler on the
    /// challenge broadcaster.
    fn quit_challenge(&self);
    /// The messages the audio system sent since the last call, in order.
    fn take_speech_messages(&self) -> Vec<SpeechMessage>;
}
