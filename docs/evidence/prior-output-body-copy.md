# Consecutive prior body output copies

2026-09-12. Source: `PhysicalPlayerHiLOD::Update1_ProcessInput`,
`0x82e01f98`, bounded continuation `0x82e02e60..0x82e0312c` immediately after
the prior-output tail's wipeout impulse store. Confidence **high** from fresh
IDA decompilation and raw load/store/branch inspection. No category test gates
this span. Constructor/reset output is valid input but not proof of a previous
engine publication or a complete frame.

Every row below starts with a fresh PhysOut write-latch check, then its
component pointer load. Preserve that ordering and prior writes if a later
component/destination is unavailable. `try_*_output` animation consumers are
unguarded and cannot replace the new guarded prior-input getters.

| Prior component field | Processed destination | Source load/store |
|---|---|---|
| Skeleton+241 skateboardDontPush | +8c8 mask00000800 | 2e8c/2e98 |
| Skeleton+244 isLeftFootOnDeck | +8d0 mask00002000 | 2ec8/2ed4 |
| Skeleton+245 isRightFootOnDeck | +8d0 mask00001000 | 2f04/2f10 |
| Filtered+0 filteredPhysicsState | +8f4 fullword | 2f40/2f44 |
| OffBoard+38 isGrabbingObject | +8cc mask00800000 | 2f74/2f80 |
| OffBoard+1c timeToLand comparison | +8cc mask00080000 | 2fb0/2fcc |
| OffBoard+3f isHoldingSkateboard | +8d0 mask00040000 | 2ffc/3008 |
| OffBoard+41 onDeck | +8cc mask00040000 | 3038/3044 |
| OffBoard+42 canLandOnSkateboard | +8cc mask00000200 | 3074/3080 |
| OffBoard+1c timeToLand fresh scalar | +a0c fullfloat | 30b0/30b4 |
| OffBoard+43 landOnSkateboardFlipBoard | +8d0 mask00000010 | 30e4/30f0 |
| OffBoard+40 isBoardMountInvalid | +8d4 mask00800000 | 3120/312c |

The flag comparison is not ordered `timeToLand <= 0`: instruction2fb8 is
`40990008`, BO4/BI25, after `fcmpu cr6,f0,f31` with f31=zero. It branches when
GT is false, including unordered. r17=1 is retained on that branch; otherwise
r19=0 replaces it. Thus NaN sets the bit. Scalar30b0 is sampled independently,
not recycled from the comparison, and is copied without conversion or clamp.

The implementation extends the actual `SkeletonProcessedContact` flags and
adds only missing scalar destinations in its existing prior-output child.
Processed Reset82b394b8 stores integerzero to+8f4; Reset82b39580 stores floatzero
to+a0c. These fields are distinct from the later+ a10 timeOffboard accumulator.
No additional Sim padding assumption is needed. Partial-known flags+8d4 retain
their neighboring value/known bits; only written bit00800000 becomes known.

Focused tests cover mixed source booleans/neighbor preservation without a
category gate, positive/negative zero/infinities/NaN with retained scalar bits,
late destination failure preserving the already-written prefix, foreign owner,
write latch, and reset readiness. Tests are drafted; root owns registration,
source review, original corpus integration and Cargo gates.

## Next consecutive dependencies (read-only map)

`3130..33a8` samples wheel contacts using short-circuit OR pairs (2,3), (0,1),
(2,0), (3,1), with independent guards on each reached read. It rereads processed
flip bit+8c4/00100000 before each of four destination bits+8c8/10000000,
08000000,04000000,02000000. These are derived from prior collision output,
not new collision queries. The existing concrete CollisionRemainder owns
the individual wheel flags; a fresh guarded index accessor is sufficient.

`33ac..3500` reads prior State104(skitching) and conditionally samples live
kTimeToAvoidVehicleAfterSkitching, stores HiLOD+168, then guarded State grab
spline objectID+20 to HiLOD+164. Positive retained avoidance time subtracts
processedDT and copies the ID. Ground+114/+118 independently refresh positive
reskitch/repush timers; positive retained timers then subtract DT and publish
the result, without clamping crossing-zero values. Every float BLE here is
BO4/BI25 (NOT GT), so NaN skips updates. All three retained timers and the
vehicle ID already belong to `PhysicalHiLodPrefix`.

`3504..35c8` uses the category captured at Update1 entry: Offboard500 copies
actual Skeleton animation-to-world translation (Skeleton+2d80+30) to retained
HiLOD groundPosAllStates+e0 and adds fixedDT3c888889 to timeOffboard+160. Other
categories call actual board CalculateGroundPos at3560 and zero timeOffboard.
Always copy timeOffboard to processed+a10, then call actual Skeleton
GetEffectiveAnimToWorldTransform82b3ba48 at359c and copy all four columns to
processed+c0. Existing `try_calculate_board_ground_position` resolves the same
board and completed Reckoning phase, but does not perform these HiLOD stores.
Skeleton roots already own the animation transform. The missing effective
animation helper copies all columns FIRST, samples processed+8cc mask8 from
Skeleton+0, and if set multiplies every X/Z lane by -1 (rawbae4/bae8).
This is distinct from the physical board effective-transform helper's flip.

`35cc..36c4` changes retained ground history only when saved category is neither
Air200 nor GroundTrick600 and HiLOD framesSinceJump+128 is positive. Within
that branch it saves the old groundPos, writes current groundPosAllStates to
groundPos at3608, THEN tests processed framesSinceTeleport+928 >1 at3628. Only the
inner branch computes delta and damped velocity. The vector graph uses estimate
and two reciprocal refinements with native negative-fused residuals; numerical
implementation still needs an explicit collaborator and independent raw VMX
operand review. Finally all paths copy retained groundPos+ d0 to processed+1e0,
groundPosAllStates+e0 to+1f0, and groundVelDamped+100 to+200 before PrepareToolkit
at36cc. Do not zero skipped history or turn the outer test into the inner test.
