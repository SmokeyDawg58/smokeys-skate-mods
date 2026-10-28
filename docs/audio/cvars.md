# Audio cvars

Extracted from every cvar static initialiser whose category contains Audio, Sound, Music, Speech or
SFX, or whose variable is in the `Sk8::Audio` namespace: **146 cvars**. Source: the
`cVarBoolRef` / `cVarIntRef` / `cVarFloatRef` / `cVarEnumRef` constructor arguments in IDA
(r4 category, r5 name, r6 description, r7.. default/min/max; floats in f1..f3).

`bool`: default in r7. `int`: default/min/max = r7/r8/r9. `float`: default/min/max = f1/f2/f3.
`enum`: default index, then the comma-separated value list.
A blank float default means f1 was not loaded by `lfs` in the initialiser (zero or computed). Re-check
those before relying on them: kReverbTime, kSmallJumpDV, kDebugImpactForce.

None of these exist in the port's cvar runtime yet. They must be added to the existing
`input/tuning/input_cvar_runtime.rs` registry, keeping their original categories, labels and defaults.

| Category | Name | Type | Description | Default | Min | Max |
| --- | --- | --- | --- | --- | --- | --- |
| Audio | `kCompensateForSlowFrameRate` | bool | Compensate For Slow Frame Rate | 1 |  |  |
| Audio | `kForceSingleAudioUpdate` | bool | Force Single Audio Update | 0 |  |  |
| Audio | `kbEnableFESoundOutput` | bool | Print FE sound ID to screen | 0 |  |  |
| Audio_AzimRate | `kPrintOutAzimRate` | bool | Print Out Azim Rate Change | 0 |  |  |
| Audio_AzimRate | `kPrintOutLocal` | bool | Local Skater | 1 |  |  |
| Audio_Collision | `kEnableLandCollisionOutput` | bool | Enable Land Collision Output | 1 |  |  |
| Audio_Collision | `kSlideSurface` | int | Set Slide Surface | -1 | -1 | 6 |
| Audio_Contacts | `kEnableSweetener` | bool | Enable L3 | 1 |  |  |
| Audio_Crowds | `kEnableCrowdOverlays` | bool | Enable Crowd Overlays | 0 |  |  |
| Audio_Crowds | `kIgnoreNicotine` | bool | Ignore Nicotine Levels | 0 |  |  |
| Audio_Debug | `kAllowLoadScreen` | bool | Allow Load Screen | 1 |  |  |
| Audio_Debug | `kAmbienceDebugOut` | bool | Ambience Debug Output | 0 |  |  |
| Audio_Debug | `kDMOCollisionOutput` | bool | DMO Collision Output | 0 |  |  |
| Audio_Debug | `kEnableReverb` | bool | Enable Reverb | 1 |  |  |
| Audio_Debug | `kEnableTreatments` | bool | Enable Treaments | 1 |  |  |
| Audio_Debug | `kFEHudDebugOut` | bool | FE Hud Debug Output | 0 |  |  |
| Audio_Debug | `kFootCatchDebugOut` | bool | Foot Catch Debug Output | 0 |  |  |
| Audio_Debug | `kNoRailWhenUntagged` | bool | No Rail Sounds When Not Tagged | 0 |  |  |
| Audio_Debug | `kOllieLandDebugOut` | bool | Ollie Land Debug Output | 0 |  |  |
| Audio_Debug | `kPrintMusicVU` | bool | Print Music VU | 0 |  |  |
| Audio_Debug | `kPrintSpeechVU` | bool | Print Speech VU | 0 |  |  |
| Audio_Debug | `kRenderAudioSpheres` | bool | Render Audio Spheres | 0 |  |  |
| Audio_Debug | `kRenderAudioSpheresAEMS` | bool | Render Audio Spheres - AEMS | 1 |  |  |
| Audio_Debug | `kRenderAudioSpheresCrowd` | bool | Render Audio Spheres - CROWD | 1 |  |  |
| Audio_Debug | `kRenderAudioSpheresMusic` | bool | Render Audio Spheres - MUSIC | 1 |  |  |
| Audio_Debug | `kRenderAudioSpheresReverb` | bool | Render Audio Spheres - REVERB | 1 |  |  |
| Audio_Debug | `kRenderAudioSpheresSpeaker` | bool | Render Audio Spheres - SPEAKER | 1 |  |  |
| Audio_Debug | `kRenderInnerRadius` | bool | Render Audio Spheres - INNER RADIUS | 1 |  |  |
| Audio_Debug | `kRenderMidRadius` | bool | Render Audio Spheres - MID RADIUS | 0 |  |  |
| Audio_Debug | `kReverbDebugOut` | bool | Reberb Debug Output | 0 |  |  |
| Audio_Debug | `kReverbTime` | float | Reverb Time |  | 0 | 5 |
| Audio_Debug | `kRollingSurfaceDebugOut` | bool | Rolling Surface Debug Output | 0 |  |  |
| Audio_Debug | `kShowBodySlideSurface` | bool | Show Body Slide Surface Mapping | 0 |  |  |
| Audio_Debug | `kShowFootPushSurface` | bool | Show Foot Push Surface Mapping | 0 |  |  |
| Audio_Debug | `kShowFootStopSurface` | bool | Show Foot Stop Surface Mapping | 0 |  |  |
| Audio_Debug | `kShowGrindSurface` | bool | Show Grind Surface Mapping | 0 |  |  |
| Audio_Debug | `kShowHardnessSurface` | bool | Show Hardness Mapping | 0 |  |  |
| Audio_Debug | `kShowOllieAndLandSurface` | bool | Show Ollie and Land Surface Mapping | 0 |  |  |
| Audio_Debug | `kShowPowerSlideSurface` | bool | Show Power Slide Surface Mapping | 0 |  |  |
| Audio_Debug | `kSurfacePatternDebugOut` | bool | Surface Pattern Debug Output | 0 |  |  |
| Audio_Debug_Collisions | `kCollisionDebugOut` | bool | Collision Debug Output | 0 |  |  |
| Audio_Debug_Collisions | `kCurrentCollisionPrint` | bool | Print Current Collision Avergage | 0 |  |  |
| Audio_Debug_Collisions | `kDebugAudioMaterial` | enum | Debug Audio Material | 132 | eAudioMaterialTypes (132 names: Asphalt_Smooth … Pottery_2_Rattle) |  |
| Audio_Debug_Collisions | `kDebugImpactForce` | float | Debug Impact Force - Value |  | 0 | 5 |
| Audio_Debug_Collisions | `kDebugImpactorHardness` | enum | Debug Impactor Hardness - Value | 0 | Soft, Medium, Hard |  |
| Audio_Debug_Collisions | `kPlayImpactedCollision` | bool | Play Impacted Collision | 1 |  |  |
| Audio_Debug_Collisions | `kPlayImpactorCollision` | bool | Play Impactor Collision | 1 |  |  |
| Audio_Debug_Collisions | `kUseDebugImpactForce` | bool | Debug Impact Force - Use | 0 |  |  |
| Audio_Debug_Collisions | `kUseDebugImpactorHardness` | bool | Debug Impactor Hardness - Use | 0 |  |  |
| Audio_HOM | `kNoHOMInReplay` | bool | No HOM in Replay | 0 |  |  |
| Audio_HOM | `kPrioritizeBaseSounds` | bool | Prioritize Base Sounds | 0 |  |  |
| Audio_HOM | `kPrioritizeHOMSounds` | bool | Prioritize HOM Sounds | 1 |  |  |
| Audio_Jitter | `kDebugPrintJitter` | bool | Print Jitter | 0 |  |  |
| Audio_Jitter | `kJitterToSpew` | int | Jitter To Spew | -1 | -1 | 10 |
| Audio_JumpTuning | `kLargeJumpDV` | float | Max Jump Delta | 2.65 | 0 | 10 |
| Audio_JumpTuning | `kMinAirFrames` | int | Min Air Frames | 5 | 0 | 60 |
| Audio_JumpTuning | `kSmallJumpDV` | float | Min Jump Delta |  | 0 | 2.65 |
| Audio_MoveableObject | `kSpeedDebugPrint` | bool | Speed Debug Print | 0 |  |  |
| Audio_Music | `kAddDspNearOuter` | bool | Add DSP From Mid To Outer | 1 |  |  |
| Audio_Music | `kApplyFakeQuad` | bool | Apply Fake Quad | 1 |  |  |
| Audio_Music | `kEnableEnvironment` | bool | Enable Ambient Music | 1 |  |  |
| Audio_Music | `kEnableMusic` | bool | Enable Music | 1 |  |  |
| Audio_Music | `kEnableOnscreenPrints` | bool | Enable On-Screen Messages | 0 |  |  |
| Audio_Music | `kGainMultiply` | float | Multiply Nicotine Gain | 1 | 0 | 4 |
| Audio_Music | `kResetMusic` | bool | Reset Music State | 0 |  |  |
| Audio_Music | `kRockStarType` | enum | Rockstar Type | 0 | Left Right and Sub, Left and Right, Left Right and Surrounds |  |
| Audio_Music | `kStraightToDac` | bool | Straight To DAC | 0 |  |  |
| Audio_Music | `kUsePauseFX` | enum | Pause Effect | 0 | Pitch down, Pitch up |  |
| Audio_Music_Channels | `kTestChannelConfig` | bool | Test Channels | 0 |  |  |
| Audio_Music_Channels | `kTestChannelDestination` | enum | Test Destination Channel | 0 | FrontLeft, FrontCentre, FrontRight, BackLeft, BackRight, LFE |  |
| Audio_Music_Channels | `kTestChannelSource` | enum | Test Source Channel | 0 | FrontLeft, FrontCentre, FrontRight, BackLeft, BackRight, LFE |  |
| Audio_Music_Chase | `kTestChaseEvent` | enum | Test Chase Events | 0 | CHASE_INTRO, CHASE_FIRSTWARNING, CHASE_MAIN, CHASE_CAUGHT, CHASE_ESCAPE, CHASE_TRANS, CHASE_RETURN |  |
| Audio_Music_Chase | `kTestChaseTrigger` | bool | Trigger Selected Event | 0 |  |  |
| Audio_Music_DSPfx | `kGameSpeedEffect` | enum | Game speed effect | 1 | Low pass, Pitch down, Slow down, Slow down + low pass |  |
| Audio_Music_DSPfx | `kGameSpeedLowPassScalar` | int | Game speed low-pass multiplier | 100 | 0 | 200 |
| Audio_Music_DSPfx | `kGameSpeedPitchMultScalar` | int | Game speed pitch-mult multiplier | 100 | 0 | 200 |
| Audio_Music_DSPfx | `kGameSpeedTimeStretchScalar` | int | Game speed time-stretch multiplier | 100 | 0 | 200 |
| Audio_Music_DSPfx | `kTestGameSpeedEffect` | int | Test game speed | 100 | 0 | 200 |
| Audio_Music_DSPfx | `kTestHighPass` | int | Test high pass | 100 | 100 | 100 |
| Audio_Music_DSPfx | `kTestLowPass` | int | Test low pass | 100 | 0 | 100 |
| Audio_Music_DSPfx | `kTestMaxPitchChange` | int | Test max pitch change | 1 | 0 | 100 |
| Audio_Music_DSPfx | `kTestMaxStretchChange` | int | Test max stretch change | 1 | 0 | 100 |
| Audio_Music_DSPfx | `kTestMaxVolChange` | int | Test max volume change | 1 | 0 | 100 |
| Audio_Music_DSPfx | `kTestPitchMult` | int | Test pitchmult | 100 | 0 | 200 |
| Audio_Music_DSPfx | `kTestTimeStretch` | int | Test timestretch | 100 | 0 | 200 |
| Audio_Music_Environment | `kApplyDSP` | bool | Apply DSP | 1 |  |  |
| Audio_Music_Environment | `kDSPChannels` | enum | DSP Channels | 1 | Quad, 5.1 |  |
| Audio_Music_Environment | `kGlobalReverb` | bool | Global Reverb | 1 |  |  |
| Audio_Music_Environment | `kSpewPositionalDebugs` | bool | Positional debug spew | 0 |  |  |
| Audio_Music_Interactive | `kInteractiveDebugPrint` | bool | Debug Print | 0 |  |  |
| Audio_Music_Pathfinder | `kDebugController` | bool | Debug prints: controller | 0 |  |  |
| Audio_Music_Pathfinder | `kDebugErrors` | bool | Debug prints: errors | 1 |  |  |
| Audio_Music_Pathfinder | `kDebugEvents` | bool | Debug prints: events | 1 |  |  |
| Audio_Music_Pathfinder | `kDebugFXBus` | bool | Debug prints: fxbus | 0 |  |  |
| Audio_Music_Pathfinder | `kDebugFades` | bool | Debug prints: fades | 0 |  |  |
| Audio_Music_Pathfinder | `kDebugFiles` | bool | Debug prints: files | 1 |  |  |
| Audio_Music_Pathfinder | `kDebugOverlays` | bool | Debug prints: overlays | 0 |  |  |
| Audio_Music_Pathfinder | `kDebugPositional` | bool | Debug prints: positioning | 0 |  |  |
| Audio_Music_Pathfinder | `kDebugRouting` | bool | Debug prints: routing | 0 |  |  |
| Audio_Music_Pathfinder | `kDebugStatus` | bool | Debug prints: status | 0 |  |  |
| Audio_Music_Pathfinder | `kDebugStreams` | bool | Debug prints: streams | 1 |  |  |
| Audio_Music_Pathfinder | `kDebugThreads` | bool | Debug prints: threads | 0 |  |  |
| Audio_Music_Pathfinder | `kDebugTiming` | bool | Debug prints: timing | 0 |  |  |
| Audio_Music_Pathfinder | `kDebugVolumes` | bool | Debug prints: volumes | 0 |  |  |
| Audio_Music_Race | `kTestRaceEvent` | enum | Test Race Events | 0 | race_countdown, race_start, race_halfway, race_threequarters, race_straightaway, race_win, race_lose, race_shortjump |  |
| Audio_Music_Race | `kTestRaceTrigger` | bool | Trigger Selected Event | 0 |  |  |
| Audio_Music_Replay | `kTestReplayEvent` | enum | Test Replay Events | 0 | REPLAY_START, REPLAY_STOP, |  |
| Audio_Music_Replay | `kTestReplayTrigger` | bool | Trigger Selected Event | 0 |  |  |
| Audio_Music_iPod | `kPlayIPodSong` | int | Play IPod Song | -1 | -1 | 96 |
| Audio_PCFoley | `kClothingType` | int | Force Clothing Type | 1 | -1 | 4 |
| Audio_PCFoley | `kEnablePCFootSteps` | bool | Enable Foot Steps | 1 |  |  |
| Audio_PCFoley | `kFootStepOverride` | bool | FootStep - Override | 0 |  |  |
| Audio_PCFoley | `kFootStepOverrideType` | enum | FootStep - Surface | 0 | eAudioMaterialTypes (132 names: Asphalt_Smooth … Pottery_2_Rattle) |  |
| Audio_PCFoley | `kFootStepShoeType` | int | Force Shoe Type | 0 | 0 | 5 |
| Audio_PCFoley | `kUpDownTolerance` | float | Up-Down Tolerance | 0.07 | 0 | 5 |
| Audio_Pedestrian | `kEnableFootSteps` | bool | Enable Foot Steps | 1 |  |  |
| Audio_Pedestrian | `kEnableSpeedOutput` | bool | Enable speed output | 0 |  |  |
| Audio_Pedestrian | `kForceSpeed` | int | Force Ped Speed | -1 | -1 | 1000 |
| Audio_Pedestrian | `kForceTerrain` | int | Force Foot Terrain | 0 | 0 | 5 |
| Audio_Pedestrian | `kNumPedestrians` | int | Number of Pedestrains | -1 | -1 | 15 |
| Audio_Pedestrian | `kPedFootStepOverride` | bool | FootStep - Force Surface | 0 |  |  |
| Audio_Pedestrian | `kPedFootStepOverrideType` | enum | FootStep - Surface Type | 0 | eAudioMaterialTypes (132 names: Asphalt_Smooth … Pottery_2_Rattle) |  |
| Audio_Pedestrian | `kShoeType` | int | Force Shoe Type | 0 | 0 | 5 |
| Audio_SenseOfSpeed | `kPlayDistortion` | bool | Play Distortion | 1 |  |  |
| Audio_SenseOfSpeed | `kPlayRattle` | bool | Play Rattle | 1 |  |  |
| Audio_SenseOfSpeed | `kPlayRocket` | bool | Play Rocket | 1 |  |  |
| Audio_SenseOfSpeed | `kPlayTone` | bool | Play Tone | 0 |  |  |
| Audio_SenseOfSpeed | `kPlayVolumeModulation` | bool | Play Volume Modulation | 1 |  |  |
| Audio_SenseOfSpeed | `kPlayWind` | bool | Play Wind | 1 |  |  |
| Audio_SenseOfSpeed | `kTurnOffDryRolling` | bool | Turn Off Rolling To Test Distortion | 0 |  |  |
| Audio_Speaker | `kEnableMS` | bool | Enable Multispeaker Mode | 0 |  |  |
| Audio_Speech | `kCollisionFrameDecay` | int | Frames before wiping out | 5 | 0 | 120 |
| Audio_Speech | `kEnableANSpeech` | bool | Announcer Speech Enabled | 1 |  |  |
| Audio_Speech | `kEnableCMSpeech` | bool | CameraMan Speech Enabled | 1 |  |  |
| Audio_Speech | `kEnableLWSpeech` | bool | Living World Speech Enabled | 1 |  |  |
| Audio_Speech | `kEnableMCSpeech` | bool | Main Cast Speech Enabled | 1 |  |  |
| Audio_Speech | `kEnableSpeechOutput` | bool | Enable Speech Event Output | 1 |  |  |
| Audio_Speech | `kForceSinglePlayerMode` | bool | Force Single Player Mode | 0 |  |  |
| Audio_Speech | `kPlayNearMissOnWhoosh` | bool | Play Near Miss on Whoosh | 1 |  |  |
| Audio_Speech | `kSetPhotogReactDistance` | float | Set Photog React Max Distance | 10 | 0 | 50 |
| Audio_Speech | `kSetPhotogTrickReactDistance` | float | Set Photog Trick React Max Distance | 10 | 0 | 50 |
| Audio_Speech | `kSkaterGruntImpactLevel` | int | Skater Grunt Min Impact Level | 0 | 0 | 3 |
| Audio_Traffic | `kAddDeltaTime` | int | AddDeltaTime[0-1] | 0 | 0 | 1 |
| Audio_Traffic | `kEngineAudio` | int | EngineAudio[0-1] | 1 | 0 | 1 |
| Audio_Traffic | `kHornAudio` | int | HornAudio [0-1] | 1 | 0 | 1 |
| Audio_Traffic | `kSkidAudio` | int | SkidAudio [0-1] | 1 | 0 | 1 |

