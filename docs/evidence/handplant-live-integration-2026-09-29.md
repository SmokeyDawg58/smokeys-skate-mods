# Handplant live integration — September 29

Priority: implement the handplant flow behind
`Before(Handplants(ActiveLaunchPrefixPending))`, checking the debug executable in
IDA and reusing existing implementations. Work was SOLO.
**These changes are UNCOMPILED and UNRUN.** Source review,
asset inspection, static graph audits and scoped rustfmt are not runtime proof.

## Existing work and actual missing flow

The project already had the Sim-owned HandPlantManager/position and trajectory
selector construction, reset, six retained prediction futures, SurfaceQuery,
animation output fill and IsHandPlanting conditions. Skeleton.UpdateHandPlant,
world hand IK, World.GrindData queries, prediction scheduling, Reckoning math,
generated graph declarations and callback owners were also present. The live
Ground launch path stopped at ActiveLaunchPrefixPending; state600 did not have
the complete selected entry/update/exit/publication route. Do not build another
manager, spline registry, prediction queue, scoring system or skeleton controller.

## Source checked and implementation locations

Addresses below are in the connected Skate 2 debug IDA database. Full-width
attribute keys and ambiguous VMX operands were checked in raw PPC where the
decompiler lost high words or misrepresented vector registers.

| Original source | Existing/new Rust integration |
| --- | --- |
| Ground.UpdateHandplants `82d8d258`; ShouldLaunch `82dbd158`; Launch `82dc4140` | `physical_player_ground_handplants.rs`, `physical_player_handplant_manager.rs`: committed direction, reset/eligibility, actual COM/velocity/look/normal, native query box and launch flags |
| Vertical plane `82db86a8`; transform segment `82db87c0`; clip Z/XY `82db89f8`/`82db8ba0`; SimilarDirection `82db8e50`; FindChain `82dbec08` | `physical_player_handplant_geometry.rs`: source filtering, clipping, chain traversal and tolerances |
| Viable extents `82dbddf0`; best position `82dc1ea0`; position Launch `82dc3bc8` | `physical_player_handplant_position.rs`: seven authored graphs, ballistic extents, 40 filtered segments, height/depth choice, coping chain and original primitive provenance |
| Matrix from heading/up `82db5878`; trajectory using apex `82db6a58`; trajectory starting at apex `82db6c60` | `physical_player_handplant_manager.rs`, `physical_player_handplant_predictions.rs`, existing numerical/trajectory services |
| Selector Launch `82dc26d8`; reflect normal `82db8ec8`; prediction `82dbf050`; selector Update `82dbf6a0`; SurfaceQuery Start/GetResult `82dc8668`/`82dccea8` | Same six futures and surface future, admitted to the existing predictor; no fabricated completion or duplicate query owner |
| StartHandPlant `82dc3af0`; State Enter `82d89330`; State Update `82d97408`; empty Exit/Post `82d893b0`/`82d893b8`; Fill `82d893c8` | `physical_player_handplant_state.rs` and existing selected physics lifecycle/publication files; state600 uses the real bound board and skeleton |
| EstimateTimeToApex `82db7b90`; AdvanceTime `82dbd910`; CalcValues `82dc1970`; Bezier `82db7388`; COM `82db76c8`; orientation `82dbd5a8` | Manager/values modules: graph-scaled parametric clock, unscaled real clock, landing selection, exit velocity bound, Bezier and SLerp through existing math |
| Reckoning.UpdateHandPlant `82de0248`; existing Skeleton.UpdateHandPlant `82b5da30` | `reckoning_air.rs`, retained Reckoning completion bridge and existing skeleton implementation |
| UpdateIK `82db7d30` | `physical_player_handplant_ik.rs` plus actual raw hand/shoulder reads and existing world-hand IK setter; reset timing, stance/hand selection, approach distance, blend and arm clamps |
| ShouldPlayHandPlantAnim `82ca7970`; HasNewHandPlantPos `82ca7c30`; antic normalization `82cb5148`; SetHandPlantAnticLength `82cb51d8`; InitHandPlantAnimInfo `82cb70e8` | `actor_motion_handplant.rs`: authored timings and actual playback tree lengths, stored on the existing actor |
| ScoringHandPlants.Create `82ccbe58`; SetTrickDataHandPlant `825fe3e0` | Shared directional grab-sector helper, with the handplant trick bit `0x01000000`; ordinary grabs retain `0x02000000` |
| Action HandPlant update `828ed318` | Existing generated Action runtime, all 26 ancestor/leaf behavior occurrences |

