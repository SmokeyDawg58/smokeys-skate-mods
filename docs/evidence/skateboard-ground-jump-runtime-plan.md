# Ground-jump runtime integration plan

Source audit, 2026-09-08, original debug XEX/PDB; read-only implementation
planning. This packet does not authorize a fabricated inactive jump flag or
claim a complete GroundAnimation/Actor update. Broader DMO work stays parked.

## Native scope and ABI

| Function | Range | SHA-256 of original function bytes |
| --- | --- | --- |
| `Skateboard::Toolkit_CalcGroundJump` | `82b315a8..82b3212c` | `c8d44e0949e87773f2955c4d1a3c5c7864f1cdacea9b5d6375030a0d599f2c97` |
| `PhysState_GroundAnimation::UpdateSkateboard` | `82d8a7b0..82d8a9e0` | `960b4cf9fdfc2fdf4808f2c639acc7bbedc00a2af77aaec5265b415e51b18e5c` |

The jump calculation has a hidden return buffer: raw r3 becomes result r27;
r4 becomes actual Skateboard r28. Decompiler casts which show result fields
as board fields are wrong. PDB `SkateboardJumpInfo` has full Vector3 at +0,
COM-height float +10, bool +14. The only executable call to CalcGroundJump is
`82d8a7ec`; the outer board updater is called by GroundAnimation::Update at
`82d98db8`. Other xrefs listed by IDA are non-code metadata.

## Inactive decision is a real produced branch

Native order is cvar target load `315f8`, `kMaxJumpError` scalar load `31600`,
then result doJump=false `31604`, COM-height=+0 `31608`, full four-lane
newVelocity=+0 `3160c`. It next obtains actual board processed owner +110,
reads flags +8c4, and tests mask **00400000** (`31610..31620`). Missing cvar
authority must leave the old result completely intact. Missing processed
authority is encountered after those three clears, not in a preflight.

If the bit is absent, `3210c` repeats false and returns. No jump attribute
lookup, body jump-DV mutation, or fabricated prepared-velocity sampling is
needed on this branch. Absence of an implemented flag producer is not proof
that the bit is false.

PDB names the bit `mStartJump`. Its concrete producer is
`Skeleton::ProcessAnimAttributes` `82b59de8` called by actual `ProcessData`
at `82b5ed44`. The raw attribute loop reads its five packed name words
(`a420..a43c`) and raw type byte. The scalar branch requires **type 0**
(`a6f4/a6f8`); the contact type-3 activeflags&0x0c gate is a different branch
and must not be applied to this jump attribute. At `aEC8` a packed
`FastString<30>` comparison against `kJumpName` selects an OR of 00400000
into the existing processed flags (`aed8/aedc/aee0`). The jump branch does
not decode a scalar payload or require its numeric value to be nonzero.
The original dynamic initializer `82f7ca70..ca84` binds the literal **"jump"**
to kJumpName; static .bss bytes are zero before that constructor and must not
be mistaken for the initialized packed key.

The same loop's exact `PrepareJump` scalar-name branch ORs 00200000 at
`5ae2c`, then calls actual `mIActorQuery->GetInput`. If present, InputMap IDs
74 and 75 provide clamped retained Skeleton +3e34/+3e38 adjustment values.
No-input retains those older Skeleton values. The end-of-loop jump-strength
stage samples global `anim_motion_jumping` attributes only when startJump is
set, with later independent override and adjustment-policy reads
(`5b7ac..5b9f8`). Consequently a raw jump flag projection alone does not
produce all inputs needed by an active jump. Existing semantic
`ProcessedAnimationIntent`/`ProcessedPhysicsInput` can help name this data;
their authored scalar records are not evidence that actual raw dispatch ran.

## Concrete input owners and reusable current producers

