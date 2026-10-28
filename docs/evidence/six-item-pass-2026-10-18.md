# Six-item pass (2026-10-18)

## Cutscene fades: 0.2 s

`TransitionPresentation::StartTransition` (`0x827db108`) gives `BloomToWhiteTransitionEffect` a
time and duration of 1.0. The port uses 0.2 for all four fades (before the scene, into it, at its
end, back to the game): `nis/transition.rs` `DURATION`. At 60 Hz the f32 countdown completes on
the 13th update (the 12th leaves 1.1e-8). The PostFX-off white cover still follows the bloom value.

## Mission-end replays: pedestrians animate

`LivingWorldPresentation::DoSimSubmit` (`0x8280e8b0`) writes the LivingWorld manager's animation
command buffer at the start of every recorded frame. `DoSimRestore` (`0x82813a90`) reads it back
from the shown frame (`mRestoreDataContainer[0]`) into `AnimCommandBufferData`, and the
presentation's `FullPedestrianACS` (`InitRegularPedestrianAnimSys` `0x8280cd40`, callbacks
`GetPedestrianPoseBufferFromCharacterID` `0x82809a58` and friends) runs it into the presentation
pedestrians' own pose buffers. `UpdateLivingWorldPresentationEntities` (`0x828133e0`) blends each
entity's transform from the previous frame and keeps only the pedestrians, vehicles and dynamic
objects the frame records. `cPresPedestrian::UpdateFromReplayData` (`0x82809d98`) restores the
six post-adjust bone rotations (`tPedestrianReplayData.BoneAdjusts`, 0x80-byte record) and
`BlendPoseBufferIndex`.

The replay editor already drew a frame's recorded pedestrians with their recorded poses
(`RecordedWorld`, `restore_replay_pose`). A replay-controlled NIS drew the live pedestrians,
whose palettes stand still while the simulation is paused, at transforms from the restore: they
slid. `try_nis_visual_snapshot` now takes the shown frame's recorded pedestrians (their poses
restored into the playback palettes), vehicles and dynamic objects while a replay clock runs.

## Replay Editor saves: resolution and frame rate (PC addition)

After the Save Replay confirmation the editor asks for 720p or 1080p, then 30 or 60 FPS
(`Dialog::Resolution`, `Dialog::FrameRate`, `VideoSettings`; each opens on the last choice,
1080p / 60 by default). The capture renders at the chosen height (`begin_media_capture(lines)`)
and the encoder runs at the chosen rate. The export still steps the replay at 60 Hz with each
step's audio; a 30 FPS video encodes every second step, and the steps between are still drawn so
the per-frame post-fx adaptation (`PostFX::Tonemap` exposure, colour cube blend) runs as at 60.

## Photographer aim

`PhotographerActions::PhotographSkater::Update` (`0x82717018`) writes `PitchToTarget` and
`BearingToTarget` (`AddSettableAtt`) for the `PHOTO_SHOOT_BLEND` animation.
`LivingWorldOrientationOps::GetBearingToTarget` (`0x82e5acc0`) takes the sign with
`fsel f11, f12, f0, f13` (`0x82e5af10`) on `(forward x offset).y` (decoded from the raw
`vpermwi128 0x63` / `vnmsubfp` words): f0 is the stored -1.0 (`var_30`), f13 the stored 1.0
(`var_20`), so a non-negative cross gives -1. The port had the opposite sign, which mirrored the
aim to the other side. The other caller, head tracking (`UpdateWanderDynamic`), only compares the
absolute bearing. `GetPitchToTarget` (`0x826fd8b0`) was already right (target minus current).

## DMOs freezing in the air

`World::Initialize` (`0x827b0e58`) builds both `SimulationManager::Params` with
`freezingThreshold` 1e-5 (`+0x20`) and `numFramesBeforeFreezing` 2 (`+0x24`). The manager
constructor (`0x82e0c298`) stores the threshold as `Simulation.m_MinEnergy` (`0x82e0c70c`) and
the frames as `m_CoolDown` (`0x82e0c71c`); `FreezeIfNecessary` (`0x82e0be40`) freezes an Active
body whose `mCool` reached the frames. `DynamicUpdate` counts a frame toward freezing while
`spherical * inverse_mass * |omega|^2 + |v|^2` is below the minimum energy and not rising.

The port's World projection set the frames and the cooldown but left the minimum energy at the
pool's initial 1.0, so anything slower than about 1 m/s (a thrown object near the top of its arc)
froze after two frames. `try_initialize_world_feedback_parameters` now stores 1e-5
(`0x3727c5ac`).

## Pick a Pro sign-up: B gives the pause menu back

