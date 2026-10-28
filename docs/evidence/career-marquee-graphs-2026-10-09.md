# Career marquee and photographer graphs (2026-10-09)

Native: `sk82_na_m.xex` (Jan-17-2009 debug), read-only IDA. Every address below was read in the
IDB (decompile, and disassembly wherever Hex-Rays dropped asserts, 64-bit arguments or vector
math). Data facts come from the shipped vaults (`skater.vlt`) and `PedestrianSkeletonPres.abin`.

## Summary

- The challenge characters' four generated state graphs are ported: `HCDelegate_marquee`
  (`marquee.xml`), `HCDelegate_Photographer` (`Photographer.xml`), `HCDelegate_motiongraph_marquee`
  (`motiongraph_marquee.xml`) and `HCDelegate_Motiongraph_Photographer`
  (`Motiongraph_Photographer.xml`), with their expressions, state updates, transitions, the seven
  behaviours and three conditions that `Pedestrian.xml` does not have.
- The pedestrian AI runner (`PedestrianGraph`, `LocalBehaviourBindings`, `PedestrianConditions`) and
  the motion runner (`PedestrianMotionDelegate`, `MotionObjects`) now take the delegate the graph
  file selects. The `Pedestrian.xml` / `MotionGraph_Pedestrian.xml` paths call the same code with
  the same tables as before.
- The `ai_graph_ported` guard in `session/selected_session_career_living_world.rs` is gone: the
  character manager's spawn now loads the marquee model, spawns the census pedestrian and runs its
  graphs (section 1).

## 1. Native chain

| Step | Native | Address | Port |
|---|---|---|---|
| Spawn request | `cChallengeCharacterManager::TestBBoxAgainstTree` | 0x825ab590 | `challenge/characters.rs` `LivingWorldOp::Spawn` |
| Model load | `RecipeMan::LoadLivingWorldModel` → `RecipeFactory::CreateMarqueeLoader` | 0x82814998 | `SelectedRecipeFrame::load_living_world_model` (unless a skater holds the schema, section 7) |
| Marquee recipe | `MarqueeLoader::MarqueeLoader` | 0x82d0b110 | existing `MarqueeLoader` name round trip |
| Census spawn | `ILWCensusMan::SpawnPedestrian` → `Pedestrian::Pedestrian` | 0x82e97748 | `spawn_census_pedestrian` → pedestrian factory |
| AI graph | `HierarchicalController::Load` (root name hash at 0x82cef218) | 0x82cef180 | `PedestrianConstruction::load_graph` (`pedestrian_model_init.rs`) |
| Delegate choice | generated delegate by graph basename (`GENERATED_STATE_GRAPH_DELEGATE_BASENAMES` 16 / 19) | — | `AiDelegate::from_delegate_index` |
| AI controller | `TemplateHC<HCDelegate_marquee>` / `<HCDelegate_Photographer>` constructors | 0x82947490 / 0x8294cac0 | `LocalBehaviourBindings::for_delegate` |
| AI conditions | `HCDelegate_marquee::Init` / `HCDelegate_Photographer::Init` | 0x82927698 / 0x829279e0 | `PedestrianConditions::for_delegate` |
| Motion graph | animation data RefSpec text +0, basenames 17 / 18 | — | `selected_living_world_pedestrian_animation.rs` → `MotionDelegate::from_delegate_index` |
| Motion controller | `TemplateHC<HCDelegate_motiongraph_marquee>` / `<HCDelegate_Motiongraph_Photographer>` | 0x82949300 / 0x8294ae88 | `PedestrianMotionDelegate::new(.., delegate)`, `MotionObjects::bind` |
| AI update | `TemplateHC<..marquee>::Update` / `<..Photographer>::Update` | 0x829575b8 / 0x82957a68 | `PedestrianGraph::update_delegate` (`IndexSet<3>`, 4 / 6 instances) |
| Motion update | `TemplateHC<..motiongraph_marquee>::Update` / `<..Motiongraph_Photographer>::Update` | 0x82957748 / 0x829578d8 | `PedestrianMotionDelegate::update` (`IndexSet<6>`, 3 instances, no condition cache) |
| Cull / release | `ForceCullChallengeSpawnedLivingWorld` / `ReleaseModelData` | 0x825a85f0 / 0x825a4800 | unchanged (`career-challenge-characters-2026-10-09.md`) |