All twelve Motion shared update thunks for the 44 states under HandPlantAir20
and HandPlantGround717 were checked:
`828ee558`, `828eece8`, `828efca0`, `828efd28`, `828efd88`, `828efd90`,
`828f0ee8`, `828f0f38`, `828f0f98`, `828f0ff8`, `828f1ce8`, `828f2c18`.
`generated_motion_runtime_handplant_states.rs` maps their actual occurrence lists;
the existing lifecycle executes the same callback objects. It covers FS/BS,
anticipation, into/cycle/out, one-foot and dismount branches.

Precondition and Sustain cases were separately checked against the native switch.
The subtree references expressions
`11,39,125,126,127,128,129,130,131,132,133,225,301,302,303,351,352,403,404`
plus the sentinel. Transition sections refer through the generated INDICES table,
not directly into TRANSITIONS; do not repeat that indexing mistake. No new
transition hook implementation was needed for these 44 states. Reused filters,
HandBusy, EndGesture, DisallowPumping, PlayAnimation, attribute and scoring callbacks.

## Important parity details

- Original launch rejects inadequate speed, descending motion, insufficient
  slope and nearly horizontal support normals. Holding RB on flat ground should
  no longer fail merely because the active launch branch was unimplemented; it
  does not force a handplant without an eligible authored coping primitive.
- Original selection returns the first chosen primitive's provenance even if
  the final closest point lies on another segment in its connected chain.
- Surface query completion gates landing selection; six real prediction results
  are consumed in source order. Landing uses the first eligible steep normal or
  the smallest normal Y, with the original last-trajectory fallback.
- Real time advances by 1/60 independently of the trajectory timing graph.
- IK reads raw Part3/7 hands and Part6/10 shoulders. The existing world-hand IK
  setter performs its own second clamp using its animation-bone mapping; these
  two shoulder references must not be incorrectly merged.
- The manager survives state exit. Source HandPlant.Exit and PostPhysics are
  empty; the state600 contact/animation gate and common priority handling select
  Ground, Wipeout or Teleporting through the existing state changer.
- Existing manager debug cvar522 is reused. New targets936..956 use original
  defaults and labels checked at their static initializers. No guessed gameplay
  timing constants were substituted for Vault or animation data.
- Animation class `f190522e738ec5cb`, default `d7edbd362d7d2152`, was verified at
  GlobalAttributeCollection constructor `828b32a0..32b8` and added to the SAME
  selected Vault class list. Jan17 shipped Vault contains antic
  `673f3b1bd056c39e` = 0.75, into `512342fb1d4a7d57` = approximately0.53 and out
  `b9012b86eb5258fe` = approximately0.1. Code reads these values at runtime.
- Shared animation lengths come from the original late-antic, antic and into
  animation trees in the existing playback database; no guessed clip durations.

## Validation and remaining limits

Read-only Python audits confirmed all 44 state/address/ancestor-slot mappings
against the generated declarations, Action37's 26 slots/address, 957 cvar
definitions and a complete unique registration permutation. All 21 new cvar
section/label pairs have no duplicate definitions. Scoped Rust formatting/parser
checks passed. Geometry clipping/chain, ballistic apex and graph-owner regression
tests were added but **NOT RUN**; existing regression tests were also not run.

No Cargo command, compiler, game window or scripted input test was run. Type,
borrow, startup and runtime correctness remain to be checked by a build
and controller test. Existing host numerical estimate policy remains in use;
this is not a claim of bit-identical Xenon arithmetic. Optional position/trajectory
debug visualization is not implemented as native debug geometry; gameplay and
existing overlay ownership checks are separate from that diagnostic presentation.

Next: run `build-release.bat`, then `play-quiet.bat`. First verify ordinary
startup/RB on flat ground, then approach eligible coping with RB, try FS/BS plants,
one-foot/dismount and release/landing. Capture any exact error and describe the
approach/stance if pose, attachment or exit differs. Do not suppress a newly
reached pending branch or invent a successful prediction to make the test pass.

Previous inactive-only handplant notes are superseded by this implementation
inventory, but their historical successful tests do not validate this new work.