| Calc input | Source | Current integration boundary |
| --- | --- | --- |
| jump normal +240 | retained board +c0, published by `PrepareToolkitInput` at `26de4` | Reuse `skeleton_processed_toolkit.rs::try_continue_toolkit_jump_normal` and the same `SkateboardForceRuntime::jump_normal`; already implemented with readiness and late cvars |
| body Z axis +160 | actual effective-transform toolkit axis, including source flipped-control scalar | Reuse existing toolkit direction producer; do not sample a new guessed board frame |
| current velocity +190 | HiLOD `e02638/e0263c`: selected prior PhysOut SkateboardMotion +50 `mDeckVelocity` | Extend actual selected-prior-output projection where missing; a live deck-body read is a different source time |
| COM +250 | HiLOD `e02940/e02944`: selected SystemReckoning +30 `mCentreOfMass` | Reuse retained selected PhysOut/system owner; no skeleton root-position substitute |
| ground position +1e0 | HiLOD retained ground-position +d0 copied at `e036a8` onward | Requires the actual retained HiLOD ground-position producer; wheel deck hit alone is not automatically this field |
| Reckoning up | actual board Reckoning pointer +104, vector +1b0, loaded in Calc around `31b74..31bb0` | Existing retained Reckoning owner supplies the value; retain full lanes |
| prepared jump velocity +2c0 | retained HiLOD +c0 copied at `e038c0/e038c4` | Missing retained-history bridge described below; do not use current velocity every frame |
| jumpStrength +950, X/Z +954/+958 | raw animation loop and late policy/input reads | Preserve loop order and retained Skeleton adjustment ownership |
| gravity +964, surfaceSpeed +96c | actual processed input publication | Bind the processed owner; no runtime tuning constants in their place |

Actual HiLOD ordering includes Reset (`e0200c`), ProcessAnimOut (`e02134`),
selected prior-output copying, PrepareToolkitInput (`e036cc`), skeleton line
results (`e036d4`), ProcessData (`e03718`), then prepared-jump history and
publication. The retained prepared velocity is not part of a fresh default
input struct. After ProcessData, prepareJump mask00200000 chooses a full copy
of processed current velocity into HiLOD+c0 (`e037a0..e037b8`). Otherwise it
normalizes processed surface velocity +1a0 with a zero fallback, computes
the slope/gravity contribution using original +9.8 bits411ccccd and fixed
update-time bits3c888889 at `8236e734`, and FMA-updates the retained +c0
(`e037c0..e03880`). This history then publishes to processed+2c0 at `e038c4`.
The VMX contribution is surfaceDirection * (-surfaceDirection.Y * 9.8),
multiplied by that fixed timestep and added to retained prepared velocity;
keep full-vector operation order, estimate normalization and unknown W.
Construction/reset of this particular retained HiLOD history still needs a
focused source read before creating its runtime owner. Zero is not assumed.

## Attribute reads and exact publication prefixes

All Calc jump curves and dynamic lookups use the **global +d0
physics_jump_default instance**, not Skateboard own physicsdeck +534. Each
source singleton/instance lookup must stay at its original stage; existing
Vault instance/layout decoding can be reused. Missing dynamic scalars use
DefaultDataArea(4), missing bool uses DefaultDataArea(1); malformed present
data remains an error. Native dynamic lookup index is zero. An immutable
retained Vault provider can bind the occurrence once, with the same explicit
reload/snapshot lifetime boundary as existing providers, and issue staged
owner-bound layout/attribute tokens. Do not eagerly decode all nine values.

