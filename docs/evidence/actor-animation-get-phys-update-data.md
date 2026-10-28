# Selected retained SkaterAnim physics output

Integrator continuation: root reviewed full GetCurrentComPos82c933a8 and
Bezier482c93070 disassembly, including the basis/FMA chain93258..9330c.
Raw93648 stores duration into temporary endSpeed.W before copying it; this
store is omitted by the decompiler and the implementation correctly retains
it. The reciprocal residuals937b4/c4 use negative rounded fused subtraction;
their signed-zero form was corrected after independent review. The Actor
prepare entry now calls the full producer with an explicit numeric service,
then the same-owner Motion transport. Current corpus uses the host numeric
policy and still does not establish a complete skating frame.

Branch review correction: raw `4099` is BO4/BI25, meaning CR6.GT is clear.
Without an intervening CROR, its `ble` mnemonic includes unordered operands.
Graph reviewer caught the distinction; root confirmed bytes at93454,93680,
977b0 and977c8. The Air assertion executes only on strict `>`, the Bezier
division only on strict positive duration, and each wipeout threshold must
be strictly exceeded. Code and NaN regression expectations were corrected;
earlier notes claiming ordered BLE behavior are superseded.

Author proof packet, 2026-09-12. Confidence: **high** for the selected
non-locomotion invocation; the full locomotion addition below requires separate
independent review of its numerical graphs. Independent integrator review and current
workspace/corpus gates remain required; this packet does not claim frame-loop
or skating parity.

## Native ownership and caller

`Sk8::Skater::SkaterAnim::GetPhysUpdateData` at `82c96d58` is called by
`Sk8::Skater::Actor::SetUpPhysics` (`825ff6c0`) at `825ff75c`, after resetting
the Actor-owned `AnimOutPhysIn` at `825ff740`. Motion payload publication,
MAIN intent-map assignment, and ordered MG/Andale attribute transfer follow
at `825ff778..825ff7f0`. The physical interface call follows at `825ff978`.
The new child of `actor_animation_construction.rs` operates on that retained
animation owner and its initialized local/global pose-buffer owners. It takes
no substitute pose or trajectory input. Outer Actor mounting and original-data
execution are integrator work; a selected invocation is not full body-tree Eval.

## Exact selected ordering

1. The size assertion is `82c96d8c`; global/local source pointers are captured
   at `82c96dac/6db8`. Store caller dt, zero both surface IDs, and clear request
   bit `08000000` at `82c96dbc..6dc8` before any action query.
2. The physical-foot action probe is unconditional (`82c96dcc..6e14`). Action
   1 determines destination bit 26. Runout front then conditional back determine
   bit 25. `IsRunningOutFront82cb7a58` checks source `612c` bit `4000`, then
   action 15; `IsRunningOutBack82cb7ad0` checks bit `2000`, then action 16.
3. `WasInLocomotion82cb7610` reads source bit `80000`. Its false branch clears
   source bits `80008` (`82c973f0..73fc`) and all four last-COM lanes at owner
   `+5e10` (`82c9740c`). The selected method reports the unreconstructed true
   branch at its actual branch point and preserves preceding writes.
4. Validate each global matrix's four axes in bone/axis order
   (`82c9742c..750c`), then both destination foot targets (`82c97514..757c`).
   Copy global (`82c97590`) before local (`82c975a4`), each `mSize * 64` bytes.
   Rust reads the actual synchronized `OwnedAnimationPoseBuffer` matrices;
   missing output fails explicitly. A late missing local source preserves the
   already copied global destination. Local matrices have no native NaN guard.
5. Publish board flip (source bits 30 and 31 differ), mirrored (bit 30), switch
   (`GetIsSwitch82cb7580`, relative stance 1), fakie (`82cb7530`, bit 29), then
   query fakie again and weight-on-nose (`82cb7558`, bit 28). Weight forwards
   inverts nose when fakie. Regular stance tests natural stance zero.
