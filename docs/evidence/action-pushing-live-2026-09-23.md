# Grounded push Action update from raw A input

An original-data public facade test booted a fresh selected skater, completed
32 neutral frames, then submitted a raw XInput A press. The first A frame
entered Action Pushing and stopped at `PendingStateUpdate(0x828ed6a0)`.

IDA identifies `0x828ed6a0` as a shared update thunk for HippyJumping state
47 and Pushing state 48. It calls the existing OnGround19 parent update, then
four leaf callbacks in packed positions 31 through 34: one
`CreateMGTimeIntentFromAGIntent` and three `CreateMGIntentFromAGIntent`.
The exact active behavior IDs are `0..=10, 42..=49, 68..=79` followed by
`120..=123` for HippyJumping or `124..=127` for Pushing. Rust now guards
both the state/address and complete ordered active list before dispatching
the 35 already-implemented callbacks.

The next replay advanced into Motion Push1028 and stopped on its `InitPush`
Create callback, occurrence 1932, at `0x82bb3da8` (see
`artifacts/public-facade-grounded-push-dispatch-2026-09-23.log`). IDA
confirmed its null instance, Begin reset of the **already retained**
`ActorSkaterGraphState.push` fields and continue-push flag, and no-op
Update/End. The Push1028 `0x828f2e60` update guard now covers its exact
31 ordered behaviors, ending with the native no-op callback for InitPush.

The mounted replay moved beyond InitPush to occurrence 2441,
`ComputeFirstPushStrength::CreateInstance` `0x82ca0c90`, inside the selected
push child (`artifacts/public-facade-grounded-push-after-init-2026-09-23.log`).
The independently reviewed real-instance callback uses the existing
Pushing attribute block, registered `kCapDvAtTopSpeed` CVar, physical board
speed, and retained PushState. The raw-A replay completes its Create and Begin,
then stops during the next behavior's Create, before this callback's live Update
math runs. That next callback is `SetPushCoefs::CreateInstance` occurrence
2442 at `0x82cba730` (`artifacts/public-facade-grounded-push-after-firststrength-2026-09-23.log`).
The route is not yet a completed push or proof of speed/animation parity.

The next source-backed additions reuse the existing Pushing Vault and
PushState for `SetPushCoefs`, and build the regular/mongo target-coefficient
blocks once from eight original push animation clips. Independent IDA review
corrected the target solver's speed-versus-strength input before lifecycle
integration. The mounted raw-A replay now passes Create/Begin for those
behaviors and stops at its selected Motion state update `0x828f4ed8`
(`artifacts/public-facade-grounded-push-after-targetcoefs-2026-09-23.log`).
This still does not prove that their Update math has run in the live frame.

The shared `StateUpdate_Default_1338` thunk `0x828f4ed8` has now been
independently checked against IDA and the generated hierarchy. Right leaf
1338 and Left leaf 1340 each have 35 active behaviors; both call the Push1028
ancestor once and then FirstPushStrength, SetPushCoefs, target coefs, and
PlayAnimation in that order. The selected right leaf's tail IDs are
2441..2444 and the left leaf's are 2448..2451. Exact state/address/active-set
guards invoke each callback once. The raw-A replay completed step32 with
Action48, Motion1338, NewPush and RightPush both 1, and both reported Motion
Pushing values present (`artifacts/public-facade-grounded-push-after-defaultupdate-2026-09-23.log`).
This now proves those Update callbacks execute on that frame, but not native
numeric parity or sustained push speed. Step33 first stopped at expression438;
IDA `0x8291ce0c` showed the lazy OR of DisablePushBrake18, GroundBSGrab288,
GroundFSGrab343, GroundDBLGrab287, Coffin225. All five condition evaluators
already existed in Rust. Adding only the ordered expression let step33 advance
to expression357 (`artifacts/public-facade-grounded-push-after-expr438-2026-09-23.log`).

IDA's Pre jump table gives expression357 `0x82916368` as lazy
`IsRidingGoofy218 && HasGestureIntent366`; sibling358 `0x82916434`
reads gesture first, then negates goofy. The shared native Motion condition
uses the same Action gesture-map owner, while retaining the Motion controller
context. Rust now validates that Motion context and calls the existing
Action-map lookup without requiring an Action controller token. The raw-A
replay advanced through these to Pre256 `0x82911568`, a direct
`WillExpire367`, and then Pre415 `0x8291a7f4`. The latter lazily reads
MongoPushFootToFar358, IsRidingGoofy360, CurrentState359/361 in the native
branch order. After adding those source expressions, step33 reaches the real
`MongoPushFootToFar` condition358 (`0x82caa870`), which is the current boundary
(`artifacts/public-facade-grounded-push-after-expr415-2026-09-23.log`). That
condition reads the animation tree's `push_contact` and foot pose through
several existing owners; no guessed result is substituted.

The contact was present in the original-data first push frame. The condition
now reads the retained tree attribute, mirror bit, foot/deck physical output,
and live threshold (see `mongo-push-foot-condition-2026-09-23.md`). Pre356
`0x82916198` negates that same condition before its shared stance/state tail;
Pre430 `0x8291bcf4` checks Dismount intent9 and negates four exclusion gates
10..13 in source order. Both use existing condition evaluators. The raw-A
replay now completes 52 selected visual frames through held A and release
(`artifacts/public-facade-grounded-push-after-pre430-2026-09-23.log`). It
still needs physical speed/displacement and exit-state validation.
