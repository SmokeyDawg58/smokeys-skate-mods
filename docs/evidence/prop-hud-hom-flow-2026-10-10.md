# Prop controls and Hall of Meat, October 10, 2026

Solo implementation, using live read-only IDA for the rich-symbol Skate 2 debug XEX. No IDA database edits. Existing unrelated work and the real `saves/SKATER.P` are preserved. The reference exports and inspection output for this work are under `.scratch/prop-hom-20261010`.

## Final result

Implemented the requested prop controls and automatic HoM flow. Confidence is **high** for reconstructed control flow/data backed by the addresses below; these PC runs do not establish instruction-level numeric parity with an original runtime.

- Final release build8 passed (10m52s); cargo checks12/13/14 passed. Existing compiler warnings remain. No tests were added or test suite run.
- Run14: RB dragging, D-pad Add/Remove/Add, saved-prop restoration, and no left-stick Reset regression. Run10 also exercised Reset/cull/respawn before the distinct D-pad mapping correction.
- Run18 on the final build: actual18,111-point/nine-break bail, one completed challenge, authored automatic highlight966..1407, flat challenge/points results, Annals tabs/path navigation, B return to newly created results, Continue, original FMV-HOM/reward handling, resumed gameplay and autosave. Captures run18_016/021/024/027 show Annals, returned results, tutorial movie and resumed world. The resulting version13 save has209 tuples and the expected nine/two/one/four/zero injury totals.
- Run17 reload: persisted injury/time/bail totals and completed/next-active challenge data rendered in Annals; trigger tabs and D-pad path selection work.
- The final runs use original assets and controller samples on scratch profiles. Real saves/SKATER.P remains unchanged (last write12:28:28, before this work). No live IDA database writes.
- Source-audited paths without final runtime coverage: upright torque execution, all mission/chase gate combinations, nonempty top-record save/reload and the top-record toe-tag variant. Run8 did verify the actual20,183-point top/PB/replay gate, before the later APT fixes. Audible output could not be checked because this runtime had no audio output device.
- The full Replay Editor FE12 remains a separate unported frontend subsystem; its results-menu row is inert. The automatic bail replay is functional. Other host/native differences are listed under Boundaries below.

## Prop controls

| Native owner / address | Connected behavior |
| --- | --- |
| `PhysOutModule_Hud::UpdateHUDsAndObjectGrabbing`, `827c6ee0` | Offboard interaction/grab selects the object; grabbed splines resolve their owner. Skitch hides the phone without replacing the cached prop HUD. A zero object ID preserves the previous state. |
| `SignUpHUD::Refresh/Update/UpdateInput`, `826bd2d0`, `826bd3e8`, `826bd7c0` | The existing HUD group priority and visibility rules select the prop overlay. Up uprights, down resets, right adds, left removes. Upright/reset honor their disabled flags; add/remove messages are sent even when the row is dimmed. |
| `DMO_GetParams`, `826b88c8` | Five original flags, including the native false capacity flag. The session marker existence and membership queries are `826b52c8` / `826b5350`. |
| `PlayerUI` messages, `828aabd0`, `828aac58`, `828aace0`, `828aad58` | Commands reach the live object/locator manager. Marker restore also restores its saved props. |
| `DynamicObject::IsUpright`, `82d2ac50`; upright update `82d2cfa0` | Runtime upright-angle cvar, source angle calculation, body inertia and torque path, and flag `0x40` clearing. |
| `CanReset`, `82d3a908`; reset `82d39e10` | Native obstruction checks and reset operations, including the actual HiLOD collision volume and skater capsules. |
| Session marker add/remove/clear, `828aedf0`, `828add60`, `828ad6a0` | Locator membership flag and saved transforms; linked props update recursively in native map order. Clearing/replacing a marker clears locator membership. |
| Map insertion/rehash, `828ad7c0`, `828abd00` | High-plus-low-word hash, full 64-bit equality, bucket-front insertion and the original 256-entry prime table at `822852f0`. PPC `cmpld` was checked where Hex-Rays truncated IDs. |
| Recursive update, `82d35850`; volume list overlap, `82e4bc00` | Unpadded transformed model AABBs first, followed by the native shape overlap with `0.05` padding. The fixed locator lists retain capacity 50. |
| Recursive restore, `82d37c70`; unused hand prop handling, `82d37710` | Saved-map iteration, obstruction handling, locator resets and full 64-bit null asset ID comparison. |

