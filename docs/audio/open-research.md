# Open audio research (implementation order)

Each item must be closed from IDA (debug xex, raw PPC where the decompile is ambiguous), the PDB and
the shipped data before its code lands. Each closure gets an evidence note
(`docs/evidence/audio-*.md`).

## Formats (phase A0/A1)
* **R-F1 Csis `.csi` (MOIR) layout.** Descriptor tables for classes, functions and globals; CRC
  fields; parameter descriptors and clamp ranges. Code: `Csis::System::Subscribe` 0x829b20c0;
  types `CsisDef::*`.
* **R-F2 `*.ems` emitter placements.** Record layout and consumers: `Sk8EmitterSystem::LoadGameFiles`,
  `EmitterSystem` (octree, `tEmitterVolume`).
* **R-F3 SNS seek tables and `.sek`.** `SeekTableParser::ParseHeader0/1`, `ParseChunkSection0/1`;
  the loose `Whls_spins_*.sek`.
* **R-F5 EALayer3 v1 decoder (software on 360, portable 1:1).** `rw::audio::core::EaLayer31Dec`,
  `EaLayer3DecBase`, `EALayer3Core`, `Layer3Dec`, `CMpegLayer3Base`, `HELPER_CEALayer3*`: bitstream
  layout, granule handling, synthesis, output format. Used by all main-cast speech and the French
  announcer.
* **R-F4 XMA2 frame decoding.** Choose and implement the decoder (README D1). Validate s16 output
  against an oracle on a sample from every archive. Confirm `DecoderTable` (`w & 3`) values across
  the data.

## Engine (A2/A3)
* **R-E1 Channel order.** `ReOrderRwAudioCoreToWave` and `Dac::sChannels`: the RWAC internal channel
  order versus WAVE order.
* **R-E2 DSP plugin maths**, bit-accurate where practical: Pan2D1 (pan law, front/rear angle
  defaults, `Pan2DKernel`), LowPassIir2 / HighPassIir2 / PeakingIir2 / HighShelfIir2 (coefficient
  formulas, parameter smoothing), Resample (interpolation method, pitch range), Gain / GainFader
  (ramps), Send / SubMix (routing, summing), Rechannel, Delay, VuMeter, Limiter, TimeStretch (music),
  ReverbModel1 / ReverbIR1 (GlobalEffects reverb).
* **R-E3 SndPlayer1.** Request queue, loop handling (`HandleLoopStart`, `HandleSampleEnd`), stream
  chunking (`StreamNextChunk`), declick, start time, `ModifyStartTime`.
* **R-E4 StreamPool / filesystem streaming.** `StreamPool::CreateInstance/AcquireStream/ReleaseStream`
  and `RwacTimerClient`: buffer sizes, prefetch and latency.
* **R-E5 Voice priority and stealing.** `Voice::SetPriority`, `System::UpdateExpellingVoices`,
  `ExpelReason`, the CPU load balancer: what happens when the voice limit is reached.
* **R-A2 RAM versus stream sample selection** in `SndPlayer1` (pRamSample + stream path/offset).
* **R-A3 Raw-PPC confirmation of ambiguous AEMS opcodes.** ENVELOPE end of the last segment, TABLE
  8-bit signedness, OSCILLATOR sine scaling (`iSNDsin`), RAMP scale/4096.
* **R-A4 Bank removal.** `BeginRemoveModuleBank` / `IsModuleBankRemoved` / `RemoveModuleBankHandler`.
* **R-A5 `iSNDrandom` / `iSNDsin`.** Generator algorithm and seed (AEMS randomness must match),
  and the sine table.
* **R-A6 `SkateSamplePlayer::GetOutputs` / `Pause` / `Unpause` / `Release`** and
  `RegisterAemsPlugIns`.

## Game layer (A4–A6)
* **R-N1 Nicotine.** `MixMapSK8.mxb` layout; `NFSMixMap::InitMixMap / PreProcessMixMap /
  ProcessMixMap`; master and sub channels; event mix controls (ADSR/ASR/AR); 3D mix controls; curve
  shapes; snapshots; `DMixIO` inputs and outputs; `SFXObjBase::Get*FromMixer`.
* **R-G1 AudioAssetManager.** Asset types, slots, swap banks, async loading and unloading.
* **R-G2 FE / NIS / movie / challenge entry points** listed in game-layer §7.
* **R-G3 Every state manager.** CreateStateMan, EnterGamePlay, UpdateParams, ProcessUpdate,
  GetFreeState, attach rules and ranges (Collision, Whoosh, Environment, TrafficCar, Pedestrian,
  Emitter, Crowd, DynamicObject, Speaker, ObjectInstance, NISCharacter).
