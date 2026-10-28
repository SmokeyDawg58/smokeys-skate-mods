# Skateboard spawn transform and attached Part update source

2026-09-07. Live IDA/PDB/PPC recovery followed by the implementation below.
Historical source notes remain; host validation is not a native numerical or
per-tick trace and does not recover the actual player spawn location.

Confidence is **exact** for the cited control, layouts, source constants and
recovered arithmetic graph; **high** for the bounded typed ownership mapping.
Existing host estimate/reduction and packed-native-W limitations remain explicit.

## Implemented live owner path

`skateboard_spawn.rs` now executes the complete ordered relocation graph through
a private actual-owner adapter in `dynamic_assembly_hook_pool.rs`. The managed
constraint session forwards the operation without exporting a mutable member
session or body capability. Current tolerance comes from an explicit service
read on EVERY call; assertion handling may stop via panic or return and continue
as after a resumed native assertion. No default epsilon or spawn location is
hardcoded in production. Missing hook membership produces a late typed host
error after the seven board writes instead of the native invalid dereference.

`skateboard_spawn_math.rs` implements distinct reverse classical Gram-Schmidt,
full-lane composition/rigid inverse and the quartic assertion predicate. Numeric
dot/estimate operations are explicit injectable boundaries; the concrete owner
uses pure infallible host FMAs and sqrt().recip() estimates. Separate projection
multiply/subtract and both refinement rounds remain. No generic orthogonality,
finite-input or handedness repair substitutes for the source behavior.

`dynamic_assembly_part_transform.rs` binds live getters and setters to actual
session Part ordinals, definition inverse metadata and privately held body
occurrences. Foreign authority/index/body failures precede the affected write.
Detached getters/setters copy all16 raw words without inverse/body math. Attached
setters apply existing inverse mapping, call the backend's narrow in-place
mutation and publish the ORIGINAL raw Part cache last. Live getter uses the
actual body rather than that cache, with the existing canonical-W/finite-XYZ
boundary explicitly retained.

`SimulationRigidBodyPool::try_set_part_body_transform_host` preserves allocation,
rings, states, motion, forces, RF and inertia owner. It rw-normalizes, writes
basis/COM XYZ, computes candidate PLUS normalized quaternion, and updates all
six tensor fields/mass ONLY for nonnull live body inertia. Immutable Arc ownership
and pure numeric helpers explicitly exclude callback-time inertia replacement;
this is not a claim to expose the source's internal capture/reload timing.
The required exclusive backend/pool borrow prevents mutation beneath a borrowed
solver phase. No removal/re-addition, reset, snapshot rehydration or fabricated
native metadata is involved. Existing helper production arithmetic is unchanged.

Independent source/code/test review passes. Root also reread all194 live outer
setter instructions in IDA. Twenty-three new tests:10 math,4 outer chronology,
4 real-pool write-set/failure and5 session cache/live/ownership tests. Tests cover
21 outer axis estimates, late hook failure, preceding-write-visible live reads,
current tolerance/resumed assertion, all6 tensors, null-inertia sentinel tensors,
nonzero RF/motion, retained owners, stale/free/foreign bodies and corrupt structure.
Shared-helper comparisons test wiring, not an independent native numeric oracle.

The mounted real-Vault board/hook run now performs two controlled relocations
after prior actual solver frames, when live deck pose demonstrably differs from
its cache. Both retain all non-pose/tensor body fields and unchanged Joint/Drive
snapshots, then execute another real C/J/D0/6/3 solve with finite motion and RF
clearing. Static hook remains unchanged during integration. Explicit cleanup
and retained owner lifetimes still pass. This is not an original spawn-location
trace, complete SkaterPool registration, authored world collision or native Tick.
Strict gate77951 exit0:2379unit+6doctests, fmt, warnings-denied all-targets Clippy
and all4core original-data probes. App ignored corpus not rerun. Earlier focused
gate77380 passed Clippy plus mounted combined-board relocation. Initial unit
gate36872 passed2151core tests; subsequent Clippy documentation/test-style
diagnostics were fixed before the final gate without production math changes.

## Connected outcome

`Sk8::Physics::SkateboardBody::SetTransform`, `0x82b34a90..0x82b34d98`, moves
the existing seven board Parts relative to the LIVE old deck transform and
then moves the separate hook Part with the ORIGINAL requested matrix. It does
not recreate bodies, reinitialize constraints, or reset their motion state.

