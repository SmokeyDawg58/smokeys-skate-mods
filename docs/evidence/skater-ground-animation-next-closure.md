# GroundAnimation: next actual producer-to-consumer closure

2026-09-08 read-only audit after the selected Body/Up continuation work.
Existing production callers and prior evidence were inspected before fresh
debug-XEX raw reads. No implementation/Cargo; broader DMO remains parked.

## Recommendation

Close **actual retained Reckoning -> remaining surface-motion publication ->
same-pair prior-output copy** next, alongside the narrowly recovered animation
start/prepare-jump producer. This supplies real non-default motion to existing
brake/slide/up consumers and the upcoming prepared-jump history. Do not count
the recently added deck-motion publisher as all motion, and do not write a
second jump or six-force solver to hide missing inputs.

After those dependencies, connect the existing GroundAnimation steering,
material, jump calculation, real velocity/trajectory, and existing six-force
branch through one actual state updater. A hardcoded startJump=false would
exercise only a manufactured branch and leave the requested control closure
unimplemented.

## What the current production code establishes

- `physical_player_board_motion.rs` captures the actual Part6 linear velocity
  at the selected early FillPhysOut site82b2f550, seals it to actual body/pair/
  solved frame, and publishes via the existing PhysOut owner.
- `physical_player_phys_out_motion.rs` currently implements only angular
  velocity, deck velocity, and deck speed at82b2f714..f7f8. It does not write
  motion SurfaceVelocity+60, SurfaceSpeed+a4, or ZSpeed+a8.
- The PhysOut schema and `skeleton_processed_prior_output.rs` already represent
  all three missing fields and copy them in native Update1 order. Their prior
  corpus constructor-origin values must not be relabeled solved publication.
- `physical_player_board_up_input.rs` reads the late copied surface speed on
  the actual bound pair, so completing that producer improves an existing
  consumer rather than adding an isolated leaf.
- Actual steering/material/six-force/queue/Body/Up services now exist as
  separately scheduled retained-owner stages. The legacy `physics.rs`
  `update_ground_animation_non_jump` is a convenience calculation interface;
  do not route live execution through its independent legacy force queue when
  `skateboard_ground_force_tail` already uses the actual owner and preserves
  collision-response short circuit and force write order.
- `Reckoning::try_new` and `try_update_grounded` implement retained filters,
  attributes, spin and grounded math. The actual original corpus retains its
  Reckoning in `RootCorpus`/`PostRecordsCorpus`, a test driver. Current actual
  world root/GeneralUpdate entry points accept a borrowed Reckoning. Neither
  the board runtime nor physical-player owner currently owns a reconstructed
  native outer+104 Reckoning lifecycle. Reusing a corpus object is not that
  ownership integration.

## Remaining surface-motion source stages

Fresh raw FillPhysOut82b2fdf8..82b301a0 confirms:

1. Read the already-published SAME motion DeckVelocity+50, actual outer
   Reckoning+104 groundNormal+1f0, and current ProcessedPhysIn owner+110.
   Project the full deck velocity off that normal: dotfe2c, separately rounded
   normal*dotfe3c, subtractionfe40 -> local surface velocity (stack var_240).
   Do not substitute current body Y or the different processed ground+1d0.
2. Read current processed SkateTransformAnim Z at+60fe48, project it off the
   SAME captured Reckoning ground normalfe4c..58, NormalizeSafe with full-zero
   fallbackfe60. The signed-direction choice then tests dot of the unnormalized
   projected Z and surface velocity atfe94 using ordered GT. Unordered/zero
   selects the sign-XOR opposite atfec8/cc; do not replace with >=0.
3. Read actual Reckoning transform+20 atfefc..ff24; its Y is the wind axis.
   Call existing Math::AngleBetweenVectors graphff2c on chosen direction and
   surface velocity. Normalize angle via reciprocal2pi/floor/greater-half
   mask/subtract/multiply atff30..ff74. This runs BEFORE the surface-velocity
   publication. Its result publishes a distinct motion scalar+ac atffdc.
4. Write full motion SurfaceVelocity+60 atff9c, then scalar+ac atffdc, then
   length(surfaceVelocity) with original dot/estimate/two refinements/zero
   selectionffe8..30028; write SurfaceSpeed+a4 at3004c. Each source PhysOut
   latch sample/assert/store stage remains distinct. A failed angle/numeric
   stage must not prematurely publish surface velocity or speed.
5. There are intervening speed-wobble and tilt publications before signed
   Z speed. At300e8 get actual Body::GetPosition(6), which is the existing
   live Part getter (not the Body-only getter used by compression). Read
   current processed flipped flag00100000 at300ec..fc; if set, XOR full X
   and Z columns3011c..30. Dot previously published motion DeckVelocity+50
   with this Z3016c..74, then publish signed ZSpeed+a8 at301a0.

This is a usable narrow continuation over existing numeric/PhysOut services,
but it has explicit angle/speed-wobble/tilt gaps. Do not silently skip native
gaps while claiming a contiguous FillPhysOut. A selected stage can expose its
real preconditions and readiness, as the existing deck-motion sampler does.
Before implementation, independently audit raw aliases for sign selection,
angle arguments, floor/NaN behavior and each latch timing; this planning packet
does not claim those new full numerical leaves were independently reviewed.

The existing prior copier then supplies:

