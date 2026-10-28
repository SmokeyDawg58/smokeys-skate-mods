# Grounded front and rear foot force source packet

2026-09-08. Initial bounded read-only source/caller audit,
followed by the explicitly assigned leaf implementation and original-Vault
inspection recorded at the end. No Cargo execution was performed by the
author. This targets actual grounded forces after active IK, not parked DMO.

## Functions and actual callers

| Source symbol | Half-open span | SHA256 of original IDA bytes |
|---|---|---|
| `?Toolkit_CalcFrontFootForce@Skateboard@Physics@Sk8@@QAA?AUSkateboardForce@123@MM@Z` | `82b28b70..82b28c5c` | `5e04e32d0bd4c28fd65bd27e97ad98ee5651d694d54cff9c62b53dadccfcb8e1` |
| `?Toolkit_CalcRearFootForce@Skateboard@Physics@Sk8@@QAA?AUSkateboardForce@123@MM@Z` | `82b28d98..82b28ed8` | `68fcab0fe4f2d06d9e69c047e2c6e5a888709ac34ad21a35754af73a431e7e2d` |
| `?ToolkitHelper_CalcFootForce@Skateboard@Physics@Sk8@@AAA?AUSkateboardForce@123@MMMM@Z` | `82b28828..82b28b70` | `9a58a627cae27b80fcac0cc00070d81d3e5f5ab79fc6d19c35dc7a5aecf26536` |
| `AddSkateboardForce` | `82b38ce0..82b38d88` | `0256b541ee65fae2373758d7c83802bd0542f6bf21dd0b46cba2be3f0c15299a` |
| `ApplyForces` | `82b35380..82b355a0` | `15499ec513913eaf53ffdb17b1e006e4e93b87d25eeddfa1547f44cfec616608` |

Both wrappers have exactly three direct callers found in this IDB:

| Caller | Front call | Rear call |
|---|---|---|
| `PhysState_GroundAnimation::UpdateSkateboard` `82d8a7b0` | `82d8a8b8` | `82d8a8cc` |
| `PhysState_SlideGround::UpdateSkateboard` `82d8e2a8` | `82d8e8a0` | `82d8e8b8` |
| `PhysState_PhysicsGround::UpdateSkateboard` `82d9a018` | `82d9a6ec` | `82d9a704` |

The bounded GroundAnimation non-jump branch supplies `timeSinceLanding=1.0`
and `absorption=+0.0` to both. It calculates front, rear, brake, slide,
linear drag, collision in that order. Only afterward does collision choose
either the type-15 collision entry or the four ordinary entries front4,
rear5, slide1, brake2, followed by the linear-drag write. Calculations must not
be skipped because the later collision result discards their returned forces.
The type-15 collision branch suppresses the linear-drag write entirely.
The existing `physics.rs::update_ground_animation_non_jump` preserves this
wrapper order, but its force trait has only recording-test implementations.
Earlier steering, wheel-material and ground-jump logic remains outside that
narrow non-jump helper.

The wrappers and helper return a 32-byte value through a hidden output
argument: source `r3` is output and `r4` is Skateboard. IDA's guessed
`this/result` labels are reversed. The apparent `result[8]` pointer arithmetic
is actually Skateboard member access; it must not become a native Rust layout.

## Retained typed inputs and outputs

PDB Skateboard fields: deck body pointer `+10`, Reckoning `+104`, current
`ProcessedPhysIn* +110`, retained applied-force queue `+120`, own
`physicsdeck` debug attributes `+534`.

The foot path consumes current processed fields:

| Offset | PDB name | Use |
|---|---|---|
| `+180` | `mBodyYDirection` | Full-lane absorption direction, sign-bit inverted |
| `+190` | `mVelocity` | NormalizeSafe input for force projection removal |
| `+96c` | `mSurfaceSpeed` | Single-precision multiply by 0.125 |
| `+9a0` | `mBalance` | Discrete front/rear allocation and landing activation |

It separately reads actual `Reckoning::mUpVector +1b0`; this is not permission
to substitute processed up, ground normal or world J. The existing
`Reckoning::up()` exposes that retained four-lane field.