The outer constructor calls this at `0x82b388b4`, after mandatory
CreateHookDrive (`0x82b388a8`). Thus the intended successful constructor caller
has seven admitted board bodies, the separate admitted hook body and the actual
retained Joint/Drive relationships. Creation-time local Part transforms are
not sufficient to model this operation after admission or after a solver frame.

The smallest connected implementation needs a checked in-place body pose/inertia
setter, owner-qualified live Part get/set access, and the distinct outer Math
normalization. Reusing AddRigidBody or cached snapshots would change source
behavior. The source-backed hook ownership prerequisite is retained separately
in [skateboard-hook-source.md](skateboard-hook-source.md).

## Reference functions

| Function | Reference address | Role |
| --- | --- | --- |
| SkateboardBody::SetTransform | `0x82b34a90..0x82b34d98` | Seven board updates, followed by hook update |
| Math::OrthoNormalizeRotation | `0x82b22c78..0x82b22d24` | Reverse-axis classical Gram-Schmidt; NOT rw OrthoNormalize3x3 |
| Math::IsOrthogonalRotation | `0x82b2f038..0x82b2f1b0` | Original assertion predicate with quartic error reduction |
| kOrthonalTolerance initializer | `0x82f7ba20..0x82f7ba80` | Default0.01, range0..1, current value sampled by setter |
| Part::SetTransform | `0x82b25480..0x82b25ba0` | Attached body pose/tensor update, then raw Part cache copy |
| Part::GetTransform | `0x826c3480..0x826c35a8` | Live-body-first transform selection |
| rw::math::vpu::OrthoNormalize3x3 | `0x82ae33b8..0x82ae3750` | Existing body normalization graph |

Relevant PDB names include
`?SetTransform@SkateboardBody@Physics@Sk8@@QAAXABVMatrix44Affine@vpu@math@rw@@@Z`,
`?OrthoNormalizeRotation@Math@@YA?AVMatrix44Affine@vpu@math@rw@@ABV2345@@Z`,
`?IsOrthogonalRotation@Math@@YA_NABVMatrix44Affine@vpu@math@rw@@ABVVecFloat@345@@Z`,
and `?kOrthonalTolerance@Physics@Sk8@@3VcVarFloatRef@@A` at `0x8308dab0`.
The exact source addresses and byte fingerprints below remain the automation
anchors where IDA displays demangled or hidden-return ABI aliases.

## Matrix notation and source composition graph

Matrices below are four column vectors X,Y,Z,P, each with four lanes. P denotes
the translation vector. Only XYZ coefficient lanes select linear combinations;
each resulting vector retains all FOUR output lanes. The reference stores and
composes those W lanes even where the safe host cannot represent native body
metadata as floating-point values.

For outer matrix A and inner matrix B, the recovered composition used in this
setter is:

```text
for j in X,Y,Z:
    result[j][lane] = fma(B[j].z, A.Z[lane],
                         fma(B[j].y, A.Y[lane],
                             B[j].x * A.X[lane]))

result.P[lane] = fma(B.P.z, A.Z[lane],
                     fma(B.P.y, A.Y[lane],
                         fma(B.P.x, A.X[lane], A.P[lane])))
```

This means X multiply then Y FMA then Z FMA for axes; translation begins with
an X FMA into the outer translation. Preserve the explicit grouping and argument
order. A generic affine helper that resets W to0/1 is not full raw-lane parity.
The formulas are readable scalar semantics, not permission to replace the
source graph with differently associated matrix library operations.

The rigid inverse used for a matrix D transposes only the XYZ basis, sets its
new three basis W lanes to positive zero, and computes its inverse translation:

```text
J.X = [D.X.x, D.Y.x, D.Z.x, +0]
J.Y = [D.X.y, D.Y.y, D.Z.y, +0]
J.Z = [D.X.z, D.Y.z, D.Z.z, +0]
negative_position = +0 - D.P                 // arithmetic subtraction
J.P[lane] = fma(negative_position.x, J.X[lane],
                fma(negative_position.y, J.Y[lane],
                    negative_position.z * J.Z[lane]))
```

The inverse translation is Z multiply -> Y FMA -> X FMA. This is a rigid
transpose inverse, not the general determinant/cofactor inverse used by
ClusteredMeshData::Reset. It does not normalize D before taking the transpose.

## Exact outer SetTransform chronology