| Actual source output | Processed field | Native load/store |
|---|---|---|
| motion DeckVelocity+50 | velocity+190 | e02638/63c |
| motion SurfaceVelocity+60 | surfaceVelocity+1a0 | e02674/678 |
| motion ZSpeed+a8 | zSpeed+944 | e026dc/6e0 |
| motion SurfaceSpeed+a4 | surfaceSpeed+96c | e02710/714 |

The surface vector is required by HiLOD prepared-jump history; signed Z speed
is required by braking. Surface speed feeds jump curves, foot forces and the
new UpVector correction cap. None should be recomputed ad hoc from a newly
solved velocity at its consumer, because these are selected PRIOR-output copies.

## Reckoning and actual GroundAnimation order

Existing source packet `skateboard-ground-animation-prefix-source.md` establishes
GroundAnimation::Update82d98d68: clear jumpedThisFrame at8d80, actual
Reckoning::UpdateGroundStatesFixedHeading82ddd468 at8d94, then
Skeleton::UpdateAnimatedSkateboard82b5bc58 at8da4, set actual skeleton
add-board-error flag8db4, then UpdateSkateboard82d8a7b0 at8db8.

The existing `reckoning_grounded.rs::try_update_grounded` already performs the
grounded/fixed-heading mathematics. Retain ONE actual Reckoning owner at its
native outer constructor stage and adapt actual inputs into that method. Ground
normal passed by this caller is processed+1d0; dynamic-up input is processed
+210. Those caller fields, Reckoning's retained ground+1f0/up+1b0, and outer
UpdateUpVector.dynamic_up+70 are distinct stages/owners, not interchangeable
vectors. Heading, current animation, body spin and COM-to-deck producers must
be bound explicitly; no fresh Reckoning per frame or constant heading shortcut.

Animated-board update calls UpdateRootTransforms at5be90, derives/stores current
animation board and flag, ApplyBoardAnimation at5c378, then GeneralUpdate at
5c3a4. The existing IK Contact->DriveFrames->drives continuation belongs inside
that ONE GeneralUpdate call. Do not invoke an extra GeneralUpdate beside an
outer wrapper just to claim all components ran.

## Minimal genuine jump decision and launch continuation

`SkeletonProcessedContact::apply_raw_attribute` currently handles accepted raw
type3 contact bone records only. `ready(flags_8c4)` still receives an explicit
caller flag snapshot. Neither is a start-jump producer.

Existing jump proof establishes raw type0 exact packed name "jump" in actual
ProcessAnimAttributes82b59de8: match5aec8 -> OR00400000 at5aee0, with no scalar
payload-value/nonzero test and no type3 active-mask gate. PrepareJump name
sets00200000 and performs actual input-map74/75 reads and Skeleton adjustment
updates. Extend the same raw attribute traversal/state with these source cases;
do not dispatch semantic strings into invented flags or create a second loop.
The accepted animation producer/order must be retained even if a selected raw
record corpus is used before native Actor sampling exists.

Prepared velocity HiLOD+c0 is another retained owner: after ProcessData,
prepareJump true copies current processed velocity; otherwise NormalizeSafe
processed surfaceVelocity and integrate direction*(-direction.Y*9.8)*fixed
update time into old history at82e037c0..3880. Publish processed+2c0 at82e038c4.
Its constructor/reset initialization remains a
required focused source recovery, not guessed zero or current velocity every
frame. System COM/processed jump strength and adjustment provenance also remain.

UpdateSkateboard first calls existing steering(target+0)a7d8 and actual wheel
material applicationa7e0, then CalcGroundJumpa7ec. Preserve maxJumpError cvar
BEFORE three result clears and late actual startJump read. Reuse the corrected
existing jump math: the ordered-GT error gate and prepared-tangent-length times
curve correction have already landed (the older jump packet's proposed-fix
paragraph is historical). Remaining full4/fallible staged integration must
preserve velocity intermediate stores, Vault pointer/layout order, and native
fsel exceptional behavior; an eager whole-solution helper cannot do that alone.

True jump branch: copy jump result into SAME GroundAnimation state, actual
SetVelocity BEFORE jumpedThisFrame, then drag0, launch-info construction,
actual SkeletonGetLaunchInfo, playerJumped/start velocity, selector Launch,
selector Update, selected trajectory publication. Reuse existing launch/root/
trajectory owners where available; missing GetLaunchInfo/selector service is a
typed continuation boundary, not a default launch record. False branch invokes
the existing actual six-force tail, not another port of its formulas.

Board jumpDV+b0 is retained by Calc. Later FillPhysOut rereads current startJump
and publishes retained DV or full0 into actual Air+70 at2f8f4..2f94c with its
write latch. It neither clears DV nor replaces actual immediate SetVelocity.

## Next bounded work packages

1. Bind the actual outer Reckoning lifecycle and finish the selected surface
   motion publication using existing numeric/PhysOut owners; extend the current
   original corpus through prior-copy and existing brake/up consumers. Keep
   omitted Fill/Actor stages explicit.
2. Recover prepared-history initialization and extend the actual raw animation
   attribute/control producer with start/prepare-jump; retain flags/history on
   the same processed/pair owner. This removes the fabricated-decision risk.
3. Connect one retained GroundAnimation UpdateSkateboard around existing
   steering/material, staged corrected jump math and true-launch/false-force
   branches. Then integrate its earlier actual animation/Reckoning/GeneralUpdate
   order and native Actor/input scheduling. Completion of a diagnostic sequence
   must remain distinct from a playable live control loop.