The screen is the original `hud2/actionoverlay2` APT, fingerprint `9bd9bc9c`, template `82fdb8f8`, placement 0. Its `InitDMOHUD` action at `3bf8` supplies the row dimming, D-pad icons, localized add/remove labels, wrapping offset, and backer geometry. The native `SetScreenState` and the screenmanager APT do not cache repeated state values; the DMO state applies its authored intro on each refresh. No blanket mission exclusion was invented: the component's existing HUD group rules control availability.

## Hall of Meat

| Native owner / address | Connected behavior |
| --- | --- |
| `ChallengeHOMManager::CanEnableHOM`, `825ae488` | No sign-up in progress, no main challenge, no chase. |
| Manager update, `825b30a8` | Single-player flags 4; defer enabling while a bail is in progress/completed, but disable immediately; switch objective evaluation and reset on disabling; marker permission depends on completed challenges and personal-best score. Start the authored manager when replay becomes ready. |
| Wipeout handlers, `825b3290`, `825b6110` | Local-player bail start frame and final score; statistics update; automatic replay on completed HoM challenges or a new personal best. |
| `StatsBackEnd::UpdateBailStats`, `8257da60` | Lifetime bail/time/collision totals; eligible last bail, injuries and tweaks; minimum 20,000 for top-ten records; strict-greater insertion, preserving tie order; top-bail and personal-best flags remain distinct. |
| `TrackBailStatsAndAchievements`, `82569348` | Native statistics/PB recording gates, injury/drop/score records, pedestrian/security/vehicle totals, and one-time achievement requests with Vault thresholds. |
| `SaveData` / `LoadData`, `8256e6b0` / `8257d640` | `STATISTICS` / `StatsBackEnd::Data`, version 13: 45 booleans, 65 statistics, 13 DMO statistics, 22 PB values, 52 object movement times, 40-byte totals, record count, ten 1,360-byte result slots. Complete 1,264-byte bail outputs are retained; the host supplies local calendar time. |
| `VDWriter::UpdateHoMSlowMotion`, `827fd448`; `WriteData`, `827ffb2c` | Authored normal/severe/ending curves, min/max selection in source order, effect/cvar/bail gates, and one-tick simulation rate requests. |
| `FrontEndState_HOMResults`, `82642348`, `826591e8`, `826486c0` | Paused results state 9, challenge and points pages, Continue's HUD event and authored reward/tutorial state machine, cash audio, Annals transition and return. |
| Annals entry/input, `826ae5a0`, `826a1e68`, `826a1ef8` | FE state 77, initial tab based on records, trigger-tab cycling with empty-top-bail skipping, B return, top-bail arrows, challenge-path selection. |
| FE stack, `82d69cf0`, `82d694e8`, `82d672f8` | Pushing Annals closes the parent screen; popping recreates it. `PrepareInput82d67808` restores input allowance each pass. |

Results use the shipped `hudComponents/hom_results` (fingerprint `5b4276e3`, screen 46, placement 0). The shared `controls/skeleton` is `7b44af51`. Annals is `b0e2d7f4`, screen 128, template `82fdbb58`, placement 0. Their APTs, fonts, textures, labels, timelines, button/menu controls and sound callbacks are used directly through translated handlers. The November and January archives contain identical compressed bytes for these three movies.

The skeleton preserves the original bone map, duplicate hip mapping, groin/toe-tag behavior, per-bone delays and distinction between animated results and static Annals highlighting. Case-insensitive APT frame-label lookup was confirmed at `829da970`. Results use plural injury labels for counts above zero; Annals uses them only above one. Cash accumulation retains floating-point updates and the full native sound key `52436b5267fae4b4`.