1. Read current `kOrthonalTolerance.mTarget->mValue` at target+0xd8 and splat
   it (`0x82b34ac0..d0`). Call Math::IsOrthogonalRotation(requested,tolerance)
   at `0x82b34ad4`. False reaches FailAssert at `0x82b34af8`, source file
   physicsskateboardbody.cpp line1992, then DebugBreak at `0x82b34afc`.
   There is no return/fallback after a resumed assertion.
2. Read the LIVE deck Part6 transform D with Part::GetTransform at
   `0x82b34b10`. This is not its stored constructor transform or a detached
   copy. Build rigid inverse J using the transpose/translation graph above at
   `0x82b34b34..78`.
3. Compute N = Math::OrthoNormalizeRotation(ORIGINAL requested matrix) at
   `0x82b34b7c`.
4. Compute and retain L = N * J using the full four-lane graph above
   (`0x82b34b84..0x82b34c18`). This displacement is computed BEFORE updating
   deck6. In particular N is not replaced with the deck body's subsequently
   normalized matrix, and L is not recomputed from that updated body.
5. Part::SetTransform(deck6,N) at `0x82b34c20`.
6. Visit indices0..6 ascending, explicitly skipping6. For each of0,1,2,3,4,5:
   - Read current LIVE Part transform at `0x82b34c50`, after all prior writes.
   - Compute moved = L * current, using the exact full-lane composition graph
     (`0x82b34c6c..f4`).
   - Compute normalized = Math::OrthoNormalizeRotation(moved) at
     `0x82b34d00`, and copy that raw result back to scratch.
   - Part::SetTransform(currentPart,normalized) at `0x82b34d44`.
7. AFTER the seven board updates, check mHookAssembly at `0x82b34d58`.
   If null, FailAssert line2492 at `0x82b34d70` then DebugBreak. On resumed
   execution it still dereferences the hook; native code has no safe recovery.
8. Part::SetTransform(hookPart0, ORIGINAL requested matrix) at `0x82b34d84`.
   The hook receives neither N nor any normalized nondeck scratch matrix.

There are seven Math normalization calls and eight Part setter calls for the
complete successful graph. Each attached Part setter performs its separate
rw normalization afterward. Do not precompute all seven old poses or all eight
results and then commit a transaction: the source reads and writes in order.
For the real board all Part bodies are distinct; that fact can simplify safe
ownership, not replace the source read chronology with unproven alias behavior.

No Vault fetch occurs in this path. The tolerance is a live cvar read, not an
attribute, fixed epsilon, or a constructor-only snapshot.

## Math::OrthoNormalizeRotation is a distinct algorithm

`0x82b22c78..0x82b22d24` performs reverse-axis CLASSICAL Gram-Schmidt. It first
loads the original translation and writes a temporary zero output translation.
It then processes input axes Z, Y, X, in that order:

```text
for current_axis in Z,Y,X:
    original = input[current_axis]
    candidate = original
    for previous_axis in already-written output axes, ordered Z then Y:
        projection = dot3(output[previous_axis], original)
        product = output[previous_axis] * projection
        candidate = candidate - product
    squared_length = dot3(candidate, candidate)
    estimate = reciprocal_sqrt_estimate(squared_length)
    twice:
        square = estimate * estimate
        half = estimate * 0.5
        residual = fma(-squared_length, square, 1.0)
        estimate = fma(half, residual, estimate)
    output[current_axis] = candidate * estimate
output.P = original input.P
```

Important details verified directly in PPC:

- Projections use the ORIGINAL current input vector (retained in v62), not
  the already shortened candidate (v63). Thus the X path subtracts its Z
  projection and then its Y projection, both computed from original X.
  Modified Gram-Schmidt is not an interchangeable implementation.
- Each projection product is a separate multiplication followed by a separate
  subtraction (`0x82b22ccc/cd0`), not a fused subtraction.
- Projection subtraction and final scaling retain all four vector lanes;
  dot3 uses XYZ. Each candidate's reciprocal-square-root estimate receives
  the source squared-length result and both refinement rounds run.
- The estimate/refinement sequence happens once per axis, in Z/Y/X order.
  No zero/epsilon/finite guard, alternate-axis selection or repair exists.
- There is no cross-product handedness correction. This is not the existing
  rw helper that independently normalizes axes and repairs a cyclic pair.
- Translation, including its W bits, is copied verbatim from the input at the
  end. Do not normalize it or invent a homogeneous1.