6. Destination bits 31/30 query current/prior locomotion; bit 28 receives source
   bit `400`. Load dismount frame count, clear source `400`, then store count
   (`82c976c4..76d4`). Query current locomotion again. Add the fixed f32 update
   constant to owner timer `+615c` (`82c97704`), then insert current into prior
   source bit 19 (`82c9770c`). The constant is `kUpdateTime_56` at `822f2204`,
   bits `3c888889`, independent of caller dt.
7. Non-Air state supplies positive-zero air/landing times and clears destination
   BigAir bit 29 (`82c97798`). Read minimum cvar, conditionally landing cvar,
   then source bit `200`. Raw `4099` branches when CR6.GT is clear, including
   unordered NaN, and skips the request. Each gate requires ordered `>` to
   continue. Final request is ORed into bit
   27, preserving any request produced earlier (`82c977f0`).

The VMX128 debug check's raw first sequence includes `16b6b2f5` (copy/or),
`1aa1b755` (rotate/insert mask 1, rotation 1), and `1a95a865` (self equality).
It substitutes X for W before checking equality: XYZ NaNs fail, W NaN is
ignored, and infinities pass. IDA's VMX128 high-register operand rendering is
not reliable; this interpretation must be reviewed from the raw encoding.

Integrator review: fresh IDA bytes at `82c9742c` were
`12ddd0c7 16b6b2f5 1aa1b755 1a95a865`. Applying the independently
recorded VX128 VA extension rule (bit 5 to register bit 5, bit 10 to
register bit 6) gives the same loaded vector as both OR inputs, and the
same modified vector as both equality inputs. The rotate/insert selects
W from rotated X; this checks XYZ while preserving the original W on copy.
Fresh bytes at `822f2204` are `3c888889`, confirming the fixed timer step.
The integrator also reviewed the full native branch/copy/flag sequence;
original mounted execution and current Cargo results are recorded separately.

## Inactive action probe and allocation provenance

Trajectory input feet begin at `+0`; physical feet begin at `+500`. There are
four feet of stride `140` in each array. A foot contains three states of stride
`50`, action at state `+44`, foot data at `+f0`, stride at `+120`, initialization
bytes `+130/+131`, and reserved trailing storage at `+134`.

The action address is `trajectory + 544 + 140*index + 50*current_state`.
For nonnegative physical index, source reduces modulo four; for negative
index, it adds four once. Invalid remaining indices are rejected explicitly.
With constructor/reset state `-1`, physical index zero reads input foot 3's
reserved `+134`; physical indices 1/2/3 read physical foot 0/1/2's reserved
`+134`. This is a same-allocation read and is never an alias of state zero.

Actual Actor allocation at `825fedbc` requests `61d0` bytes from category Sim,
debug name `SkaterAnim`, flags zero. `XCore_EASTL_Category_Alloc828b4bf0` obtains
the category allocator. Sim row 55 at `82ff0eb0` has raw bytes
`821678a0000800000000000700000000ffffffff00000100`; its flags mask is
`ffffffff`, OR is `100`. Both category allocator overloads (`82972d28`,
`82973570`, latter re-decompiled during final audit) pass
`(requested_flags & mask) | category_OR` into `Internal::Alloc82972a18`.
`AllocHeapMem82972240` clears the allocation for bit `100` in both small and
heap paths (`82972340`, `829723bc`). Thus all eight reserved foot tail words
start at zero on this actual allocation.

Rust records this narrow provenance immediately after retaining the new
Actor animation allocation, before face-manager binding. Generic construction
keeps the word unknown. Foot Reset preserves its previous reserved word;
Reset does not turn arbitrary storage into known zero. No whole-allocation
zeroing inference is applied to unrelated fields or owners.

## Live cvar references

The graph-runtime agent owns the additions to `input_cvar_runtime.rs`.