| Order/site | Read |
| --- | --- |
| `31634..31684` | Capture full jump normal, take Y, evaluate cached 8-point JumpYScalarVsGroundNormalY (layout+80) immediately; retain result for late compensation |
| `316a8..31748` | Prepared velocity then current velocity; project both tangents and compare squared error with already captured maxJumpError |
| correction only `3186c/31878` | cached 8-point JumpSpeedScalarVsAngle layout+d0; afterward prepared-tangent length times this scalar bounds current tangent |
| correction only `3191c` | first draw-jump cvar read and possible overlay/debug calls, before height reads |
| `319a0` | `97cdb6c313cd0a1b`, height reference speed; then processed speed load `319c4` and division/clamps |
| `31a10`, `31a54` | cached 8-point MinHeightVsSpeed layout+110 and MaxHeightVsSpeed layout+150 |
| `31a9c` | `a35ee33195004033`, shared low endpoint |
| `31af4` | `bb9e9bafba1b29e6`, minimum endpoint; compute minimum interpolation before next lookup |
| `31b54` | `0b2454c666410860`, maximum endpoint; then source COM/ground/up/jumpStrength/gravity reads and ballistic calculation |
| `31c80` | `c1a016ba4ca88480`, byte enables cached 16-point VerticalResponse layout+0, evaluated only when true (`31d5c`) |
| `31da0` | `8d134b7861d88c9a`, response floor |
| `31dd0`, `31dfc` | **doJump=true**, then **first full newVelocity** (tangent + normal launch term) |
| `31e40` | `be6c6a3ed7c6d148`, compensation cap; failure already leaves doJump=true and first velocity |
| `31f10` | **second full newVelocity**, after Y-only compensation |
| `31fb0` | `09a6c205c73dd180`, sideways scale; then processed X adjustment read and two scalar multiplies followed by negate |
| `32044` | `4eddbd6972ee9c56`, forward scale; processed Z adjustment multiplies in different source operand order |
| `32098` | **final full newVelocity**, after both adjustments |
| `3209c..320b8` | source global-singleton existence check, although no attribute value follows; preserve failure boundary |
| `320bc..320d0` | reacquire processed owner and current velocity; **board jumpDV+b0 = finalVelocity - freshly read currentVelocity**, full4 |
| `320d4..32104` | second, late draw-jump cvar target/value read; overlay absence fails after final result and jumpDV publication |

Fresh raw scan of all stores to result r27 confirms COM-height has exactly
one store, the entry +0 at `31608`; it is never replaced with the internally
calculated COM height. This resolves the older prefix packet's open question.
The caller's stack result and board-owned jumpDV are separate retained/error
prefixes: on a Calc error, do not copy an incomplete stack result into state,
but retain any already executed board jumpDV write.

The deferred jumpDV consumer is also concrete: Skateboard::FillPhysOut reads
the current startJump bit again at `82b2f8f4..f904`, selects the board+b0
vector when set, or a local full zero vector otherwise, then checks the
actual PhysOut write latch (`f91c..f938`) before publishing to selected
PhysOut_Air+70 `mJumpDV` (`f93c..f94c`). This does not clear retained board
jumpDV. Bind the existing selected PhysOut occurrence/write guard; do not
publish jumpDV eagerly from Calc or unconditionally from stale board state.

## Existing calculations need correction, not a duplicate solver

`physics.rs::PointGraph::evaluate` is already the shared source-backed
point-graph evaluator. The named `GroundJump*`/`JumpPhysicsCurves` structures
and normal-stage functions remain useful. However, current direct reuse is
blocked by two freshly identified discrepancies in
`calculate_ground_jump_tangential_velocity`:

1. Native correction is **ordered squaredError > maxJumpError**.
   `31748` vcmpgtfp followed by the summary-bit branch `31768` means NaN
   retains the prepared tangent. The current `<=` early return instead
   corrects on NaN.
2. Native maximum length is **length(preparedTangent) * curve(angle)**.
   `318a0` reloads prepared tangent from stack+80 (IDA var_150),
   `318a8..318f8` derives its
   length with two rsqrt refinements and exact-zero selection, `318fc`
   multiplies by the graph scalar, and `31904/31908` passes that bound to
   ClampVectorWithinMaxLength on current tangent at stack+a0 (IDA var_130).
   The helper currently passes
   only the curve scalar. This affects ordinary finite non-unit velocity.

Correct these in the existing helper with discriminating non-unit and NaN
tests when authorized, and extract shared staged primitives rather than
writing a second guessed jump solver. An eager whole-solution call cannot
preserve the three velocity publications or the fallible attribute order.
The shared numerical primitives also need an explicit full4/fallible-port
entry for native runtime use; preserve the existing XYZ API as a documented
compatibility wrapper if retained for external callers/tests.

Other numerical boundaries which a runtime port must preserve:

- Two scalar fsel clamps at `319cc..319d8` map NaN normalized speed to **1**,
  whereas Rust `.clamp(0,1)` retains NaN; nonpositive signed zero becomes +0.
- Remaining-height fsel `31bf4` selects +0 for negative or unordered height
  difference; ballistic multiplication is `(remaining * gravity) * -2`.
- Ground-angle scalar clamps `31d18..31d24` likewise select 1 on NaN after
  the VMX clamp/acos boundary. Do not assume Rust min/max/clamp operand rules
  reproduce VMX min/max, acos approximation, or exceptional signs.