Entities and graphs (`livingworld_entities` text 0xca4d0ad4ea45965b; animation data RefSpec
0x00367b8f33e79c43, class 0x53e332f67b8124b2, cached-layout text +0):

| Entity | AI graph | Animation data | Motion graph |
|---|---|---|---|
| slappy, cuz | `marquee/marquee.xml` | 0x1639cbc9a68e92fe | `marquee/motiongraph_marquee.xml` |
| giovanni_reda, mike | `marquee/marquee.xml` | `marquee` | `marquee/motiongraph_marquee.xml` |
| attiba_jefferson, michael_burnette | `marquee/Photographer.xml` | 0xd2b7efdf5bfb5d2b | `marquee/Motiongraph_Photographer.xml` |

All six carry the chase, locomotion and take-down RefSpecs `PedestrianResources::load` requires.

## 2. Generated tables

Exported from the IDB by `tools/ida/export_marquee_graphs.py` (reads the `?k*@HCDelegate_*`
symbols by item size) into `living_world/pedestrians/pedestrian_marquee_graph_data.rs` and
`animation/graphs/pedestrian_marquee_motion_data.rs`.

| Delegate | kStates | States | Transitions | Behaviours | Conditions | Attributes |
|---|---|---|---|---|---|---|
| `HCDelegate_marquee` | 0x821da2f0 | 5 | 1 | 5 | 3 | 3 |
| `HCDelegate_Photographer` | 0x821db730 | 7 | 1 | 9 | 5 | 5 |
| `HCDelegate_motiongraph_marquee` | 0x821da810 | 11 | 5 | 8 | 6 | 23 |
| `HCDelegate_Motiongraph_Photographer` | 0x821db0e0 | 17 | 8 | 12 | 10 | 35 |

Expressions (`pedestrian_graph_expressions.rs`, `pedestrian_motion_expressions.rs`):

- marquee `<1>` 0x828ff168: 0 NeedsToBeginColliding, 1 !IsPedestrianColliding, 2 IsFacingSkater.
  `<2>` is inlined in `GetStateActivation<2>` 0x82948438 and reads only expression 2; every other
  state sustains.
- Photographer `<1>` 0x828ff608: 0, !1, 2 IsPhotographerTakingPictures, 3
  IsPhotographerReviewingPictures, 4 IsFacingSkater; `<2>` 0x828ff728: 2, 3, 4.
- motiongraph_marquee `<1>` 0x828ff220: 0..5 read the condition of the same index, 6 is !0;
  `GetStateActivation<2>` 0x82949d68 sustains every state.
- Motiongraph_Photographer `<1>` 0x828ff388: 0..5 as above, 6 !0, 7 → 6 (HasIntent
  PhotographSkater), 8 → 7 (LastState Photographer.Idle), 9 → 8 (LastState
  Photographer.TrackTarget), 10 → 9 (HasIntent Review); `<2>` 0x828ff590 reads only 7.