Annals challenge paths follow completed order first, then authored active order (`825d6280`); the native completed-count binding counts both complete and active entries. Date and location use the host clock and original WorldPainter/FELocation labels (`8261bbb0`, `82615b68`, `82981ad0`). The offline backend reports disconnected online statistics.

The visual effect gate is separate from challenge eligibility: `IsHallOfMeatEffectEnabled827acbd0` checks the effect cvars, front-end state, Free Skate mode, manager visuals and the career's `Hom Available` setting. A main challenge disables ambient HoM challenge evaluation through the manager; the visual/slow-motion gate follows its own native rules and authored Lua calls.

## Build and runtime verification (chronological record)

`cargo check -p skate2-pc -j 1` passed in check12, check13 and check14. Release build2 passed (12m 51s). Build3 was deliberately cancelled after the packed-vector save issue was identified; build4 includes its correction and is queued to run the next diagnostic automatically. Original-data runs use copies under `.scratch/prop-hom-20261010/save-*`, never the real save.

Runtime evidence so far (build2):
- run4: a real 450-point bail reached WipeoutOver, eligible=true, no replay, as expected below the record threshold without a completed challenge. Direct results inspection caught the missing InitAction registration at asset81/character175/0xfe70. It is now registered, together with Annals189/0x9770 and the three original skeleton class registrations.
- run5: the authored dam location yielded 14,350 points and four broken bones, no new personal best or completed challenge, so no automatic replay.
- run7: the authored Mega Compound location yielded 19,718 points and 12 broken bones, completed one HoM challenge and set replay_ready=true. The authored manager ran highlight_generic_right over recorded frames991..1436 (bail start1051), returned through its complete state and requested OpenHomResults. This run stopped at the same APT registration error, so successful results display is still pending.
- Map travel grounds the skater; the early high-Y trial was not evidence of a high fall. PAD uses front-end ticks, whereas PAD_AFTER/TRIGGERS/STICKS use ticks since the career was created. Runtime scripts now use matching clocks. Use --live-completions with the career end tick; --live-frames counts rendered frames and ended the first two attempts early.
- Real skeleton Part transforms intentionally expose unknown packed W. PDB inspection identifies their adjacent native ID/pointer fields; the portable save representation and provenance mask are documented below. Physics lane knowledge is unchanged.

The opt-in `SKATE2_DIAGNOSTIC_HOM_SCREEN=<frontend tick>:results|annals` opens the original screens using existing recorded data, without injecting scores. `SKATE2_TRACE_CAREER` reports bail eligibility, score, records, completed challenges and replay readiness. `SKATE2_TRACE_MARKER_UI` reports prop actions.

## Boundaries to retain in review

- The automatic bail replay uses the existing authored challenge Lua, NIS and presentation recorder. The separate results-menu Replay entry targets FE state 12, the full replay editor, which was absent before this task; its scope is a separate question. It must not be described as working merely because a destination field is set.
- The existing replay recorder uses a frame-count approximation for its byte budget. This work does not establish byte-for-byte recorder parity.
- Platform achievement delivery and online leaderboards require backend services outside the offline implementation. Native statistics and achievement state are retained locally.
- Original saves retain meaningful fields and loaded raw bail images. New native structure padding is canonical zero. PDB RigidBody identifies the fourth mCom lane as adjacent mId (0x1c), and axis tails as mStasis/mInertia/mTag; Part.GetTransform826c3480 copies/mixes those packed bytes. A new portable bail save writes zero for unavailable fourth bone-position lanes and retains an explicit unrecorded_bone_w provenance mask in memory. Gameplay vectors keep W unknown. Loaded raw bytes are preserved; byte identity for nonportable native metadata is not claimed.
- Exact instruction-level floating-point parity needs reference traces. IDA-confirmed branches/data and a compiling implementation alone are not runtime parity evidence.

## Runtime follow-up

