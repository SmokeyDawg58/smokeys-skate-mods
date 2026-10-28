# Skateboard motion publication and prior-output input copies

Read-only original executable/PDB audit, 2026-09-08. This packet proposes a bounded retained output owner; it does not implement publication or claim a complete HiLOD frame. No Cargo was run for this audit.

## Original anchors

Addresses below are original executable addresses. SHA-256 covers the original function bytes, end exclusive.

| PDB function | Span | SHA-256 |
|---|---|---|
| `Skateboard::FillPhysOut` | `82b2f3e8..82b3076c` | `8c27321995ce79faf4d251aaf9c5aeac49182b7d076084730d04572b282f18c7` |
| `PhysicalPlayerHiLOD::Update1_ProcessInput` | `82e01f98..82e03be4` | `599ffd6046e32187a8142dd9f49fa2eb850d97ec9dbc3bfb0b36c1f0f519eccc` |
| `PhysicalPlayerHiLOD::Update5_ProcessOutput` | `82e04ea8..82e06234` | `46fcbd20ae90e7b6d33d1516ac348a2984f338768e4d2a97da90775a5c289fae` |
| `PhysOut::PhysOut` | `82e2f2a8..82e2f740` | `ecc42605dbd136b4ca9bb9b3c0237cf5563e3608ae0c19eb362bffd411525a44` |
| `PhysOut::Reset` | `82e2f740..82e2f8f0` | `d3f61933884611155253cb543234a012adfe01b1dad382bca2a9061f6b0805c7` |
| `PhysOut_SkateboardMotion::SetResetValues` | `82e251e8..82e252d8` | `78434c2209826c370caf99dd9020a21d523de1fbddd6e99d889f5e6da7b4bb91` |
| `PhysOut_Ground::SetResetValues` | `82e250c8..82e251e8` | `22c29a499d6d99a94000296c4e9dd31c45a898ac5b1efeab3d3138c07cbf94b2` |
| `Reckoning::FillPhysOut` | `82dd4150..82dd4348` | `22ca59600dca0534980a5372c9fd1533708133d1f263128a812e8fbddc76b767` |

## Ownership and phase timing

The inspected route has ONE retained `PhysicalPlayerHiLOD::mPhysOut` pointer at `+2ac`. HiLOD construction allocates a `0x50` byte `PhysOut`, calls its constructor at `82e03f20`, and stores the pointer at `82e03f38`. The same pointer is supplied to the conditioner constructor (`82e03f54/58`) and SurfacePhysics (`82e03f8c/90`). These operations precede the actual Skateboard constructor call at `82e0413c` and Skeleton constructor call at `82e041c8`. A diagnostic that constructs selected output components after existing pair admission must explicitly retain that chronology limitation; allocation of an unknown host occurrence is not native component construction. There is no output pointer swap in the inspected Update1/Update5 route. A new double-buffer owner would not follow this source proof.

PDB `PhysOut` is a pointer aggregate: skateboard reckoning `+00`, motion `+04`, ground `+20`, system reckoning `+24`, and `bool mIsLockedForWrite` at `+4c` (other output component pointers occupy the intervening words). Its constructor individually allocates components, including motion `0x120`, ground `0x130`, and system reckoning `0x90` bytes. Motion allocation is `82e2f30c`, component reset `82e2f31c`, pointer store `82e2f328`. Null allocation leaves that component null; it does not authorize a synthetic replacement. The final constructor lock store is false at `82e2f730` (`r19=0`).

`Update1_ProcessInput` reads the retained previous publication, then sets `mIsLockedForWrite=true` at `82e03ae8` (`9a26004c`, `r17=1` established `82e020b8`). This happens AFTER the processed-input copies, toolkit, Skeleton processing and further input work. Mutable component getter code checks that this flag is false and asserts/breaks otherwise. The source flag is a write-protection latch, not evidence for a mutex or a buffer index.