## Audio material enum (`kDebugAudioMaterial` value list, 132 entries)

Verified identical (names and values 0..132) to the PDB enum `Sk8::Audio::eAudioMaterialTypes`
(`eAudioMaterialType_Max` = 132). It drives surface-to-sound mapping (`aud_material`).

| | | | | | |
| --- | --- | --- | --- | --- | --- |
| 0: Asphalt_Smooth | 1: Asphalt_Rough | 2: Concrete_Polished | 3: Concrete_Rough | 4: Concrete_Aggregate | 5: Wood_Ramp |
| 6: Plywood | 7: Dirt | 8: Metal | 9: Grass | 10: Metal_Solid_Round_1 | 11: Metal_Solid_Round_1_Up |
| 12: Metal_Solid_Round_2 | 13: Metal_Solid_Square_1 | 14: Metal_Solid_Square_2 | 15: Metal_Hollow_Round_1 | 16: Metal_Hollow_Round_1_Dead | 17: Metal_Hollow_Round_1_Dn |
| 18: Metal_Hollow_Round_2 | 19: Metal_Hollow_Round_2_Dead | 20: Metal_Hollow_Round_2_Dn | 21: Metal_Hollow_Round_3 | 22: Metal_Hollow_Round_4 | 23: Metal_Hollow_Square_1 |
| 24: Metal_Hollow_Square_2 | 25: Metal_Hollow_Square_3 | 26: Metal_Hollow_Square_3_Dead | 27: Metal_Hollow_Square_4 | 28: Metal_Hollow_1 | 29: Metal_Hollow_2 |
| 30: Metal_Sheet | 31: Metal_Complex_1 | 32: Metal_Complex_2 | 33: Metal_Complex_3 | 34: Metal_Complex_4 | 35: Metal_Complex_5 |
| 36: Metal_Complex_6 | 37: Metal_Complex_7 | 38: Metal_Complex_8 | 39: Metal_Complex_Debris | 40: Wood_1 | 41: Wood_1_Up |
| 42: Wood_2 | 43: Wood_3 | 44: Wood_3_Up | 45: Wood_4 | 46: Plastic_1 | 47: Plastic_2 |
| 48: Plastic_3 | 49: Plastic_4 | 50: Glass_Thick_Large | 51: Glass_Thin_Small | 52: Concrete_Curb | 53: Concrete_Bench |
| 54: Leaves | 55: Bush | 56: Pottery | 57: Paper | 58: Cardboard | 59: Garbage_Bag |
| 60: Garbage_Spill | 61: Bottle | 62: Tile_Ceramic | 63: Marble_or_Slate | 64: Brick_Smooth | 65: Brick_Course |
| 66: Manhole_Metal | 67: Metal_Grate_Sewer | 68: Metal_Grate_Planter | 69: DeepSnow | 70: PackedSnow | 71: Ice |
| 72: Antennas | 73: Chandelier | 74: Plexiglass_Small | 75: Plexiglass_Large | 76: Potted_Plant | 77: Crumpled_Paper |
| 78: Cloth | 79: Pop_Can | 80: Paper_Cup | 81: Wire_Cable | 82: VolleyBall | 83: OilDrum |
| 84: DMORail | 85: Fruit | 86: Plastic_Bottle | 87: Drum_Pylon | 88: Metal_Rail_4 | 89: Wood_5 |
| 90: Metal_Ramp | 91: Complex_Plastic_1 | 92: Max_Mappable_Surface | 93: Board | 94: Truck | 95: Head |
| 96: Torso | 97: Leg | 98: Arm | 99: Foot | 100: HOMHead | 101: HOMTorso |
| 102: HOMLeg | 103: HOMArm | 104: HOMFoot | 105: Skin | 106: Denim | 107: Cotton |
| 108: BoneCrack | 109: BoneSnap | 110: FaceHit | 111: BoardTumble | 112: Metal_Hollow_Square_3_Dead_Rattle | 113: Metal_Hollow_1_Rattle |
| 114: Metal_Hollow_Square_3_Rattle | 115: Metal_Hollow_2_Rattle | 116: DMORail_Rattle | 117: Antennas_Rattle | 118: Metal_Complex_5_Rattle | 119: Metal_Sheet_Rattle |
| 120: Metal_Complex_1_Rattle | 121: Wood_1_Rattle | 122: Wood_2_Rattle | 123: Wood_3_Rattle | 124: Wood_4_Rattle | 125: Plastic_1_Rattle |
| 126: Plastic_2_Rattle | 127: Plastic_3_Rattle | 128: Plastic_4_Rattle | 129: Cardboard_Rattle | 130: Pottery_Rattle | 131: Pottery_2_Rattle |
| 132: eAudioMaterialType_Max |