The current project host models for dot3 reduction and reciprocal-square-root
estimates can remain explicitly named numeric boundaries. Native VMX rounding,
subnormal behavior, NaN payloads and exceptional signed-zero bits remain open.

## Exact orthogonality assertion predicate

`Math::IsOrthogonalRotation`, `0x82b2f038..0x82b2f1b0`, is not a conventional
per-axis epsilon or determinant check:

1. Read the input XYZ basis. Build its XYZ transpose and a zero translation.
2. Form R-transpose * R, using X multiply -> Y FMA -> Z FMA per basis vector.
   The zero translation path is also evaluated through its actual zero-term
   arithmetic rather than omitted.
3. Subtract I,J,K from its first three columns and zero from the fourth.
4. Compute each error column's squared XYZ length e0,e1,e2,e3 using dot3.
5. Pack `[e0,e1,e2,e3]`. At `0x82b2f18c`, dot4 multiplies this vector by
   ITSELF. The resulting value is therefore the sum of SQUARES of the already
   squared error-column lengths, not the ordinary Frobenius squared norm.
6. Clear the scalar sign bit, compare strictly `> tolerance`, pack comparison
   bytes and return true only when the packed comparison word is zero.

The raw instruction at `0x82b2f18c` is `148529f5`; decoded VA is37 and VB37.
This confirms the fourth-power reduction despite IDA's misleading high-VA
display `vmsum4fp128 v36,v69,v37`. The packing permutations before it are
VectorPermuteConstant<0,0,0,5>, then <2,7,0,0>, then an eight-byte shift/merge.

For ordinary finite input the fourth error column is zero. Keep its source
arithmetic for exceptional-value semantics rather than silently deleting terms.
Strict greater-than accepts equality and unordered comparisons; a NaN final
error does not trigger the comparison. This is NOT a finite-input validator.
There is no determinant/handedness check, so a reflected orthogonal basis can
pass. Do not use a generic library orthogonality predicate as a substitute.

The cvar initializer `0x82f7ba20` calls cVarFloatRef constructor at
`0x82f7ba60` with section `Physics_ATEST`, name `kOrthonalTolerance`, title
`Orthogonal Tolerance`, default bits3c23d70a (0.01), minimum0, maximum1,
and null update callback. The setter samples the CURRENT target value each
call; the default is evidence, not permission to hardcode the runtime service.

## Attached Part::SetTransform behavior and preserved state

`0x82b25480..0x82b25ba0` receives a Part and the requested Part matrix. It
first reads Part.m_body (+0x4c). With no body it skips all body math and reaches
the unconditional raw Part cache copy below.

For an attached Part:

1. Read Part.m_inverseBodyLTM (+0x44). If present, form a rigid transpose
   inverse of that matrix and compute requested * inverse, with all four output
   lanes and the exact multiply/FMA graph. This is the same source graph as
   Part::AddToSimulation, already represented by world::part_attachment_transform.
   If the inverse pointer is absent, use the requested matrix directly.
2. Call rw::math::vpu::OrthoNormalize3x3 at `0x82b255a4` (inverse-present)
   or `0x82b2588c` (inverse-absent). This is additional normalization, not the
   outer Math function.
3. Write body Ri/Up/At XYZ followed by COM XYZ. Native stores preserve adjacent
   packed W words using lane insertion. They do not write those body metadata
   words from the caller's affine W lanes.
4. Capture the live body's inertia pointer BEFORE quaternion completion
   (inverse-present `0x82b2575c`, inverse-absent `0x82b259f0`). Construct the
   quaternion using the recovered sign/candidate-selection
   graph, with its four-lane estimate and two fused refinement rounds. Then
   compute quaternion dot4 and perform a SECOND estimate/two-refinement stage
   for final quaternion normalization before storing it. Candidate-only
   conversion is not enough. Existing simulation_quaternion_from_orthonormalized_matrix_host
   models both stages.
5. Test the RETAINED earlier inertia pointer. If nonnull, use its diagonal
   to update the six rotated
   inverse-inertia components with the existing basis/diagonal multiply/FMA
   graph and preserve unrelated packed W storage. Then RELOAD the body's
   inertia pointer (`0x82b2586c` / `0x82b25b58`) for the inverse-mass update.
   If the earlier pointer was null, skip those tensor/mass writes, leaving
   their existing values untouched. The Static hook's live body has null
   inertia even though its Part retains definition inertia.
   Independent raw source review established this early-capture/later-reread
   distinction. One immutable retained host owner can be an explicit policy;
   a callback-visible source read contract must not silently move both reads
   after quaternion completion or coalesce them into one late lookup.
