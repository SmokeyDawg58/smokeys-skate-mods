# Selected complete Skeleton post-physics caller draft

2026-09-12. New-only drafts, no Cargo or shared registration. Root owns source
review and gates. Exact paths were checked absent before creation. This joins
existing kernels; it is not a second collision engine or a full HiLOD.Update4.

## Entry and integration interfaces

`physical_player_skeleton_postphysics_live_pool.rs` registers as child
`skeleton_members::postphysics_live`; parent skater_pool reexports that module.
It can therefore access SAME candidate.construction/body owner without exposing
private fields broadly. `physical_player_skeleton_postphysics_live_world.rs`
registers under the existing constraint session. Its entry is:

```rust
world.try_update_skeleton_postphysics_live(
    board, skeleton, references, actual_partial_ragdoll_argument, numeric,
)
```

The boolean is the native caller's already-evaluated argument r30. The world
reads actual Processed dt93c, matching HiLOD1d34 before Skeleton1d4c. Computing
the surrounding board/deck-in-hand request and the remaining HiLOD state,
wipeout and post-wipeout phases is outside this selected Skeleton method.
Do not supply a fake result and call the full HiLOD phase complete.

Required earlier amendments are in skeleton-postphysics-shared-amendments.patch.
The pool additionally needs this narrow construction borrow helper:

```rust
pub(crate) fn postphysics_inputs_mut(&mut self) -> Option<(
    &mut constructor::SkeletonOuterRemainder,
    &mut SkeletonBodyAttributes,
)> {
    Some((self.constructor.outer.as_mut()?, self.body_reader.as_mut()?))
}
```

## Source and host ordering

1. Existing physical-member/manager checks validate BOTH actual paired owners.
   Existing contact-spy current-frame and completed-feedback/list checks run
   even for empty contact vectors. No synthetic frame or contact list exists.
2. Pool verifies same actual constructor owner/processed binding and actual
   initialized history. Existing one-attempt marker begins before preparation
   writes, a checked host policy. Later failures retain writes and cannot replay.
3. Actual preparation: body-local attribute/current bodies IsBadData; taken
   ResetSkeleton/IK-reset/respawn; fresh board error; live draw cvar. Then the
   existing history kernel executes once, returning its captured board P.
4. COM-local VMADDccbc/ccc0/ccc4 storesccc8. Deck-local VMADDccdc/cce0/cce4
   storescce8. This is the same captured cc4c board P, never a fresh query.
   Root independently confirmed these addresses; older ccfc/cd04 commentary was
   corrected (those are restore/return). Body.GetPosition in preparation is
   actually57684;5767c only sets up its result argument.
5. Complete history, capture system velocity576dc..76f8, then actual world
   GetCollisions5770c. Native GetRWSimulation57728 resolves the already-verified
   same manager/backend used by existing live collision services, not a copied
   simulation. Query operations retain the actual list, including empty lists.
6. Late input function locks actual Reckoning then its Processed owner, matching
   existing lock order. It does not resolve the optional constructed Reckoning
   payload until the ground-normal source read inside input assembly. This
   preserves earlier missing-grab/ground/flags errors. Only copied source locals
   survive; no lock spans Collision.Update or post-record service callbacks.
7. Existing pool normal collision kernel and its owner/current-frame checks run.
   Existing volume-state/record scheduling follows; no parallel implementation.

One additional source-order correction was required for live integration.
UpdateRecords54118 first CalculateErrors, copies old COM, reads numeric estimate,
publishes current COM and velocity, THEN reads Processed flags8c8 bit1000. The
old API accepted a controlled captured word. New world passes a lazy callback
over SAME Processed Arc at this exact kernel read. Missing flags therefore keep
the preceding error/COM/velocity writes. It must not read flags before volume
states or numeric work. Root requested callback over a new runtime mode enum.
Exact shared amendment is skeleton-postphysics-late-flags.patch: generic flag
payload in the borrowed input, one kernel, legacy wrappers capturing known flags.

## Original completed-frame corpus placement

`physical_player_skeleton_postphysics_live_corpus.rs` is a new child helper for
the existing `physical_player_collision_corpus.rs` original fixture. Reuse its
setup through actual solved frame/contact feedback and
`post_records.prepare` at approximately330. In a dedicated fixture invocation,
replace the FIRST old controlled normal-chain call with the new helper, passing
the retained original cvar references and actual solved `frame` Arc. Do not call
both on one frame: history is intentionally one-attempt. Do not silently alter
the old force/priority oracles to match the new live inputs, which differ from
the older controlled danger-zone/system-velocity parameters.

The helper verifies same solved-frame identities, completed collision, both
specific-region current/recent bytes known, then same-frame replay refusal and
unchanged retained history/COM. It requires an actual admitted fixture; there
is no constructor-only fast path. Root should split the existing fixture's
setup/teardown or use a selected mode before the old controlled assertions,
preserving its original invocation. Do not reload another diagnostic asset
format, invent a pose, or seed contact bytes for this test.

After this actual producer succeeds, the same completed-frame occurrence can
feed mandatory Audio.Fill. The ordinary startup Update1-to-Update5 corpus still
skips solver and several Update4 predecessors; do not move the helper there and
pretend those phases executed.

Two kernel regression tests in skeleton_postphysics_late_flags_tests.rs reuse
the existing post-record test fixture: failure at late flags retains exact
error/old/current COM/velocity but never enters hook services; earlier numeric
failure never calls flags. Root registers them as a child of the existing test
module. No tests have been run by this agent.

## Explicit limits

Actual bad-data Reset changes body frame identity. Existing later contact-list
freshness checks may reject pre-reset solved contacts. This host admission
limitation remains explicit; the draft does not forge a new frame or bypass
the checks. Selected normal successful frame remains the intended test domain.

Existing history's full-array/host canonical-W policy, unknown-Up-W projected
COM restriction, unsupported nonnull GrabSpline body targets, and enabled debug
reload/drawing branches remain as documented in the earlier packets. The draft
does not claim target-hardware numerical parity or complete gameplay scheduling.

2026-09-12 root accepted postphysics integration. Latest accepted Skeleton post-physics checkpoint: **workspace53 passes 4,046 unit tests and six doc tests; original47 passes all 17 core corpus tests; Clippy86, format40 and diff40 pass.** A dedicated original solved-frame instance now executes actual bad-data preparation, same-owner history/local COM and deck publication, late collision-input reads, collision/volume updates and post-records. Current and recent contact-region records are established. The actual Processed primary GrabSpline is constructed/reset in its own retained storage. Flags8c8 are read inside the existing post-record kernel after COM/velocity writes. Replay refusal preserves the same completed frame's history. The fixture retains controlled original-pose/Reckoning preparation and an explicit caller argument; full HiLOD scheduling and a playable frame remain pending.

Root fresh raw checked576dc..578b4 collision input/child order, Body.GetPosition82de7b20 full Part.GetTransform, post-record flagsite541b8/bc. Existing host numeric policy remains an explicit boundary; source final-negated reciprocal refinement identified for next integration.
