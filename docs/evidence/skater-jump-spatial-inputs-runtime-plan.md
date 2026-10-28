# Jump spatial inputs: actual Skeleton COM first

Independent read-only source/code audit, 2026-09-08. Confidence **exact** for the raw copies/constants below and **high** for the bounded integration proposal. No Cargo or production implementation was performed. The complete Actor/Update1/Fill sequence is not claimed.

## Actual COM producer and selected output publication

`Skeleton::FillPhysOut` is `82b58230`. Its output argument is retained in r31; Skeleton is r27. The relevant destination is the SAME PhysOut pointer aggregate's **+24 SystemReckoning**, distinct from Skeleton output +14 and skateboard motion +04.

The exact source graph is asymmetric and must remain staged:

| Address | Operation |
|---|---|
| `58578`, `58588`, `5858c` | r29=0x10, r25=0x30, r10=0x3d20 |
| `585ac` | sample aggregate write latch byte +4c |
| `585b0` | **early** full-vector load Skeleton+3d20, COM velocity, into v124 |
| `585b4..585cc` | check sampled latch; assert/debug-break if nonzero |
| `585d0..585dc` | r11=3d00; fetch SystemReckoning pointer +24; **late** load Skeleton COM +3d00; store SystemReckoning **+30** |
| `585e0..585fc` | independently sample/check write latch again |
| `58600`, `58614` | fetch SystemReckoning pointer again; store already captured v124 into **+10** |

The earlier animation-transform/angle operations are omitted stages, not silently satisfied by this selected publication. A first latch refusal occurs after the COM-velocity sample but before COM read/store. A second refusal retains the COM store and leaves COM velocity unchanged. No numeric reconstruction or extra body reads belong in these copies.

Existing actual producer: `SkeletonNormalAssembly::physics_history().record().centre_of_mass()` and `.centre_of_mass_velocity()`, in `skeleton_physics_history.rs` and `dynamic_assembly_skeleton_history.rs`. The retained non-Clone history already computes weighted full-four-lane values from its source Part records, including the actual cvar-controlled COM projection. Do not recompute mass averaging, substitute root/deck position, or create another history owner.

For a live publication, use the existing SAME-pair/backend/attachment checks and require `physics_history_frame_state().progress()==Complete`, with its retained frame Arc identical to the world's actual current completed/exported frame. Progress alone explicitly does not prove freshness after a later solve/body mutation. Reuse the existing world current-frame validation and invalidation path. A selected output publication may record this frame as host provenance, but must not claim the rest of Skeleton Fill completed. Preserve the existing producer's full lane representation; do not manufacture affine W or discard W merely because it is named a Vector3.

## SystemReckoning constructor and Reset

The aggregate allocates the actual 0x90-byte SystemReckoning component at `82e2f4c0/f4c8`, then publishes its pointer at `f544`. Null allocation remains null.

Constructor `f500` sets v62 to all-zero words. r20=0x10, r21=0x20, r22=0x30, r23=0x40. Stores `f50c/f510/f514/f518/f51c` initialize component +00/+10/+20/+30/+40 to **full positive zero**. Later +50/+60/+70 initialize J/K/I respectively. Thus COM +30 and COM velocity +10 both have known-zero constructor provenance, not live publication provenance.

Aggregate Reset `82e2f740` reaches SystemReckoning after Ground Reset `f7b8`. It loads the same component pointer at `f7bc`; r30=0x10 and r26=0x30. The retained full-zero v127 is stored to +10 at `f7dc` and +30 at `f7ec`. The aggregate's final latch-clear remains `f8dc`, after later omitted components. Extend the existing selected constructor/reset stages; do not add a competing component owner or a reset that unlocks early.

## Following HiLOD prior copies

`PhysicalPlayerHiLOD::Update1_ProcessInput`, `82e01f98`, copies COM and COM velocity before the already implemented Reckoning-up copy:

| Source | Destination | Exact guard/read/store |
|---|---|---|
| SystemReckoning +30 COM | processed **+250** | aggregate +2ac fetch2910; latch2914/check291c; System pointer2934; full load2940; store2944 |
| SystemReckoning +10 COM velocity | processed **+260** | fresh aggregate2948; latch294c/check2954; System pointer296c; full load2978; store297c |
| SystemReckoning +50 up | processed +220 | fresh aggregate2980; existing guard/read/store through29b4 |

r26=0x30 at `82e02280`; r24=0x10 at `2274`. Each copy has its own source latch check. Extend `SkeletonProcessedPriorOutputState` on the same processed owner, preserving `PhysOutValue` constructor/Reset/publication origin and existing first-output occurrence binding. Insert explicit omitted-stage callbacks before COM and COM velocity, rather than placing both inside an opaque `BeforeReckoningUp` gap. A late refusal retains earlier successful copies and keeps selected-copy readiness false. Processed Reset separately writes full zero +250 at `82b39318` (r5=0x250), and +260 at `3931c` (r4=0x260); these are not aliases of prepared velocity +2c0.

## Gravity +964

`ProcessedPhysIn::Reset` loads the actual constant **820100b8 = c11ccccd** (-9.8000001907) via `82b3940c/39434`, then stores processed +964 at `39444`. This is a source Reset constant, not a guessed Vault tuning value. Allocation remains unknown until an actual producer runs. A scan of executable aligned instructions with displacement0964 found this as the only direct `stfs ...,+964` writer; this is not proof against every possible indirect alias write. The selected Reset-backed field is viable immediately, with explicit provenance. Do not use +9.8 from prepared-history decay as the jump gravity value: the sign is opposite.

## Retained ground position +d0: next source dependency

HiLOD constructor `82e03cac` sets r4 to stack var_C0; `3cd4/3cd8/3ce0/3ce4` initialize all four words zero, `3d50` loads v60, and `3db0` stores it to HiLOD+d0 (r7=d0 at3d6c). This is known full-zero constructor state, not a terrain contact.

Ordinary input update first produces **+e0 mGroundPosAllStates**. For category OFFBOARD=500 (`3508`), it copies Skeleton animation-to-world translation +2d80/+30 at `3528..3538`. Otherwise it calls actual `Skateboard::CalculateGroundPos` **82b2f1b0** at `3560`, then stores its full result to +e0 at `356c`. No Rust implementation of that board calculation was found. Its source reads the retained Reckoning basis and actual wheel Part body positions, transforms them into that basis, combines their positions and selected height, then transforms back. A ground-line result or current contact position is not an equivalent replacement; full VMX/NaN selection proof is required before implementation.

After the effective animation transform stage, retained +d0 is updated from +e0 only if category is neither AIR=200 nor GROUNDTRICK=600 and HiLOD+128 mFramesSinceJump is **signed >0** (`35cc..35e0`). It loads old+d0 and new+e0, then stores new+d0 at `3608`. A later processed+928 frames-since-teleport gate controls separate displacement/filtered-velocity fields, not the already completed ground-position store. It always copies retained+d0 to processed **+1e0** at `36a8/36ac`; +e0 separately copies to processed+1f0 at `36b4/36b8`.

Therefore the minimal next spatial work is COM/System publication and its two prior copies plus Reset-backed gravity. Ground position requires the actual CalculateGroundPos kernel and category/jump-history producers. Keep its constructor value distinct from a live ground calculation. No full GroundAnimation jump stage should claim live spatial inputs until those dependencies are closed.