6. In ALL branches, including detached, copy all sixteen ORIGINAL caller-input
   words into Part.m_partTM at `0x82b25b70..8c`. This copy is AFTER attached
   body writes. The cache does not receive the additionally normalized body
   pose or the inverse-mapped matrix.

No velocity, angular velocity, force, torque, RF, raw state, ring link, cooldown,
allocation occurrence, body identity, inertia-owner pointer, definition mass,
inverse-body transform, material, or Joint/Drive endpoint is reset or rotated
by this setter. The current pool's AddRigidBody commit is therefore not a
legitimate replacement for this update. Removing/readding would also alter
allocation/ring authority and potentially invalidate the existing constraints.

The actual PartDefinition inverse-body matrix remains definition-owned and
unchanged. For the authored board, deck has its previously forced identity
inverse-present1 while the ordinary fresh hook has inverse-present0. Both
branches must retain their distinct source paths, even for identity inputs.

## Live Part getter versus cached matrices

Part::GetTransform (`0x826c3480..0x826c35a8`) checks body first:

- No body: copy stored Part.m_partTM, all sixteen words.
- Body present, no inverse-body pointer: copy live body's basis/COM vectors.
- Both present: compose live body transform * inverse_body_ltm using the
  source full-four-lane multiply/FMA sequence.

The stored Part cache can intentionally differ from its attached body's
normalized pose. It can also become stale relative to integration. Therefore
reading `DynamicAssemblySession::part_transforms()` cannot implement the
outer setter's live deck or nondeck reads. That accessor currently returns
retained Part caches, not an authoritative per-body getter.

Existing `world::part_get_transform` preserves the branch logic and ordinary
finite XYZ semantics. The current `SimulationRigidBodyPool::try_rigid_body_pose`
returns live XYZ with canonical axisW0/translationW1, explicitly not the native
packed adjacent link/ID/pointer words. Its composed getter path also uses a
canonical-W affine helper. This is an honest finite-XYZ reuse boundary, not
proof of arbitrary raw native W equality.

Native raw W metadata does not justify manufacturing Xenon addresses. A host
extension should preserve that declared boundary and real occurrence authority;
when promising raw cache inputs, retain the actual caller matrix separately.

## Existing Rust reuse and remaining implementation seams

Read-only audit found these actual existing helpers:

- `world::part_attachment_transform`: correct present/absent inverse mapping
  and source rigid-inverse/composition order for the body setter.
- `world::orthonormalize_collision_affine`: rw OrthoNormalize3x3 only; its
  documented host estimates/reduction boundaries apply.
- `world::simulation_quaternion_from_orthonormalized_matrix_host`: original
  candidate stage PLUS final quaternion normalization.
- `world::simulation_rotated_inverse_inertia_host`: original six-component
  tensor rotation and inverse-mass semantics, with null selection outside it.
- `world::part_get_transform`: current branch logic and finite-XYZ getter.
- `SimulationRigidBodyPool::try_rigid_body_pose`: checked current live pose.
- DynamicAssemblySession retains the actual per-Part body tokens, definition
  metadata and cached transform; SimulationAssemblyBackend owns the live pool.

The following producers are still missing at this audit checkpoint:

1. Math::OrthoNormalizeRotation and the exact assertion predicate/current-cvar
   sampling contract. Do not reuse the differently behaving rw helper.
2. A narrow owner-qualified in-place body pose/tensor update in the real pool,
   preserving all unrelated fields and current allocation occurrence.
3. Session-level live Part get/set with backend authority and index validation,
   using the retained inverse metadata and actual body token. It must perform
   the body mutation before writing the raw Part cache.
4. The ordered outer board/hook setter, calling those real services rather
   than yielding a detached vector of matrices and claiming integration.

The current generic DynamicAssemblySimulationBackend offers add, pose read,
remove and constraint lifecycle calls, but no live setter. Root can choose a
concrete private session extension rather than broadening every backend trait.
The operation should require mutable access to the owning backend so the
existing exclusive DynamicUpdate phase prevents pose mutation beneath prepared
constraint/RF plans. Run relocation before phase construction and before any
ground/contact image whose freshness depends on body state is published.