`cChallengeBEManager::OnChallengeSelect` (`0x825b7a18`) sets
`StateManagerFrontEnd::mbDisableBackpackAndPauseMenu`; `OnChallengeAccept` (`0x825b7a40`) clears
it. The manager's vtable (`0x820539ac`: select, accept, decline) holds `OnChallengeAccept` in the
decline slot too (`0x820539b4`; the effects manager's `0x82350878..80` shows the slot order), so a
declined sign-up clears the flag as well. The port cleared it only on accept, so backing out of a
Pick a Pro poster left Start (and the backpack) disabled until the next accepted sign-up.
`signup::player_decline` now clears it.

## Demo challenges: the remaining Localizer arguments

All sixteen `FEBackEnd` custom argument types are translated:

| Type | Callback | Text |
| --- | --- | --- |
| `POOL_FOUND` | `ProcessPoolFound` `0x82563320` | `%d` of the `Pools Found` points pool |
| `POOLTOBEFOUND` | `ProcessPoolAboutToBeFound` `0x825633e0` | the same, plus one |
| `TAPEGIVEN` | `ProcessTapesGiven` `0x82563960` | `%d` of `Circulate Pool` |
| `SponsorWorld` | `ProcessSponsorWorld` `0x825634a0` | `GetBoardSponsorKey` (`0x825d1198`, the `Board Sponsor` setting)'s `sponsor` world |
| `DemoWorld` | `ProcessDemoWorld` `0x82563610` | `GetDemoSponsorKey` (`0x825d1218`, the beaten pro's sponsor)'s world |
| `DEMOSPONSOR` | `ProcessDemoSponsor` `0x82563780` | `GetBoardSponsorHALID` (`0x825d1358`), localized |

`GetPointsPoolPoints(name)` (`0x825cf148`) is -1 for an unknown pool. A sponsor, world or text
that is missing reads `kMissingSponsorName` ("Online"). `kForceBoardSponsorWorldToAlien` (default
off) is not in the cvar table. The values travel as `CareerHud::frontend_arguments`.

## Highlight reels: the definition's location

`cHighlightGroup::PlayHighlight` (`0x82cd75e0`) hands `NISManager::RunReplayNIS` (`0x828a3c70`)
an `NISPlaybackOverrides`:

- speed and track as for a basic NIS;
- `focusPlayerIndex`;
- `useLocationValues`, with the definition's `LocationOverride` position and direction (a zero
  direction becomes +z);
- `useStreamPositionOnReturn` with `streamPosition` = `focusRestore` when a component exceeds
  `FLT_EPSILON` (`0x82cd7794`).

It also passes an `eNISSpecialFlags` (`0x82cd77c8`): `NIS_SPECIALFLAG_HALLOFMEAT` (3) for a HoM
group, otherwise `NIS_SPECIALFLAG_FINALHIGHLIGHT` (1) for a highlight not stitched to the next.

`ReceiveNISEnterReplayMsg` (`0x8286c0f8`) gives both to the scene player.
`UpdateLoadState` (`0x82875bc0`) places the scene at the override (normal +y) instead of the
directory record.

`PlayHighlightGroup` (`0x82cd7b68`) takes `GetRestorePosition` (`0x828a1418`, the local skater's
position) when the streamer is restored: the only highlight returns there; a longer reel keeps it
in `FocusRestore` (`+0x150`) for its last (`OnNisCompleteMsg` `0x82cd79f0`).
`PlayHighlightGroupWithSpecifiedOrder` (`0x82cd7d30`) clears both.

The port's replay NIS reset the overrides to the default, so a highlight definition's camera
played at the scene's authored location. `m_4f_01_pro_brown`'s objectives use
`Replay_Generic_High_Wide` and `Replay_Generic_Low_Long` with `UseLocationOverride` (at
(749.41, 36.99, -189.76) and (745.17, 26.19, -173.79)), so its reel showed another part of the
map. `focusPlayerIndex` has no reader in the scene code. The special flags reach the audio's
`NotifyNISPlaybackStart`; `MusicSystem::CheckReplayState`, which reads `FINALHIGHLIGHT`, is not
ported.

## Online boundaries: the bailed board froze in the air

`PlayerUI::Update` (`0x828af9f8`) runs every simulating frame. It tests
`Actor::CalcRespawnTransform` (`0x825f09a8`)'s translation with
`TestForNetworkGameCollisionConstraints` (`0x8289e510`, `IsPositionOk` `0x82da4d30`, x/z only,
radius 0). Outside, it sends command 5 to the `GetSuggestedRespawnPosition` (`0x82ce2698`)
crumb, a wipeout spot or ring crumb that passes `_TestForCollisionConstraints` (`0x82ce19b0`,
which keeps crumbs 13 m inside).

`CalcRespawnTransform` uses the skeleton (`mEffectiveSkeletonRootMatrix`) only while
`PhysOut_State.mIsOffboard` (`+0x41`, stored as `category == 500` at `0x82e05724`). In a wipeout
it uses the board (`PhysOut_SkateboardReckoning.mEffectiveTransform` + 0.2 y). This whole path
matched.

The board after a bail is a free body. Under the port's `m_MinEnergy` of 1.0 (see "DMOs freezing
in the air"), a bailed board slower than about 1 m/s, as at the top of a high bounce, froze after
two frames and hung inside the boundary while the skater flew out, so the respawn transform never
left. With 1e-5 the board keeps flying and the reset follows it.

`InvisibleBoundaryManager::Update` (`0x82da72f0`) measures the warning at the tester's
`Actor::GetBasePosition` (`0x825ed928`): midway between the feet in a wipeout, else the
position. The port measured the skeleton root; the warning now uses the base position.

## skate.Profile: Career

`IStartHubState::OnMenuSelect` (`0x82674d08`) answers the hub's skate.Profile row with
`FrontEndState_SkateProfile::mEntryScreenMode = eScreenMode_Offline`, the local peer and
`PushState(45)`. FE state 45 (`gFEStateInfo` row 45: `mbPause`, `mbEnableChyron`, `mbDimmer`,
`mShowHUD` Hide, music state 9, `mbInputToActionscript`) opens screen 93, the flat template
`SkateProfile` (`0x82fdb928`). Its `ReceiveInput` (`0x826621d0`) pops the state on B with
`FE_SOUND_CORE_B_BUTTON`; keys 6 and 9 open the Xbox LIVE gamercard and achievements, which the
PC has no service for, so the button help leaves their bumper items out. The screen takes input
from its `IntroComplete`, and the pop enters the StartHub anew.

`skateProfile.apt` (retail `0xfee31040`, which only renumbers registers against the debug build's
`0x56bab8fa`) lays out the offline screen in `SetState(1)`: the Stats and Personal Bests tabs
spaced over 694 pixels, `mProfile` and `mContent` on `sp`, the title
`ID_ONLINE_SKATEPROFILE_CAREER`, and the `dynamic_selectable_list` (row offset 24, small font,
columns 598 and 90 wide, at most twelve rows) with its `basic_slider`. `OnLoaded` (`0x82662000`)
populates the tab again. The released triggers (`AptRShift2Released` 185, `AptLShift2Released`
188) step the tabs with `FE_SOUND_CORE_TAB`; up and down step the list and its slider with the
navigation sounds. The gamer tag, the rank and the online thumbnails stay at alpha 0 or above the
screen offline.

The rows are `StatsBackEnd::GetStatsValue` (`0x82568270`) of the 50 visible statistics of
`StaticStatsInfoTable` (`0x82fd9db8`) and `GetPersonalBestsValue` (`0x8255f1d8`) of the 22
personal bests:

| Text | Statistics | Format |
| --- | --- | --- |
| Time | 0 (the DMO running time), 4, 5, 19, 21, 30, 32; bests 0, 2, 17, 20 | `FormatTime(.., true, 0)`, a space and `ID_COMMON_METRIC_TIME` |
| Cash | 9 to 11, 23, 27, 28 | `FormatCurrency(.., true)` |
| Percent | 3, 25, 35 | `ID_LOGBOOK_STATS_PERCENT_INT`, `{0:f.0}%` |
| Distance | 18, 20 (`%d`); 29, 31, 39, 40, 49 and bests 3 to 12 and 14 (`%.1f`) | `ID_COMMON_DISTANCE` |
| Number | the others, among them 1 (object types moved for 1e-4 s or more) and 8 (decks) | `FormatNumber(.., true, 0)` |

Statistic 8 first stores `CACRecord::PartCount(SKATE_DECK)`. A personal best that does not reach
its threshold (NaN included) shows `ID_COMMON_HYPHEN`. The Localizer's typed `f` token
(`eFORMAT_ARG_FLOAT`) is `FormatNumber(value, false, mPrecision)`, the precision being the digit
after the token's `.`.

## skate.Profile statistics

The profile's rows move with these producers. Each one updates `StatsBackEnd` through
`UpdateStats` (`0x82568b90`: `IsRecordingAllowed` unless forced; 8, 25 and 64 store the value,
the others add it; 14, 29 and 38 grant Uninsurable, Skitched Up and Taste The Mongo past
`MinBonesBrokenForAchievement` less 0.1, `MinSkitchDistanceForAchievement` and 4999.5),
`UpdatePersonalBests` (`0x8255f4c8`), `RecordPersonalBests` (`0x82a72f40`) and
`GrantAchievement` (`0x8255f580`).

| Statistics and bests | Producer |
| --- | --- |
| 2 (forced), Gender Bender | `FrontEndState_CaC::UpdateStatsAndAchievements` (`0x82680b20`) from `LeavingCACConfirmationCallback`, when the gender differs from `mIsOriginallyMale` (`InitiateStartup`) |
| 9 (forced) | `FrontEndState_Receipt::OnMenuSelect` (`0x826810b0..0x826810f4`): each purchased line's cost, after `SubtractCash` |
| 10, 12; 11 (forced), 13 | `cSignPostManager::IncrementPointsPoolPoints` (`0x825d78d0`) on the cash pool (type 43): a gain and its sponsor bonus to 10, the gain to 12 with credit 1; a loss's magnitude to 11, and to 13 with credit 1 |
| 19 | `FrontEndManager::Update` (`0x82640130`): `16 * 0.001` per in-game frame the front end does not pause |
| 17 | `UpdateWipeouts` (`0x827c47b8`): once per wipeout, from `mIsWipeoutOver` until the filtered state is ground |
| 18, 20, 29, 30, 49; bests 8, 9 | `UpdateDistanceTravelled` (`0x827c5e48`): distances above 0.005 per frame bank whole metres (`BankStatistic` `0x827c4900`, `BankPersonalBest` `0x827c4998`) |
| 21, 31, 33, 36, 39, 40, 42; bests 3, 4, 5 | `UpdateTrickCompletion` (`0x827c5af0`) |
| 32, 34; bests 2, 6, 7, 10, 11, 12 | `UpdateAirFinished` (`0x827c5818`), on a successful landing |
| 35 | `PhysOutModule_Hud::Update` (`0x827c79b8`): `mIsAttemptingABodyFlip` |
| 37, 38 | `UpdatePushingStats` (`0x827c4338`) |
| 41 | `UpdateTricksCancelled` (`0x827c5a30`): cancelled tricks that were not tapped |
| 43, 46, 47 | `UpdateAirContextData` (`0x827c6c40`): the accumulated context less the take-off's and the landing's; group 0's first bank (pedestrians), its entity type 3 (`GetEntityTypeCount` `0x82de8ae8`), group 1's first bank (vehicles) |
| bests 16, 17, 18; dynamic objects 3 to 12 | `UpdateSequenceCompletion` (`0x827c4668`) and `CheckForObjectSequences` (`0x827c4508`, ten lists of `eEntityType`s, `DoesObjectSetContainObjects` `0x827c4438`) |
| bests 19, 20, 21 | `TricksBackEnd`: `SetLineScore` (`0x82a73588`) through `UpdateLineScoreStats` (`0x8255c2d8`), `Update` (`0x82564a88`, the line time and the line's end), `UpdateTrickLandedStats` (`0x825770e8`) |
| dynamic object 0, movable object times | `UpdateHUDsAndObjectGrabbing` (`0x827c6ee0`): a grab spline's owner (`GetGrabSplineOwnerObjectID`) moved more than 1e-4 squared from where the grab began adds 1/60 s to `UpdateMovableObjectStat` (`0x8255f158`, the characteristics layout's `MovableObjectType` at +0x68) and `UpdateDmoStats(0)` (`0x82569090`) |

`PhysOutModule_Hud::Update` samples `AreStatsAndAchievementsAvailable` (`0x827c4ac0`) into +0x4A
and calls `ClearPersonalBestFlags` (`0x82a72ed0`) first. Its achievements: On Top Of The World
(the finished handplant's centre of mass above key `0x5b1d5b4f677cae0d`), Skater Evolved, I Like
To Move It, DIY (`MinTimeSpentMovingObjectsForAchievement`), Need For Speed (the damped centre of
mass speed above key `0xe4f1a7b85e6528e3` on the ground past `MinTimeAtMaxSpeedForAchievement`),
Grasshopper and Stairmaster (`MinGrassGapLengthForAchievement`, `MinStairGapLengthForAchievement`).
Every threshold test is `fcmpu` followed by `ble` past the update or `bgt` to it, an ordered
greater-than that NaN fails.

The statistics record only in game mode 1, so the career save writes them; the challenge code's
calls apply with the frame's outputs and again before each save. `UpdateStatsAndAchievements`'
statistic 8 (shiny new owned decks) is not kept, since `GetStatsValue(8)` stores the record's
deck count before it reads. The Create-a-Spot tutorial's statistic 64 and the trick book's
per-trick counts have no ported producer.