`SkateboardForce` has `mWorldSpaceForce +0` and `mLocalSpacePos +10`, both
16-byte vectors. `AppliedSkateboardForce` has type at `+0`, force at `+10`,
size 48. Current `physics::SkateboardForce` only stores two XYZ arrays.
A full-lane internal result is needed before calling this reconstructed path
parity: output force W is generally propagated, while local position is exactly
`[+0,+0,forwardDisplacement,+0]`.

## Runtime attribute reads

Every scalar below uses `GlobalAttributeCollection::physics_feet_default`
at singleton `+bc`, `GetAttributePointer(key, index=0)`, and, for a missing
pointer, `DefaultDataArea(4)`. Preserve these lookups and pointer read timing;
do not hoist a tuning snapshot or replace missing values with guessed numbers.
The PDB physics_feet layout names only three unrelated graphs; original
authored names for these scalar keys were not identified by this audit.

| Role used in this packet | 64-bit key | Lookup call |
|---|---|---|
| Positive local foot offset | `481e7fa61a3e5ad0` | Front `82b28c14`, rear `82b28e90` |
| Rear absorption scale | `63412dc23ce787d1` | Rear `82b28e08` |
| Landing blend denominator | `7a0429eb688c3919` | Helper `82b288b0` |
| Surface-speed coefficient | `1a77d90a2eea371c` | Helper `82b28974` |
| Initial force coefficient | `1ef73284bdb152de` | Helper `82b289cc` |

These keys were recovered from `lis/ori/insrdi` raw instructions, not the
decompiler's spurious extra integer arguments. Original VLT values were later
inspected and are recorded below; runtime reads remain dynamic. Scalar schema
names remain unidentified. Existing AttributeRuntimeVault/Instance services
and the current `ReckoningVaultAttributes` pointer/value pattern are reused.

Read order differs between wrappers. Front reads balance before the offset
lookup. Rear resolves and reads its absorption scale first, then reads balance,
then resolves and reads its offset. The rounded absorption product executes at
`82b28eb4`, **before** offset negation at `82b28eb8`.

## Wrapper scalars, including exceptional branches

Independent review correction, verified again against the original raw words:
IDA's `ble` mnemonic must not be translated as a host ordered `<=` test.
Front branch `82b28bac = 4099000c` decodes to **BO=4, BI=25**, testing
CR6.GT **not set**. It takes the branch for LT, EQ and unordered. Front then
executes `fsel(balance,1,2)` (`82b28bc8 = ffe0636e`) on that branch;
the positive fallthrough explicitly assigns zero. Therefore front NaN
produces **2**, correcting the earlier erroneous value zero in this packet.
Front passes positive offset and unmodified absorption to the helper.

Rear produces general scalar 0 for negative or NaN balance, 1 for either zero,
2 for positive balance. It passes the bit-negated offset and the rounded
single-precision product `rearAbsorptionScale * absorption`. Do not use a
generic signum or clamp that chooses different NaN/signed-zero behavior.

Rear branch `82b28e34 = 40990010` also has **BO=4, BI=25** and tests GT-not-set.
Its taken branch prepares candidate one; positive fallthrough prepares two.
Then `fsel(balance,candidate,+0)` (`82b28e50 = ffe06b2e`) selects zero for
negative or unordered balance. This second select explains why identical
branch encodings do not imply identical front/rear NaN outputs.

| Balance relative to +0 | CR6 LT/GT/EQ/UN | GT-not-set branch | Front scalar | Rear scalar |
|---|---|---|---|---|
| Negative, including -infinity | 1/0/0/0 | Taken | 2 | 0 |
| Either signed zero | 0/0/1/0 | Taken | 1 | 1 |
| Positive, including +infinity | 0/1/0/0 | Not taken | 0 | 2 |
| NaN/unordered | 0/0/0/1 | Taken | 2 | 0 |

The compare at front `82b28ba8` and rear `82b28e30` is the same raw
`ff006800` (`fcmpu cr6,f0,f13`). BO=4 ignores CTR and branches when the
selected CR bit is zero; BI25 is CR6.GT, not a combined ordered LT-or-EQ
predicate. Both signed zeros also take the nonnegative arm of `fsel`.

## Shared helper arithmetic and publication order