| Reference | Initializer / construction | Static table slot | Default bits |
|---|---|---|---|
| `kTimeToLandToWipeout`, `830a4c98` | `82f80d88` / `82f80dc8` | `82fd48dc` | `3fb33333` (1.4) |
| `kMinTimeInAirToWipeOut`, `830a2884` | `82f80de8` / `82f80e28` | `82fd48e0` | `3f4ccccd` (0.8) |

Both use section `Animation_Locomotion`, matching label/title, bounds 0..5.
Static registration is landing before minimum. This function samples minimum
first (`82c977a8`), landing only after the first comparison (`82c977c0`).
`try_min_time_in_air_to_wipeout` and `try_time_to_land_to_wipeout` read live
reference targets; thresholds are not baked into the producer.

## Validation and remaining dependencies

Five focused tests cover predecessor mapping (including negative-index rules),
state-zero distinction, generic unknown storage versus actual Sim provenance,
Reset retention, prefix side effects before failure, exact suffix flags and
fixed timer addition, live overridden cvars and OR preservation, and XYZ/W NaN
behavior. They deliberately do not manufacture a successful body output fixture.
The root agent owns Cargo and original-corpus gates for this concurrent batch;
no successful gate is claimed here before its recorded results.

The integrator subsequently reported original-data gate `original4`: all 17
tests passed including retained non-locomotion body output through the Actor
reset/GetPhys/transport method. This result predates the full locomotion changes
below. Full body-tree dispatch, Action/Motion runtime, Skeleton consumption,
outer phase execution, and deterministic per-tick reference traces remain
separate required closures toward the first faithful skating test.

## Air suffix correction from raw source

Fresh `82c97728..7768` disassembly corrects two misleading decompiler outputs.
The `+a08` load is physical index, `+a04` is current state, and the final load
is `trajectory + 540 + 50*(4*physical_index + state)`. This is the **physical**
foot state's duration. The source does not wrap this index. The full runtime
Vault key assembled with `insrdi` is **`971e130c27193d76`**, not its low 32 bits
`27193d76` alone. It is sampled at index zero from the retained Offboard
instance. Missing pointer calls `DefaultDataArea828b5340` (shared zero area
`83059d30`); malformed nonnull data remains an error. Raw `bgt` at `82c97788`
sets BigAir only for ordered `duration > threshold`. Unordered comparisons
remain false here; the later BLE (NOT GT) cvar gates also skip the request
on unordered operands. The selected API now
completes this suffix too. A focused test distinguishes input versus physical
duration, no-wrap indexing, and unordered comparison outcomes.

## Full locomotion method and trajectory numerical graph

`try_get_physics_update<N: ReckoningNumeric>` is the added full API. It shares
the original prefix/suffix with the selected method; the latter still rejects
prior-locomotion at its original boundary. The numerical service supplies only
primitive reciprocal estimates and XYZ dot reduction. Despite the service's
historical Reckoning name, the animation graph is recovered independently.
Host seeds, dot reduction rounding, subnormals and exceptional fused-operation
details retain the existing explicit host numerical boundary; no Xenon bit
parity or completed trajectory producer is claimed.

The locomotion branch adds owner `mCollisionTrajDisplacement+5f80` to the actual
physical foot position (`82c96ed4/6ed8`), calls `GetCurrentComPos82c933a8` world
then animation, validates both XYZ results, and considers current and next
physical feet. It uses the next only if both are initialized and next Y is
strictly greater. Position is stored before the live world-Y-normal query;
valid is stored afterward. Uninitialized current does not fabricate a target.

`GetCurrentComPos` directly indexes the physical foot/state, then clamps the
same `mTime` with two FSEL operations (`82c93408/341c`) and an intervening
rounded subtraction. Signed zero and unordered operand selection are retained.
Air writes base, velocity*time+base, then `(HalfG*time)*time+prior` using the
source separate multiplication and fused sums, then conditionally adds world
foot position. `HalfG` initializer `82f810b8` supplies exact lanes
`[00000000,c09cf5c3,00000000,00000000]`.