- Build4 passed (11m48s). Run8: 20,183 points, ten broken bones, one completed challenge, top/PB/replay_ready true, save_image true. The automatic highlight completed and requested results. The missing mCheckMark then stopped results construction. The original character147 has no such instance; its ActionScript assignment is a no-op. Character146 also has no labeltext_glow. Those two absent targets are now handled explicitly, and the original icon==none branch skips goto/progress writes.
- The initial slowdown trace was zero because visualtreatments_bail was omitted from selected_classes. VDWriter constructor827feae8 and generated constructor827fccc8 PPC confirm class d552e0b10a9a104f and default d7edbd362d7d2152. The class is now loaded by the production Vault.
- Run10 displayed the authored prop overlay and switched add/remove using D-pad. Reset culled object83886089 and respawned its locator feafe4f7fd13ebdc. The walking input exposed an incorrect routing through generic AptDown: analog-stick navigation also reset the prop. TranslateEHudInputBit82641648 accepts separate FE keycodes16..19 and32; LoadInputMap826de5e8..618/6e8 binds AptDUp/Down/Left/Right and AptDUpReleased to150..153/166. The HUD now reads those original definitions from the same retained PRES sample. Menus keep their generic direction inputs. Live verification of that correction is pending.

- Build5 passed (9m18s). Run13 confirms the loaded authored slowdown curves: 415 rate samples, minimum0.3275764, maximum1.0, final1.0. Its real 18,786-point/nine-break bail completed a challenge and started the automatic highlight despite not being a top/PB record. The screen reached the renderer and exposed missing texture-owner registrations81..83, now added using the original RX2 texture-name prefixes. Run11 separately reached Annals construction and the same renderer boundary. Build6 includes the registrations and distinct HUD D-pad input route; full results navigation is queued after it.

- Build6 passed (12m51s including the wait for build5). The queued run12 is now checking results navigation, Annals tabs and Continue with the final renderer/input fixes.

- Run12 reached the challenge results, animated broken bones, points and Annals with controller navigation. Visual inspection caught the missing flat-projection registration. Raw template entries82fdb638 (HOMResults) and82fdbb58 (AnnalsOfMeat) both contain type0/placement0. Assets81/83 now use the flat stage; imported skeleton controls inherit the owning screen projection.


- Build7 passed (1m59s). Run14 on that build completed normally: the bin locator feafe4f7fd13ebdc was grabbed and moved from [1210.7432,63.98648,-72.88613] to [1203.6855,64.28821,-66.34194]. Its D-pad actions were exactly Add/Remove/Add, with no Reset emitted by the left stick. Holding LB+Up restored the saved transform, culled object83886089 and respawned object83886278 at [1210.7432,63.98648,-72.88613]. Captures run14_003/007/010 show the overlay, moved bin and restored bin. Upright was dimmed when the scripted Up arrived, so this run does not verify upright torque execution.
- Run12's B return from Annals exposed a queued frontend output that the overlay-only pause path never consumed. Native ExecutePopState82d672f8 re-enters the parent through UpdateOnEnter while that parent still pauses simulation. The overlay return now applies pending career outputs before retaining the frame; no simulation advance or extra challenge update is introduced. Build8 contains this correction; the return/Continue check is pending.

- Run16 (build7): real 17,773 points, seven breaks, one completed challenge. The automatic highlight returned to correctly flat results; A selected the points page, then Continue. The authored FMV-HOM branch ran, gameplay resumed and AutoSave completed. Read-only inspection of the scratch SKATER.P reports StatsBackEnd version13, 209 tuples, totals seven breaks/two dislocations/three sprains/four bruises/two scrapes/one tweak, 6.1833339 seconds and one bail. Its score is below20,000, so zero top records is correct; all ten reserved records are1,360 bytes. Captures run16_009/013 show challenge and points pages. Run15's later bail input produced only750points/no replay and was deliberately stopped after that result.

- Run17 reloads run16's saved profile. Annals displays the same seven/two/three/four/two injury counts, one tweak,6.2seconds and one total bail. The trigger opens Challenges and D-pad Down selects Body Damage, showing the completed5,000-point challenge/$100 and the next active Break Both Hands challenge. Captures run17_001/005 verify saved statistics and progression rendered from the original Annals movie.

- Build8 passed in10m52s (existing warnings only). It includes the paused parent-screen re-entry correction. No test suite was added or run; verification here consists of live IDA/PPC/original-data audits, cargo check/release builds and controller-driven original-asset runs.