`Update5_ProcessOutput` starts by resetting that SAME owner (`82e04ecc/ed0`). Reset runs the component reset sequence and only clears the lock at its END, `82e2f8dc` (`9bf8004c`, zero `r31`). Thus a fallible reconstruction must not clear the lock eagerly before a failed reset prefix. The source writes reset data directly rather than going through the guarded component getters. Update5 contains no subsequent store setting this latch true. The next Update1 eventually does that after its input work.

Update5 publication call order starts:

1. `PhysOut::Reset` at `82e04ed0`.
2. `FillPhysOut_SkeletonLineTestResults` at `82e04ed8`.
3. `Skateboard::FillPhysOut` at `82e04ef8` (the sole direct caller found).
4. `Skeleton::FillPhysOut` `82e04f18`; Pumping `4f24`; GrindManager `4f30`; TrajectorySelector `4f3c`.
5. `Reckoning::FillPhysOut` `82e04f48`; StateChanger `4f54`; HandPlant `4f60`; FootPlant `4f6c`; SkateboardController `4f78`; Wipeout `4f84`.
6. Further state/output work, state virtual FillPhysOut at `82e05b74`, surface check `82e061ec`, intents `61f4`, AnimOut FillPhysOut `6200`, AnimOut Reset `6208`, update-count increment `6214`.

These are staged calls, not an atomic snapshot transaction. Failure of a later operation retains earlier reset/publication writes. Full external scheduler dispatch and conditioner invocation timing remain outside this bounded packet; no runtime buffer flip is established or required by the observed owner route.

## Constructor/reset values of the bounded fields

Motion PDB offsets: angular velocity `+40`, deck velocity `+50`, skateboard surface velocity `+60`, deck acceleration `+70`, empirical acceleration `+80`, empirical velocity `+90`, deck speed `+a0`, surface speed `+a4`, Z speed `+a8`, surface angle `+ac`. Reset initializes those vectors to all four positive-zero words and those scalars to positive zero. Teleport transform `+00` is identity; wheel hardness `+c8` is `1.0`; wheel velocities `+d0,+e0,+f0,+100` are full zero. The remaining motion flags are false and motion tuning/output scalars are zero.

Ground reset creates identity ground transform at `+00`, and full `gJVector` at dynamic up `+40`, ground up `+50`, and last-valid ground up `+60`. These are source reset values, not a license to substitute an up vector when a later real publisher is missing. System reckoning reset writes `gJVector` to up `+50`, `gKVector` to heading `+60`, and `gIVector` to right `+70`; preceding vector slots are zero. A bounded owner must distinguish constructor/reset state from a completed post-engine publication.

## Motion producer and exact important writes

`Skateboard::FillPhysOut` gets the real assembly Part 6 rigid body (`82b2f43c`, part stride/address proven by PDB `m_partList[6].m_body`). It performs body position and repeated effective-transform calls and output writes before the motion subset. The linear velocity is sampled from that retained rigid body `+20` at `82b2f550`; angular velocity is sampled later from `+30` at `82b2f718`. Debug velocity/look-direction cvar and overlay checks occur before the first motion write. Do not move these reads or declare the complete FillPhysOut recovered merely by implementing its motion subset.