Contact/Pass validates the base XYZ, builds four Bezier inputs: zero start,
state vector1 with W=`3c888889`, vector2 with W=`duration-update`, and vector3
with W=`duration`. Parameter is raw time when duration is not ordered greater
than zero (including unordered), otherwise rounded
time/duration. `Bezier482c93070` samples its live influence, constructs handle
span `(endSpeed.W-start.W)*influence`, sets endpoint W from endSpeed.W, and
sets handle W values to start.W+span and end.W-span. With `u=1-t`, `u2=u*u`,
`t2=t*t`, its final lane graph is:

```
handle1term = ((handle1*u2)*t)*3
startterm = fma(start*u, u2, handle1term)
handle2term = fma((handle2*u)*t2, 3, startterm)
result = fma(end*t, t2, handle2term)
```

Raw `82c93258..330c` establishes this graph; it is not replaced with a generic
polynomial. The result checks all four lanes. Nonzero result W performs a
primitive reciprocal seed and two residual/refinement FMAs (`82c93784..37cc`).
Raw VNMSUB at `82c937b4/37c4` requires residual
`-estimate.mul_add(divisor, -1.0)`, including its negative rounded-zero result;
it must not become `(-estimate).mul_add(divisor, 1.0)`. It
then multiplies the separately rounded `result*time` by that inverse. It
checks all four lanes again, overwrites evaluated W with zero, adds the
original base (including its W), and conditionally validates/adds world foot
position. VMX128 source-A operands displayed >=64 require raw decoding, not
literal use of the decompiler's register aliases.

Both actual pose-buffer roots are then transformed in their retained
synchronized allocations (`82c97148..725c`): global before local. Rotation
axes use separately rounded first product followed by Y and Z FMAs;
translation uses the old translation as the first FMA addend. Global store
order is Z,T,X,Y; local Z,Y,X,T. The narrow root mutation method does not
replace either allocation or expose unsynchronized values.

After root writes, right then left IK influences become positive zero. If
source history bit8 is set, read the live distance threshold, square it,
compute the XYZ dot of current-minus-prior COM, then use raw BLT semantics:
ordered less skips, equality or unordered continues to the source `200` flag
gate and potential request OR. Store current COM, set history bit8, then
Contact/Pass requires `mFootDown != -1` and writes the chosen IK influence,
target and surface. If no longer in locomotion, clear source mask `26000`
and reset the same trajectory (all four input/physical pairs then scalar
fields, retaining padding). The common validation/copy/flags/Air suffix
follows, including its fixed timer increment.

Additional live cvars, implemented by the graph agent:

| Getter | Source initializer / constructor / static table | Source values |
|---|---|---|
| `try_loco_wipeout_delta` | `82f7fc30` / `82f7fc70` / `82fd4834` | section `Animation_Locomotion`, label `kLocoWipeoutDelta`, title `Wipeout distance delta`, default `3f333333`, bounds 0..2 |
| `try_sa_use_world_y_for_normal` | `82f80ea8` / `82f80ed4` / `82fd48e8` | section `Animation_Locomotion_Collision`, label `kSA_UseWorldYForNormal`, title `UseWorldY Normal`, false |
| `try_bezier_influence_percentage` | `82f810f8` / `82f81138` / `82fd4908` | section `Animation_Locomotion`, label `kBezierInfluencePercentage`, title `Bezier Influence Percentage`, default `3ea8f5c3`, bounds `3d4ccccd`..`3f000000` |

All callbacks are null in raw argument registers. New focused tests cover Air
time mutation/FMA/world offset, live Bezier influence override and base W,
and full locomotion retaining real COM/next-step writes before a missing
body-output failure. These new tests and full method await the root gate;
no Cargo was run by this child during concurrent integration.