* **R-G4 Message handlers.** AddMessageHandlers / AddDelayedMessageHandlers and each handler body.
* **R-P1 AudioPresentation producers.** `FillSkaterData`, `DoSimSubmit`, `BlendSkaterData`,
  `ConvertToImpactLevelFromForce`, Vehicle/Pedestrian/Dynamic/SpacialInstance fills. Map every field
  to the port's existing PhysOut / Actor / LivingWorld data.
* **R-V1 Vault `aud_*` classes.** `aud_general`, `aud_material`, `aud_skateboard`, `aud_contacts`,
  `aud_collisions`, `aud_rails`, `aud_cracks`, `aud_splicer`, `aud_whooshby`, `aud_flange`,
  `aud_reverb`, `aud_cluster`, `aud_wp_ambiences`, `aud_wp_emitters`, `aud_emitter`, `aud_crowds`,
  `aud_dynamicobject*`, `aud_traffic_engine`, `aud_pedestrian`, `aud_speech*`, `aud_music`,
  `aud_playlists`, `aud_volumemapping`, `aud_fehud`. Which `.vlt` holds them, and their layouts.

## SFX objects (A7/A8)
* **R-X1 Skater.** SFXObj_SkateBoard, Wheels, Contacts, Rail, Cracks, Tricks, Clothing, SenseOfSpeed,
  OffBoard, HandGrabs, Treatments, PlayerSpeech; SFXCTL_PlayerPhysics, 3DObjPos/3DPlayerPos/
  3DBoardPos (MicrophoneMan listener, `SPEED_OF_SOUND`, `POSMIXTYPE`, azimuth references);
  FootStepUtils; SurfaceMapping (`mAudioSurfaceMapping`, `SurfacePatternToKey`); GetSplicerInfo,
  CalculateCollisionVolume, GetCollisionLevel; Splicer (SpliceManager); GrainPlayer::AudBoard.
* **R-X2 World.** Ambience (+ Main_Ambience_Crossfade), Emitter, Reverb (+ GlobalEffects), Crowd,
  Speaker / MultiSpeaker, Dynamic / Moving (DMO), Collision, Whoosh, Traffic*, Pedestrian*,
  ObjectInstance, NISCharacter.
* **R-X3 Main state.** Master, Music, Announcer, CameraMan, Reverb, NIS, Pause (pause FX), Speech,
  Bloom, VU, Challenge, HOM (Hall of Meat bank), Menu, Jitter.

## Music, speech, UI (A9–A11)
* **R-M1 Pathfinder.** `.mpf` (PFDx v5) and `.mus` formats; MusicCore tasks and streams; the
  MusicSystem state machine (FE / world / game / chase / race / replay / challenge, iPod playlist,
  interactive levels, environment zones, DSP voice, game-speed effects).
* **R-SP1 Speech.** VOX bank format (spch library), `.sth` / `hdr` files, SpeechInstance and
  SpeechSystem streaming, the Sk8SpeechSystem rules, Contexts XML (`ContextBuilderXMLCallback`), CSIS
  trigger events.
* **R-S1 Splicer `.bnk` (SPLC)** and splice selection (SPLICE_TYPE, levels, randomisation).
* **R-G1b Grain format** and GrainPlayer::AudBoard playback.
* **R-UI1 FE sound IDs.** Map the APT `FE_SOUND_*` callbacks and `GlobalFEPlaySound(u64)` IDs to
  menu Splicer bank entries; `PlayMenuSoundImp`; menu flange voice.

<a id="census"></a>
## Codec census (all EA-XMA)

| Archive | Count | Channels | Rates | Loop |
| --- | --- | --- | --- | --- |
| ambienceresident (.snr → ambience .sns) | 16 | 5 | 48000 | yes |
| crowds | 957 | 1 (950), 2 (7) | 44100 / 48000 | no |
| dynamicobject | 1511 | 1 | 24000 / 36000 / 48000 | no |
| emitters | 714 | 1 | 36000 / 44100 / 48000 | some |
| emitters2 | 37 | 1 | 44100 / 48000 | some |
| loadscreen | 15 | 6 | 48000 | no |
| moments | 20 | 1, 2, 4 | 48000 | some |
| pedestrian | 419 | 1 | 36000 / 48000 | some |
| rolling | 117 | 1 | 48000 | some |
| skateboard | 1009 | 1 (1008), 4 (1) | 22050 / 48000 | some |
| vehicle | 121 | 1 | 44100 / 48000 | some |