Let `t`, `d`, `a`, `g` be the helper's time, displacement, absorption and
general-scalar arguments. Let `L`, `S`, `I` be its three attributes in table
order. Below `mul/add/sub/div` round to single precision; `fma` is the native
single-precision fused operation. `fsel(c,x,y)` selects x when c is nonnegative,
including either zero, and y otherwise, including NaN.

1. Resolve/read L. `r=div(t,L)` at `82b288cc`; load current balance at
   `82b288e0` and compare it with zero at `82b288e4`; then
   `u=fsel(-r,+0,r)` at `82b288ec` and
   `q=fsel(sub(1,u),u,1)` at `82b288f4`. This realizes the source
   clamping graph, including NaN ratio selecting q=1. It is not a license to
   replace the graph with a host min/max chain.
2. Use that earlier balance comparison. `active = (balance == 0 ? 0 : 1)`;
   NaN is active.
   `landing = (t == 0 ? active : 1)`, including either signed time zero.
3. Read current surface speed; `speed=mul(surfaceSpeed,0.125)` at `82b28938`.
4. Resolve S, retaining its pointer. Resolve I next. Only afterward read S
   (`82b28a14`) and I (`82b28a20`); a fallible late lookup cannot be reordered
   behind an early S value read.
5. `speedForce=mul(S,speed)`; `initial=mul(sub(1,q),I)`;
   `steady=mul(speedForce,q)`; `blend=fma(initial,landing,steady)` at
   `82b28a9c`; `negativeMagnitude=-mul(blend,g)` at `82b28aa8..ab4`.
6. Bit-negate **all four lanes** of current processed body-Y direction.
   `absorptionForce=mul4(negativeBodyY,splat(a))` at `82b28adc`.
   Load actual Reckoning up; `rawForce=fma4(up,splat(negativeMagnitude),
   absorptionForce)` at `82b28af0`.
7. `direction=NormalizeSafe(processed.velocity, [+0;+4])` at `82b28af4`.
   Load `kDrawAbsorptionCalc` target pointer at `82b28b08`, execute
   `dot3(direction,rawForce)` at `82b28b0c`, then read the target bool at
   `82b28b10`, then `parallel=mul4(direction,splat(dot))` at `82b28b14`.
   The dot excludes W but the multiply preserves full lanes.
8. Check that sampled live draw bool; when true, require the
   actual debug-overlay singleton before the final output write. The compiled
   function contains no further draw call here. Missing overlay reaches native
   assertion/DebugBreak; a host capability error must preserve earlier stores.
9. Publish `force=sub4(rawForce,parallel)` at `82b28b44/82b28b4c`.

The helper initially stores output force zero at `82b28abc`, output local
position zero at `82b28ac4`, then local position `[0,0,d,0]` at `82b28acc`.
Thus a later NormalizeSafe/numerical or debug-overlay refusal leaves those
output prefixes. A by-value success result alone does not model that partial
output if the caller needs exact fallible-host observation.

The normal ground-animation `a=0,t=1` must still execute this sequence. In
particular zero times unusual vector/attribute values is not always zero,
and the source still resolves every attribute and calls NormalizeSafe.

### Raw vector-operand correction

