# Held frontside grab, 2026-09-24

**Audit correction, later September24:** the historical HandBusy owner
interpretation below is disproved. Decompiler `GetCurrAnim` names were attached
to the wrong virtual interface. Actual ISkaterMotionGraph vtable82069688+0x30
resolves GetGrabStateWrite825fdfb8, returning Motion+0x16d0. PDB identifies
signed counters mBsHandBusy+0 and mFsHandBusy+4; constructor825fde44/4c zeros
both. The existing `ActorSkaterGraphState.grab_busy` is the correct owner.
The animation-word implementation and its passing test are invalid evidence
and are being removed in the shared FS/BS correction. EndGesture's absent
IChannelAnimatable branch also needs the native no-op. See the current
BSGrab proof and integration status; the following text is historical.

The ignored original-data held-LT corpus replays the authored Ollie stick
points, then holds raw XInput LT=128 from step 27. At step 27, the retained
Action graph reaches LeftGrab138, Motion reaches FSGrab Into647, and the
original `FSGrab` intent and grab animation are requested. With the new
source-reviewed callbacks mounted, frames 27 through 40 complete. At step 41,
source-reviewed transition predicates 35 and 427 select FSGrab Cyc648. The
source-reviewed `SetGrabType` Begin callback 1262 writes FS=1 into the
retained SkaterAnim grab-type word. FSGrab Cyc's two
`FilterMotionGraphIntent` occurrences 1263/1264 now reuse the existing
class implementation for the authored `TweakX`/`TweakY` paths. Each has its
own instance; their source Update order is 1263 then 1264. `TweakProject`
1266 now reads that existing filtered-intent map and writes `tweak_x` then
`tweak_y` to the retained animation attributes. It uses the source L1
projection arithmetic and writes neither attribute when both intents are
absent. Independent IDA postreview caught and corrected the PPC unordered
comparison case: raw branch `0x40990010` at `0x82cc5438` skips scaling when
CR6.GT is clear, including NaN. The focused projection test covers presence,
sign, L1 scaling, and this unordered branch. The replay advances past 1266
and then reaches `ScoringGrabs` Update1267. The narrowly mounted FSGrab
occurrence reads filtered `TweakX` then `TweakY`, selects the authored
direction in Up/Left/Down/Right order when its vector length exceeds 0.5,
and writes the existing Motion trick name/bits/tweak payload for the physics
transfer. Its next observed boundary was the non-optimized BlendSpace
parameterization in animation phase 2. A temporary test-only diagnostic identified the
reached node as `blend_fs_tweak2`, set 0, two dimensions, with parameter
attributes `[0.0, 0.0, 1.0]`; that diagnostic was removed after capture.
IDA routes this case through generic `BlendSpace::UpdateParametrisation`
`0x82d76a08`. Its separate live `kUseSlicingProjectionN` target was added
from initializer `0x82f8de10` and table `0x82fd5624` at a new stable Rust
index 389, inserted between Jobs and Optimizations in the actual registration
order. Independent source postreview and focused link/INI/relink tests pass.
The generic two-dimensional kernel now reuses the retained serialized volumes,
coefficient vectors, and child-timing owner; it does not submit a 3D job.
Independent IDA review corrected PPC branch handling for unordered values,
and the pure kernel tests pass (3/3). The original-data held-LT replay now
completes frame 41 through this animation phase and stops at Motion expression
435 on frame 42. The full core suite passes (4,848 nonignored tests) and
`skate2-pc` compiles. Generic debug-overlay effects and the native invalid
nearest-index exceptional path remain conditional. The generic ScoringGrabs
polar and alternate direction configurations remain pending. The host VMX reduction/estimate
bits and InAngle boundary last bits are not yet Xenon-verified. Independent
postreview approved the reached FSGrab1267 route, while explicitly keeping
those numeric and alternate-configuration limits open. The core suite passes
after this mount (4,844 nonignored tests).

IDA evidence: FSGrab Into647 has behaviors 1258 HandBusy, 1259 EndGesture,
and 1260 PlayAnimation, in that order. HandBusy Begin `0x82c9e880` and End
`0x82c9e9c8` call MotionGraph GetCurrAnim and perform wrapping full-word
`+1`/`-1` at its `Animation+4`; Update is the shared BLR. GetCurrAnim
`0x82c8a498` selects the root channel's embedded node when a root exists.
The reached corpus has root channel 5. Native `PoolStackElement` identifies
only the high byte at +4 as `mPoolAllocated`; the constructors do not
initialize the whole word. Rust therefore records an unknown initial word
plus a wrapping delta on the *current* root channel, rather than inventing a
zero counter. The test observes `(root=5, initial=None, delta=1)` after
HandBusy Begin.

EndGesture Begin `0x82c9e768` calls `EndChannelPrematurely` on
`GestureRight` and then `GestureBoth`; End and Update are BLRs. The existing
animation channel-end implementation is reused. Independent postreview
checked exact names/order, the ChannelAnimatable interface, source callback
order, and absence of duplicate HandBusy writes. The state647 Update thunk
`0x828f1510` folds ancestor callbacks once, followed by behaviors
1258/1259/1260.

The step-41 transition first checks expression 35, `HasIntent(OB_Dismount)`
(condition 9). Its Sustain is literal true. Expression 427 then gates
FSGrab Cyc648 as `HasIntent(FSGrab) || CurrentState(FingerFlipVar.GrabOut) ||
(HasIntent(MuteGrab) && HasIntent(FingerFlipShuv))`, with lazy condition order
232, 235, 233, 234; Sustain is literal true. Both expression mounts passed
independent IDA review. The held-LT replay selects state 648 before stopping
through `SetGrabType`, with the filter occurrences mounted separately.

Conditional boundaries: if no root channel exists, native GetCurrAnim falls
back to the base animation. The Rust symbolic-word helper explicitly errors
there; the reached held-LT route has a root. Native EndGesture also returns
without effects when the ChannelAnimatable interface is absent, whereas the
typed Rust mount requires the published interface. No claim is made yet that grab
release, all button combinations, physical outcome, or original per-tick
numeric values are complete.

September 24 restart handoff: the original-data held-LT replay completed frame
41 and then advanced through frame-42 typed stops 435, 236, 235, 312, 140,
305, 150, and 136 as source-backed dispatch was mounted. The reached
expression136 is Pre `c(229)` (`HasTweak` with serialized `less=0.2`), while
Sustain is literal true. Its source agent identified the same HasTweak native
activation function used by already reviewed condition236. The current tree
has the narrow condition229/236 shared arm, expression136 Pre/Sustain arms,
and a focused expression test, but the PC restart interrupted validation before
this new edit was checked. On resume: run the focused test, await independent
IDA postreview, rerun the ignored original-data held-LT test, then update the
next typed boundary. Do not treat expression136 or frame42 as validated yet.
The earlier 150/151/311 expression postreview approved the branch arms and
reported that the FingerFlip states are FSGrab sibling candidates, not
Cyc648 transition targets. Recheck transition ownership through the generated
`INDICES` array before documenting further graph paths.