1. Guarded angular-velocity full-vector store at `82b2f740` to motion `+40`.
2. After the next component guard, dot3 of retained velocity with itself at `82b2f764`. Full velocity store to motion `+50` is `82b2f788`, BEFORE reciprocal-square-root estimate `82b2f794` and the length refinements. A dot failure precedes the velocity write; an estimate failure follows it.
3. Length graph is the existing source two-refinement reciprocal-square-root graph: `q=dot(V,V)`, estimate `r`, twice `r = (r*0.5).mul_add(1-q*(r*r), r)` with the source negative-multiply-add error operation and rounded intermediate products, then `q*r`; select full positive zero when `q == 0`. Use the existing `reckoning_numeric::length_from_squared_and_estimate` graph, not host `sqrt`. Guarded scalar deck-speed store is `82b2f7f8` to `+a0`. The lock is sampled at `f78c` before estimate/refinements; assertion is reached after those calculations at `f7d8`.
4. Pump, flags, wheel/collision/below-deck outputs intervene. Surface projection begins at `82b2fdf8`: load motion component, Reckoning `Skateboard+104`, then its ground normal `+1f0` (`82b2fe1c`), and previously published motion velocity `+50` (`fe24`). `dot(V,G)` at `fe2c`; rounded product `G*dot` at `fe3c`; full-lane subtraction `S=V-product` at `fe40`, local store `fe44`.
5. Load current processed physical deck Z (`Skateboard+110`, processed `+60`) at `fe48`, dot `(Z,G)` at `fe4c`, rounded `G*dot` at `fe54`, subtract at `fe58`, local projected Z `P` store `fe5c`. This is the retained physical deck Z, not the toolkit flattened Z direction.
6. **A real NormalizeSafe call has an unused result.** `fe34` (`38610090`) supplies output stack `+90`, while input is stack `+80`; call is `fe60` (`4b981a81`). The subsequent code explicitly reloads stack `+80` at `fe74/fe8c` (`38a10080`/`13e028c7`), not result `+90`. Preserve the numeric call and its failure point but retain the raw projected vector for subsequent arithmetic.
7. Dot `(P,S)` at `fe94`; ordered `>0` keeps P, otherwise all four sign bits are XORed (`fec8/cc/d0`). Equal and unordered take the negation path. Copy the fresh Reckoning ground transform `+20` at `fed4..ff28`; its Y column supplies wind axis. Call existing signed `AngleBetweenVectors(Pchosen,S,wind)` at `ff2c`.
8. Wrap angle using vector **floor** `vrfim128` at `ff5c` (`19606335`): `t=angle*g2PIReciprocal`, `fraction=t-floor(t)`, subtract `1.0` iff fraction ordered `>0.5`, then multiply `g2PI`. Do not replace with the existing truncation-based scalar angle normalizer. Constants are the actual vector symbols `g2PIReciprocal=8227e940` and `g2PI=8227e930`.
9. Surface-velocity full-vector publication `82b2ff9c` to motion `+60` occurs AFTER angle evaluation/wrap and its component guard. Surface angle scalar `+ac` is stored `ffdc`. Only then surface length dot/estimate/two refinements/zero selection runs (`ffe4..30028`), followed by guarded surface-speed scalar store `82b3004c` to `+a4`.
10. Wobble effect/tilt and deck tilt publish before a fresh Part 6 `Body::GetPosition` at `82b300e8`. Read processed flip mask `+8c4 & 0x100000` at `300f0`; if set, XOR sign bits of the fresh physical X/Z columns. At `30170` reread the already-published motion velocity. Dot `(velocity,effectiveZ)` is `30174`. Guarded scalar Z-speed publication is `301a0` to `+a8`. Do not compute Z speed eagerly alongside deck speed from the earlier transform.

VMX128 disassembly names can alias encoded high registers (for example `v55` producer / `v87` consumer); the dataflow above follows the actual same encoded register values rather than inventing distinct temporaries from those spellings.

## Ground normal is a separate contact producer

Later in the SAME Skateboard Fill, dynamic up is written to Ground `+40` at `82b302f4`. Two distinct virtual `GetSkateboardBody` calls then sample body `+50`: first `82b30304/30c` publishes Ground `+50` at `30334`; second `30344/34c` publishes Ground `+60` at `30370`. PDB identifies SkateboardBody collision info at `+40`, and its `lastValidGroundNormal` at relative `+10`. Therefore BOTH output vectors are sourced from the last-valid contact normal, with separate reads. Do not substitute averageGroundNormal at Body `+40`, or Reckoning `+1f0`.

