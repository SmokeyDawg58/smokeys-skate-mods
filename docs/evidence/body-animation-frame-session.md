# Actor body frame owner continuation

2026-09-12. Confidence **high** for the selected source-null-tree dispatch and
owner integration. Fresh IDA decompilation inspected the symbols below. No
hardware per-tick trace or full skating parity is claimed.

## Source and ownership

- `Sk8::Actor::DispatchBodyAnim` `825f1040`: Simulation command manager lookup,
  named Skater ACS at this player ID, BeginCommandSet, actual animation virtual
  evaluation, EndCommandSet. The misleading decompiler virtual label is not
  the semantic name of the implementation.
- `Andale::Animatable::EvalCurrAnim` `82d6e658`: playback ACS/fetch owners,
  virtual blend threshold before BeginCommands, current tree then null-only
  bind fallback, Eval, SQTtoLocal, local ExtractMatrix, in-place LocaltoGlobal,
  global ExtractMatrix, EndCommands. Local and global are distinct retained
  Initialize-time buffers; GetBindPose uses another allocation.
- `Andale::ChannelAnimatable::GetCurrAnim` `82c8a498`: actual root channel node
  if nonnull; otherwise GetBaseAnim. Unknown/active roots cannot fall back.
- `Sk8::SkaterManager::UpdatePhysics` `825ffd98`: for each retained Actor,
  resolve its named body ACS and Sync at `825ffea8`, then test update mask,
  then SetUpPhysics and Update with source float `0.016666668`. It does not
  synchronize inside Dispatch.

The new `actor_animation_body_session.rs` binds real startup command/job owners
and cloned references to the original shared runtime cvar targets. The job
owner is resolved before buffered locks, preventing recursive acquisition of
the startup ACS mutex. Dispatch samples the selected threshold each call.
Actor entry points require constructor publication and pointer identity of the
retained SkaterAnim UCOM occurrence. Sync additionally compares the registry
ACS against the initialized playback ACS before touching pending work.

## Previous mismatch and validation

Before this change, both outer dispatch/sync methods had no callers. The only
BodyAnimationDispatchSource was a test fixture; the ignored original-data test
called private queue_body_pose directly and bypassed Actor lookup and cvars.
The new actual Actor corpus continuation calls the production Actor methods,
checks both outputs remain unreadable before Sync, checks command order and
live threshold, and repeats the complete asynchronous command lifetime twice.
Between first Dispatch and Sync it temporarily binds another real player's ACS
under the same registry key: Sync must refuse without consuming this Actor's
pending work. The test restores the registry before normal Sync.

Author validation: owned Rust files formatted with rustfmt. Cargo and original
corpus execution are reserved for root integration; passing results are pending.

## Exact next consumer boundary

`Sk8::Actor::SetUpPhysics` `825ff6c0` invokes an animation virtual before Reset
`825ff740`, then animation GetPhysUpdateData `825ff75c`, Motion output
`825ff778`, intents assignment, Motion attributes then animation attributes,
Actor/AI/reset/settings fields and finally physical SetPhysUpdateData.

`Sk8::Skater::SkaterAnim::GetPhysUpdateData` `82c96d58` validates bone count,
reads the per-frame global/local buffers, then consumes trajectory foot-state
and locomotion state. Its locomotion path mutates root matrices before the
global/local MemCopy pair at `82c97590`/`82c975a4`. Stance, current and prior
locomotion flags, timers, Vault and cvar decisions follow. These producers are
not yet reconstructed as the live owner path. Consequently this continuation
does not fill AnimOutPhysIn using an isolated copy, and does not claim actual
SetUpPhysics, generated Motion execution, or a runnable skater.