Neither copied public snapshots nor unrelated mutable handles should become
an admission authority. Cross-session board/hook ownership should resolve the
real hook body; do not combine it into a fabricated eighth board Part.

## Error and numeric boundaries

The source's outer assertion is not a preflight transaction, and its hook
nonnull assertion occurs after seven board updates. Failure handling in safe
Rust must describe any earlier validation or rollback as host policy rather
than source parity. Native source does not reject NaNs via the orthogonality
guard, and resumed assertions continue execution.

For a fallible host numeric provider, preserve or explicitly disclose mutation
prefix semantics: the attached setter writes axis/COM fields before the
quaternion and rotated-inertia arithmetic finish, then writes the Part cache
last. Whole-board all-or-nothing output computation is not the original call
sequence. Ordinary finite infallible host backends can reuse accepted numeric
graphs while keeping Xenon estimate, reduction, denormal, NaN and W claims
separate.

No original per-tick spawn trace has been captured for this packet. Reading
source, moving matrices in a local test or retaining endpoint counts does not
establish native input/animation/physics gameplay parity or playable skating.

## Focused acceptance discriminators

- Nonidentity LIVE old deck and requested input: prove L=N*rigid_inverse(D),
  not a simple offset or multiplication against constructor identity.
- Source call order deck6,0,1,2,3,4,5,hook0; each nondeck getter follows prior
  setter calls. L remains the pre-deck-set value.
- Reverse classical Gram-Schmidt versus both modified Gram-Schmidt and the
  existing rw axis-repair algorithm. Observe estimate calls Z/Y/X and distinct
  separate projection multiply/subtract rounding.
- Seven outer Math normalizations plus eight attached rw normalizations and
  body quaternion conversions for the complete graph.
- Getter input where cache differs from current integrated body pose, and
  setter result where cache differs from normalized body pose.
- Deck inverse-present versus hook inverse-absent; hook receives ORIGINAL
  requested matrix in its final raw cache, not N.
- Nonzero velocities, omega, forces, torques and RF rows survive relocation;
  body states, rings, occurrences, existing Joint/Drive endpoints remain fixed.
- Nonnull inertia updates all six rotated components/mass; null live inertia
  preserves existing tensor fields even with retained Part definition inertia.
- Orthogonality quartic reduction, equality/NaN comparisons, reflection and
  current tolerance sampling; no generic finite/handedness guard substituted.
- Foreign/stale/index-invalid safe failures and attempts to mutate beneath
  an active phase; document preflight/panic-prefix behavior explicitly.
- Relocated real board and hook subsequently use the same existing pool-backed
  Joint/Drive Build and Horus path; no snapshot-rehydrated body or fake address.

## Raw source fingerprints

Independent live source audit PASS for all six hashes and the proposed graph.
Root read the complete packet. One chronology clarification was integrated:
Part setter captures the inertia pointer before quaternion completion, retains
that pointer for diagonal loads, then reloads the live pointer for inverse mass.
More precisely, capture follows candidate estimate in both branches: present
inverse after first candidate refinement (2574c then2575c), absent inverse before
first refinement (259f0 then25a40). Immutable host inertia ownership and a pure
numeric helper must not claim arbitrary mutable callback interleavings. Source
approval is not implementation approval; no spawn Rust implementation exists at
the HookPool gate61189 checkpoint.

SHA-256 over live IDA raw bytes `[start,end)`:

| Function | Range | SHA-256 |
| --- | --- | --- |
| SkateboardBody setter | `82b34a90..82b34d98` | `205487dde2cbad60f045f8e5e73d777f39129a222df4c66754d420a2083f3143` |
| Math normalization | `82b22c78..82b22d24` | `dfe744df56df5b2f68438a3f8f8e66bd42e33ed4b005d6db857978c3242bb281` |
| Orthogonality predicate | `82b2f038..82b2f1b0` | `56439446d95490ad936d0acdf9623c074f58bdb466ab2293e1efb6c690725d41` |
| Part setter | `82b25480..82b25ba0` | `19c1db6470002ab49c7d8d10fe507958d8153899f7f6ff1cc66c5959fb95e8af` |
| Part getter | `826c3480..826c35a8` | `ad60181e8e2c92d1f75b60b092d975aeb92f3166f64ea6627d7633fc827edf32` |
| Tolerance initializer | `82f7ba20..82f7ba80` | `b3c11a04ace7ff79f1472b1d72a1517e4c554f01c0e52f90405261666463cb2d` |