Its missing current-frame producer is `SkateboardBody::UpdatePostPhysics` at `82b36a58`, called by `Skateboard::UpdatePostPhysics` (`82b37d20..82b37e90`) at `82b37e5c`. HiLOD `Update4_PostEngineAdjust` invokes Skateboard post-physics at `82e01cac`. The body method uses actual per-part contact processing and selected surface data; the selected-surface chooser is called at `82b3709c`. The wheel-normal sum path requires positive wheel count and squared sum ordered greater than `0.000015258789` (2^-16), performs two reciprocal-square-root refinements and writes normalized average at `82b37378`, then last-valid at `82b3737c`. Its fallback updates average ground normal from deck/truck contact normals but does not write last-valid in that tail. Exact complete contact accumulation is a separate missing implementation dependency, not reconstructed here. `CollisionInfo::Reset` (`82b23630`) initializes both average and last-valid to `gJVector`.

`Reckoning::FillPhysOut`, called later at `82e04f48`, instead copies Reckoning up `+1b0` to SystemReckoning `+50` (`82dd4180/41ac`), heading `+1e0` to SystemReckoning `+60` (`41b4/41dc`), and later the full ground transform to Ground `+00`. It does not produce Ground `+60`.

## Next Update1 consumes prior publication

All source-to-processed pairs below are direct load/store copies, each reached after the source component getter's lock check. They precede toolkit `82e036cc` and must remain retained input state rather than live reads from the current body.

| Prior output field | Processed offset | Source load / store |
|---|---:|---|
| Motion deck velocity `+50` | `+190` | `82e02638 / 82e0263c` |
| Motion surface velocity `+60` | `+1a0` | `82e02674 / 82e02678` |
| Motion Z speed `+a8` | `+944` | `82e026dc / 82e026e0` |
| Motion surface speed `+a4` | `+96c` | `82e02710 / 82e02714` |
| Motion deck speed `+a0` | `+968` | `82e02744 / 82e02748` |
| Motion angular velocity `+40` | `+2d0` | `82e02780 / 82e02784` |
| SystemReckoning up `+50` | `+220` | `82e029b0 / 82e029b4` |
| Ground last-valid up `+60` | `+1d0` | `82e02a24 / 82e02a28` |

Other source work intervenes between these copies. A bounded continuation should expose exact staged getters and retain already-written fields if a later lookup fails. The existing same-owner `SkeletonProcessedContact` plus its toolkit substate should consume these retained values; a parallel competing processed owner is unnecessary.

## Minimal integration proposal and remaining boundaries

Add one occurrence-bound PhysOut component owner to the actual physical-player pair, with source constructor/reset fields and an explicit write latch. Scope it as selected components until the remaining output components exist. Preserve partial reset and publication states; do not present reset defaults as a successful engine publication. The existing owner does not yet contain native PhysOut storage (`physical_player_ik_complete_corpus.rs` explicitly labels its inputs as fixtures).

Reuse the actual assembly Part 6 occurrence and existing backend raw XYZ velocity/position access, existing `SkeletonRootVector` unknown-W representation, existing effective-transform graph, shared length/NormalizeSafe/signed-angle numeric services, and existing Reckoning retained owner. Current rigid-body services prove XYZ; preserve unknown W unless its actual producer is recovered. Add the floor wrapping primitive only after matching the vector graph above; existing `try_normalize_angle` has different rounding semantics.

Implement staged subsets with explicit continuation boundaries for the intervening FillPhysOut producers/debug checks, rather than moving all motion calculations to function entry. The prior-output transfer can then be an exact same-owner extension into the already implemented processed/toolkit path. Before claiming an actual cyclic grounded route, close the board post-physics contact owner (especially lastValidGroundNormal), the output phase scheduler/conditioner boundary, and the remaining source gates needed to reach the selected publication stages. No keyboard/controller data can stand in for those outputs, and no DMO work is required for this motion ownership proof.

Integrated validation update: checkpoint24084/31b724 PASS; workspace3266 units
(3023core), sixdocs, originala15c4c eleven PASS, strictClippy/fmt/diff PASS.
Independent source/code/owner/corpus review passes. Selected
component/collaborator and controlled-schedule limits above remain explicit;
this does not establish a native full HiLOD frame or a playable app.