- Response-floor fsel `31dbc/31dd8` picks the floor on unordered difference;
  compensation's two fsel sites `31ed8/31ee0` can select the cap on NaN.
- Tangent normalization, vector clamp, square roots, reciprocal of pi/2,
  cross products and final full4 FMAs must use the existing numeric boundary
  and retain source refinement count/order. Current XYZ host math is not a
  declaration of bitwise VMX equivalence.

## Actual GroundAnimation caller and minimal connection

UpdateSkateboard runs steering target+0 (`a7d8`), actual wheel-material setter
(`a7e0`), then CalcGroundJump (`a7ec`). Reuse those now-existing actual-owner
stages; preserve their failures before the jump call. Successful Calc copies
full velocity to state+20 (`a7f8`), COM-height to +30 (`a800`), then doJump
to +34 (`a80c`) before testing it. False calls the already integrated staged
six-force tail once, in original order. Never use false to bypass an unready
start-jump producer.

True calls actual Skateboard::SetVelocity at `a82c`; only afterward sets
state jumpedThisFrame+40 at `a83c`, then actual SetLinearDrag(+0) at `a840`.
It constructs a local TrajectoryLaunchInfo (`a848`), lets the SAME Skeleton
populate it (`a854`), stores playerJumped=true (`a860`) then copies full
computed start velocity (`a86c`), calls the SAME selector Launch (`a870`),
then Update (`a878`, returned status ignored), then copies full selector+600
to state jumpVelocity+50 (`a888/a88c`). No six-force callback runs on success.
Failures preserve exactly this prefix; for example SetVelocity failure must
not set jumpedThisFrame, while Launch failure retains the earlier body writes.

SetVelocity is not a deferred force record: Skateboard::SetVelocity
`82b29ee0` delegates to actual SkateboardBody::SetVelocity `82b244d8`.
The latter iterates actual Assembly parts in ascending order, reloads the
incoming full vector for each part (`244f4`), obtains that Part's body+4c,
loads body velocity+20 (`2450c`), inserts its existing W with vrlimi mask1
(`24510`), then stores the vector (`24514`). It rereads assembly/count before
the loop continuation (`24518..24524`). A host failure preserves earlier
part writes. Reuse the real body pool's XYZ velocity writer only if it keeps
the existing native lane corresponding to W; do not replace it with a
seven-part fabricated full vector or an AddForce request.

Existing `GroundAnimationState::begin_update` correctly models the earlier
outer Update clear. `prepare_ground_animation_jump` is a semantic convenience,
not this runtime orchestrator: it receives Skeleton launch data too early,
sets jumpedThisFrame before SetVelocity, and manufactures compatibility W=+0.
`finish_trajectory_update` also keeps only XYZ. Retain these documented APIs
or evolve them around shared types, but do not use them to hide missing
full-vector producer/consumer authority. Existing PhysicsAir trajectory
collaborator contracts and TrajectoryLaunchInfo layout are reusable evidence;
they do not constitute an actual bound GroundAnimation selector instance.

Smallest honest implementation sequence:

1. Extend the existing processed/raw-animation owner with exact scalar-type
   jump/prepare dispatch, staged strength/adjustment inputs and readiness;
   retain actual HiLOD prepared-velocity history with proven initialization.
2. Bind missing selected PhysOut motion/COM/ground producers and reuse the
   already implemented toolkit jump-normal/axis tail on the same owner.
3. Correct/refactor existing jump calculations into source-ordered shared
   primitives, then bind global jump attributes and result/jumpDV publication.
4. Add the actual staged GroundAnimation caller joining current steering,
   wheel material and six-force stages with real SetVelocity, Skeleton launch
   info and retained selector operations. Explicitly refuse missing active
   collaborators at their first source use; do not manufacture launch records.

Validation should cover cvar-before-clears; real raw inactive and active
attributes; non-unit prepared-tangent scaling; NaN error/speed gates; each
velocity publication and late jumpDV/debug refusal; active SetVelocity failure
before jumpedThisFrame; and an original-data actual jump-to-selector-to-next
physics consumer. This packet makes no code changes or Cargo claim.