AI state updates (`pedestrian_graph_updates.rs`; each calls `DisableNavigation.Update` first, then
the state's slots; empty slots are the shared 0x825187c0):

| State | StateUpdate | Slots |
|---|---|---|
| Marquee / Photographer | 0x828f88f0 | DisableNavigation, LockToCurrentPosition (empty) |
| TrackingSkater | 0x828f8950 | + ProTrackSkater |
| Colliding | 0x828f8a20 | + PedestrianColliding |
| HoldAtSpawn | 0x828f8b98 | + ProWaitingForSkater (empty) |
| PictureTaking | 0x828f8a90 | + PhotographSkater, IgnoreCollision (empty), SendSpeechEvent (empty) |
| CheckOutPictures | 0x828f8b28 | + ReviewingPictures |

The motion graphs' state updates are the shared bodies 0x828f5c28 (one animation), 0x828f81d0,
0x828f81e8 and 0x828f89c0 the pedestrian motion backend already dispatches; their behaviours
(`PlayRemappedAnimation`, `SendSpeechEvent`, `MajorIntentComplete`) and conditions (`HasIntent`,
`WillExpire`, `LastState`) are the existing pedestrian motion implementations, bound to the
delegate's own attribute rows.

## 3. AI behaviours and conditions

`living_world/pedestrians/pedestrian_graph_marquee.rs`; lifecycle rows in
`pedestrian_graph_lifecycle.rs` (create, begin, update, end; 0x82bb3da8 null factory, 0x825187c0
empty).

| Behaviour (vtable) | Callbacks | Effect |
|---|---|---|
| DisableNavigation (0x820cfe24) | update 0x826ece68, end 0x826ece80 | clears `Pedestrian`+0x1a44 bit 0x80 every update, sets it on end |
| LockToCurrentPosition (0x820cfe9c) | begin 0x826ece98, end 0x826ecf18 | locks the spatial adjust owner at `GetPosition`; end only unlocks |
| ProTrackSkater (0x820cfbcc) | ctor 0x826eca58, begin 0x826ecac8, update 0x82717548, end 0x826ec2c0 | `RecTimer` (default 8, data 16); begin pushes `SkaterCenterOfMass` head tracking and disables nav modifier 2 (skater avoidance); update sets audio state 32 and, when the channel request has finished, `ProRecTimer` (timer 19) is not positive and `mbProChallengeStarted` (+0xca9 bit 0x40) is clear, remaps `GreetChannel` (`RemapAnim` 0x82e8d8e0), plays it (`PlayChannelAnim` 0x82e93f20, blends 0.2, not cycling), sets audio state 28 and `ProRecTimer` = `RecTimer`; end pops the head tracking |
| ProWaitingForSkater (0x820cfc08) | begin 0x826ecb50 | skater avoidance off, `Pedestrian::ClearAllIntents` 0x82e8d890 |
| IgnoreCollision (0x820cfcf8) | begin 0x826ecd20, end 0x826ecd38 | +0xca9 bit 0x08 |
| PhotographSkater (0x820cf0bc) | begin 0x82707768, update 0x82717018 | begin: ClearAllIntents, `SuggestNextIntent("PhotographSkater")`; update: audio state 29, `BearingToTarget` (`GetBearingToTarget` 0x82e5acc0 of the target transform) and `PitchToTarget` (`GetPitchToTarget` 0x826fd8b0 from 0.9 above the position) of the local skater's centre of mass, written pitch first through `AddSettableAtt` 0x82d7f5f0 (not normalized, sequence -1) |
| ReviewingPictures (0x820cf0f8) | create 0x82711ec0, begin 0x82707830, update 0x827078f8 | instance timer 30; begin: ClearAllIntents, `SuggestNextIntent("ReviewRecent")`; update: timer -= dt, clears +0xca8 bit 0x20 once the timer is not non-negative |

`PedestrianColliding` (ctor 0x826eabe8: `warnChance` 0.7, `broadcastradius` 20) and
`SendSpeechEvent` (ctor 0x826ed130: `PictureTaking`, 29) reuse the `Pedestrian.xml`
implementations.

Conditions:

- IsFacingSkater: ctor 0x826efab0 (`FOVAngle` default 0.8, `Distance` 0; data 1.4 and 6.0),
  `ActivationFunc` 0x826ffea8. Skater centre and entity position flattened to y 0; with a non-zero
  distance the guarded length must be below it; then the entity facing (transform row 2) dotted
  with `Math::NormalizeSafe` 0x824b18e0 of the offset must exceed `XMVectorCos` 0x824b0130 of the
  angle. The skater is `ISkaterManager::GetLocal`'s `PhysOut_SystemReckoning::mCentreOfMass`.
- IsPhotographerTakingPictures 0x82707a10: +0xca8 bit 0x40.
- IsPhotographerReviewingPictures 0x82707aa8: +0xca8 bit 0x20.

## 4. Motion graphs and clips

| Motion state | Clip (`anim`, blend) | Entered by |
|---|---|---|
| Idle / IdleCyc | `IdleBasicCyc`, 0.1 | default |
| Collision.Standing.Colliding.From* | `Collision{Fwd,Back,Left,Right}Standing`, 0.3 | HasIntent Collision + direction |
| StandingCollisionComplete | none (`MajorIntentComplete`, speech `PedestrianWandering`) | WillExpire (0.01) |
| TrackTargetFromIdle | `PHOT_SHOOT_FROMSTAND_N_0_INTO`, 1.0 | HasIntent PhotographSkater, LastState Photographer.Idle |
| TrackTargetCyc | `PHOTO_SHOOT_BLEND`, 0.1 | WillExpire |
| IdleFromTracking | `PHOT_SHOOT_FROMSTAND_N_0_OUT`, 1.0 | LastState Photographer.TrackTarget |
| IdleReview | `PHOT_SHOOT_REVIEW_N_0_N`, 0.1 | HasIntent Review |

Every name goes through the entity's animation data first (`RemapAnim`); the photographers'
`Collision*Standing` remap to `PHOT_HRL_STND_*`, their `GreetChannel` to `IP_GRTCHN_POSLH_LO_0_01..05`.

Data facts:

- `PHOT_SHOOT_REVIEW_N_0_N` is not in `PedestrianSkeletonPres.abin`, and IdleReview is not
  reachable: `ReviewingPictures` suggests `ReviewRecent` while the motion graph tests `Review`, and
  no native store sets +0xca8 bit 0x20 (its writers `RunMainActivity` 0x825a5998,
  `ResetAlternateLocationCharacters` 0x825a8b7c, `TestBBoxAgainstTree` 0x825ac204 and
  `ReviewingPictures::Update` 0x827079f4 all clear it; `tPedestrianStateStorage::Reset` 0x82e927fc
  zeroes it), so CheckOutPictures never runs in the shipped game.
- +0xca8 bit 0x40 is set by `TestBBoxAgainstTree` 0x825ac1f0 (a spawn during the character's own
  challenge) and `RunMainActivity` 0x825a5984, cleared by `ResetAlternateLocationCharacters`
  0x825a8b68; +0xca9 bit 0x40 is set by `TestBBoxAgainstTree` 0x825ac1e0 and
  `ForceCullChallengeSpawnedLivingWorld` 0x825a87a8. The port's `LivingWorldOp::StateFlags` and
  the spawn's `challenge_flags` write the same bits.

## 5. What runs live

Early-career entries; positions from `BAM_Sim.loc`:

| Character | Challenge entry (on start) | Locator | Position |
|---|---|---|---|
| Slappy | m_1a_01_goto_slappys (Keep) | m_1a_01_goto_slappys_challengelocator01 | (1650.31, 100.61, -519.40) |
| Giovanni Reda | m_1a_05/06/07/08 tutorials (Remove) | m_1a_03_tutorial_fliptricks_challengelocator01 | (1640.41, 100.56, -522.35) |
| Atiba Jefferson | m_1a_05_photo_rail (AlternateLocation) | m_1a_05_photo_rail_challengelocator01 | (1671.65, 102.44, -520.98) |
| Atiba Jefferson | m_1a_06_photo_puzzle (AlternateLocation) | m_1a_06_photo_puzzle_challengelocator01 | (1633.17, 100.82, -531.83) |
| Slappy | m_1a_07_follow_slappy (Remove) | m_1a_07_follow_slappy_challengelocator01 | (1597.85, 100.54, -509.71) |
| Mike | m_1b_05_goto_mikey (Keep) | m_1b_05_goto_mikey_challengelocator01 | — |

- A spawned character stands locked at its locator: AI HoldAtSpawn (DisableNavigation,
  LockToCurrentPosition, ProWaitingForSkater), motion Idle on the remapped `IdleBasicCyc`.
- When the local skater is within 6 m on the ground plane and within 1.4 rad (80.2 degrees) of the
  character's facing, the AI enters TrackingSkater: the head tracks the skater's centre of mass;
  while that character's challenge has not started, a `GreetChannel` gesture plays as soon as the
  channel is idle and then at most every 16 s. Leaving the cone returns to HoldAtSpawn.
- A photographer spawned during his own challenge (the alternate start locator) has
  +0xca8 bit 0x40 set: PictureTaking runs `PhotographSkater` (motion
  TrackTargetFromIdle → TrackTargetCyc on `PHOTO_SHOOT_BLEND` driven by `BearingToTarget` and
  `PitchToTarget`), with `IgnoreCollision` and the `PictureTaking` speech event.
  `CompleteChallenge` (`ResetAlternateLocationCharacters`) culls him there; he spawns again at his
  challenge locator in HoldAtSpawn.
- A collision runs Colliding (PedestrianColliding, motion Collision.Standing) until
  IsPedestrianColliding clears, then re-enters Marquee/Photographer.

## 6. Port structure

- `AiDelegate` (`pedestrian_graph.rs`) selects declarations, expressions and capacities;
  `PedestrianGraph::update_delegate` runs `update` for `Pedestrian.xml` and
  `update_declarations::<3>` for the two marquee delegates.
- `MarqueeBehaviour` holds the new behaviours; callbacks that need the frame
  (`ProTrackSkater`/`PhotographSkater` update, `ReviewingPictures` update with the controller's
  delta time) go through `FrameServices` (`pedestrian_graph_frame_callbacks.rs`).
- `Condition::FacingSkater` / `PhotographerTakingPictures` / `PhotographerReviewingPictures`
  (`pedestrian_graph_condition_binding.rs`); the skater centre comes from
  `ConditionServices::local_skater_centre_of_mass`.
- `MotionDelegate` / `MotionDeclarations` (`pedestrian_motion_delegate.rs`); only
  `MotionGraph_Pedestrian` uses the per-frame condition cache (`ConditionResults`), as native.
- `Pedestrian::ClearAllIntents` 0x82e8d890 and `SuggestNextIntent(const char*)` 0x82e8d830 are
  `PedestrianConstruction::clear_all_intents` / `suggest_next_intent` (`pedestrian_entity_intent.rs`).

## 7. A schema a skater holds

Native shares one recipe per schema between a skater and the living world. The port's recipe
registry holds one entry per schema, so `skater_holds_marquee` refuses the living-world load while
any skater (local, party or AI) holds that schema; the entry asks again on a later update. For
m_1a_07 (Slappy as the follow AI skater, his challenge character removed on start)
`ForceCullChallengeSpawnedLivingWorld` releases the character (cull, then `UnloadModel`) inside the
career update, the unload completes in the next tick's `RecipeMan` update, and the AI skater spawns
in the following frame's AI dispatch: no conflict. Where a skater already holds a pro's schema
when the character's box is entered, native shows both; the port shows the character only after
the skater releases it.

## 8. Deviations

- Conditions: native builds `HCDelegate_*::sConditions` once per delegate (`sInit`); the port builds
  them per graph load. They hold only `FOVAngle`/`Distance`.
- `IntentGen`+0x834 bit 0x40 (set at 0x82e93d34 in `IntentGen::SuggestNextIntent`, cleared at
  0x82e8b10c in `IntentGen::ClearAllIntents` 0x82e8b028 and at 0x82e93ae0 in `IntentGen::Update`
  0x82e937b0 once the packet list is empty) is not a separate field: the port's `IntentQueue` reads
  its queue (pre-existing pedestrian model).
- Section 7 (schema sharing).

## 9. Not ported / left

- `CharacterFaceManager::LoadFaceData` for marquee recipes (`CharacterRecipeLoader::OnComplete`
  0x82d0a140) is not wired; the face manager is not live for any recipe.
- `PHOTO_SHOOT_BLEND`'s parametric blend on `BearingToTarget`/`PitchToTarget` uses the shared
  settable-attribute path; the pedestrian tree evaluation of that controller has not been seen
  live.

## 10. Tests (not run)

`living_world/pedestrians/pedestrian_graph_marquee_tests.rs`: marquee and photographer AI
selection sequences, state updates against active behaviours, binders, the two motion graphs'
selection, `is_facing_skater` and `pitch_to_target`; `#[ignore]`
`challenge_characters_name_the_marquee_graphs_and_their_clips` (`SKATE2_GAME_ROOT`): entity graph
paths → delegates 16..19, resources, and every played clip present in
`PedestrianSkeletonPres.abin` except `PHOT_SHOOT_REVIEW_N_0_N`.

## 11. Unmatched clip parts (first live run)

The first live run stopped with `clip fetch: NoMatchedParts` a few frames after Atiba Jefferson
(0x88d72df443d56751) spawned. The fetch is the marquee hand system's right hand:
`InitHandAnimSys` 0x8280d5d0 builds one hand ACS on the left-hand poses hierarchy
(`SkaterLeftHand.abin`, part `LeftHand`), and `UpdateHands` 0x8280e0a0 evaluates the right-hand
clip (`SkaterRightHand.abin` `poses`, part `RightHand`) on it. `Andale::FetchSys::BatchFetch`
0x82d737d0 matches parts by their `UpdatePartNames` (0x82d6e728) index; with no match it writes
one null-source entry (0x82d73e78..0x82d73ed8) and returns 1, and `Vector_ExtractPackedN`
(0x82d7116c) fills the hierarchy with the identity. The port returned an error there; a
`ClipFetchPlan` with no parts now stands for that entry (`execute_fetch` already starts from the
identity) and `extract_count` counts it as one extract. Regular pedestrians load
`PedestrianLeftHandRegular.abin` for both hands and never take this path. Test (not run):
`the_marquee_right_hand_pose_fetches_the_null_source_entry`
(`session/living_world/selected_living_world_pedestrian_hands_face.rs`).