IDA's high VA register rendering is wrong in this graph. Raw fields were
decoded independently using the published [Xenia VMX128 instruction field
layout](https://raw.githubusercontent.com/xenia-project/xenia/master/src/xenia/cpu/ppc/ppc_instr.h).
This is encoding documentation only; all game behavior and words below come
from the original reference executable. No interpreter or emulation runtime
is introduced.

| Site/raw word | Correct operands |
|---|---|
| `82b28a78 / 1b9ff8f5` | shift v63 by v63 to v60, constructing sign-bit mask |
| `82b28a8c / 177de335` | xor v61 (bodyY) with v60 to v59 |
| `82b28adc / 141ba8b1` | multiply v59 by v53 to v0 |
| `82b28af0 / 17f4013c` | multiply v52 by old v127, add v0, destination v127 |
| `82b28b0c / 167ff9b7` | dot3 v63 (normalized velocity), v127 into v51 |
| `82b28b14 / 17df98bd` | multiply v63 by v51 into v126 |
| `82b28b44 / 165ff477` | subtract v126 from v127 into v50 |

`reckoning_numeric::try_normalize_safe` already implements the reviewed
`Math::NormalizeSafe 824b18e0` graph, including zero fallback and source
epsilon. Reuse its explicit numerical boundary; Xenon estimate, reduction,
denormal and exceptional payload parity still needs native traces.

## Queue-to-real-deck backend dependency

Foot helpers only produce force records. `AddSkateboardForce` uses the actual
Skateboard queue, appends while count <21, copies full force and position, and
silently ignores subsequent insertions at capacity. Existing
`SkateboardForceQueue` preserves the cap/order but is XYZ-only and currently
does not drive the actual board backend.

Actual `Skateboard::UpdatePrePhysics 82b37cb8` reads Reckoning up, starts
wheel-line testing (`82b37cfc`), then calls `ApplyForces` at `82b37d04`.
`ApplyForces` early-returns for an empty queue. Otherwise it resolves own
physicsdeck key `de53599e9c53fa4a` (missing uses DefaultDataArea4), adding its
value as a local Y offset to each queued application position. It iterates
entries in order, transforms the local arm by the actual deck body orientation,
adds force scaled by cached inverse mass to the actual body's force XYZ,
adds the inverse-inertia-transformed torque contribution, and clears cooldown
after each entry (`82b35560`). Its final own lookup
`93d84377c4da2d5b` uses DefaultDataArea1 when absent even though no later read
appears in this compiled function.

This packet identifies that consumer and its fields; its cross-product,
inverse-inertia packed-lane graph and exact per-axis stores require a separate
bounded raw arithmetic review before implementation. Do not simply sum forces
into a generic rigid body or drop application arms. `ApplyWorldSpaceForceToDeck
82b355d8` takes a different world-position input and is not a substitute for
the queued local-arm consumer.

`UpdatePostPhysics 82b37d20` clears/reset-binds the queue at
`82b37e78/7c/80` after its normal or recovery branch. `ApplyForces` itself
does **not** clear it. Clearing at insertion or immediately after applying
would change repeated pre/post scheduling and failure behavior.

Existing actual backend `try_apply_part_body_displacement` targets skeleton
displacement, only changes force XYZ/cooldown and uses a different calculation.
It cannot stand in for deck force-at-local-arm torque. The existing body pool
already retains inverse mass, orientation, rotated inertia, force/torque and
occurrence handles; a checked queue consumer must borrow that same admitted
deck body through the board's actual Part ownership. New force/torque writes
must preserve adjacent packed fields and the existing solver's acceleration
semantics. No second body or parallel force accumulator is justified.

## Minimal next implementation and verification

1. Add a scoped full-lane foot-force projection over the actual current
   processed collaborator, retained Reckoning and runtime physics_feet
   attribute reader. Keep pointer versus value reads and late cvar timing.
2. Bind a fallible version of the existing ground-animation force collaborator
   to the actual board/session. A fallible interface is necessary for runtime
   attribute, owner and numerical refusal; infallible recording traits alone
   cannot express these boundaries.
3. Complete the queued ApplyForces consumer separately against the existing
   deck body, then integrate its native pre/post scheduling. Other force
   calculations remain explicit missing callees; this packet does not
   implement brake, slide, drag, collision, steering or jumping.
4. Validate original Vault scalar values/schema/inheritance and replay selected
   original processed vectors. Focused edge cases: negative/zero/positive/NaN
   balance, both signed time zeros, q endpoints/NaN/infinities, pointer lookup
   failure before delayed coefficient read, zero/NaN velocity fallback,
   nonzero W, nonzero absorption, late overlay refusal, queue cap21 and
   repeated prephysics before postphysics clear. Native per-tick force/torque
   traces remain the parity gate.

Confidence **high** for wrapper/attribute/helper scalar and corrected vector
operation ordering and source ownership; **medium** for the summarized
ApplyForces consumer pending its independent detailed raw audit. No live
runtime execution or deterministic control-force parity is claimed.

## Assigned leaf implementation and original scalar provenance

New `skateboard_foot_forces.rs` defines `SkateboardFootForce` with typed
`SkeletonRootVector` position and force, preserving known or unknown native W.
`SkateboardFootForceServices` borrows processed/Reckoning reads, runtime
attribute reads, late draw-target/overlay reads and the shared numerical
policy. `try_front` and `try_rear` accept a mutable output and update it only
at the source output stores. Early read errors preserve the caller's entire
buffer; late numeric/overlay errors preserve the source zero-force and local
position prefix.

The typed existing `skeleton_ik_line_frame::normalize_safe` wraps the accepted
common normalization arithmetic and is reused directly. A zero-length fallback
establishes known zero W; a successful normalization of unknown W preserves
unknown W. Mul/FMA/sub propagate W knowledge explicitly, even for zero
coefficients. XYZ-only scratch for projection dot never publishes fake W.

New `skateboard_foot_attributes.rs` binds actual global
`physics_feet.default` through `AttributeRuntimeVault`. Each lookup returns
an opaque token retaining the original `AttributeRuntimeData` and a unique
instance identity; the reader rejects tokens from any other instance, even
for the same key/Vault. This retains pointer lifetime and lookup-versus-value
ordering without inventing mutable singleton-replacement support.

Generated ctor `828b0e10..828b0e88` supplies class key
`5338958c408b6009`; its byte SHA256 is
`4addb179e45815bd224087b4b7d52b224d0213145206e57a2ae0b0ede56502ac`.
`GlobalAttributeCollection` prepares default key `d7edbd362d7d2152` in r28
at `828b1630..1644`, invokes that generated ctor at `828b25e0` and stores
the result to `+bc` at `828b25ec`. Both key names independently match the
existing recovered `Attrib::StringHash64` research helper.

The author used the **already built**, read-only `inspect_vault.exe` on
original db.big's extracted temporary VLT/BIN regions, not Cargo. Original
skater.vlt begins at archive offset 256 with length 851568; skater.bin begins
at 851840 with length 812520. The extracted VLT SHA256 is
`496215b210c3ce8bda3f7a64b3cc3d59c56546e8d3026dbf1da4a72f93fa7dde`.
Original class export offset is 377480; default collection has parent zero
and 19 explicit nodes. The five relevant values are inline nodes, type index
1, node flags `40`, entry flags `00`; definitions have type id
`e22228fbb3c209d8`, size4, maximum_count1, flags4 and alignment2.

| Role | Original bits/value | VLT node-key offset | Inline word offset |
|---|---|---|---|
| Foot offset | `3e6147ae` / approximately 0.22 | `5c320` | `5c328` |
| Rear absorption scale | `3f000000` / 0.5 | `5c2f0` | `5c2f8` |
| Landing denominator | `3dcccccd` / approximately 0.1 | `5c330` | `5c338` |
| Surface-speed coefficient | `41a00000` / 20 | `5c380` | `5c388` |
| Initial force coefficient | `42c80000` / 100 | `5c360` | `5c368` |

A separate raw-byte scan verified each node key, inline word and flags at
these offsets. Rear absorption's key also occurs in another collection at
`5be38`; that occurrence is not substituted for the selected default node.
Hashing the IDA string catalogue found no exact string matches for the five
scalar keys. Semantic role names in this implementation do not pretend to be
recovered original authored names.

These values appear only in original-data **test assertions**, never runtime
construction or fallback. `skateboard_foot_attributes_tests.rs` provides a
`check_original(vault)` helper and an ignored mounted test that compares the
actual runtime reader against independent original archive bytes. Three
ordinary provider tests cover class hashes/defaults, inherited retained data,
foreign token rejection and deferred malformed-scalar refusal.

Ten force tests cover front/rear source read order, the corrected NaN balance
table, signed-zero offset/time, deferred value reads, early versus late output
prefixes, target capture before projection dot and bool read after, overlay
refusal, a cancellation fixture distinguishing vector FMA from multiply/add,
unknown W and NormalizeSafe fallback, and exceptional landing-ratio selects.
No author Cargo or original mounted-test pass is asserted; root registers
the modules and owns compilation, execution and independent review.

This implementation remains a scoped calculation/provider. Existing public
XYZ-only force records, queue cap/clear scheduling, real deck force/torque
application and other grounded control-force functions are separate required
integration work. No playable or live grounded-control milestone follows from
these leaves alone.
