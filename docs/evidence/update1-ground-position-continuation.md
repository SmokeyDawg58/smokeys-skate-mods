# Update1 ground-position continuation

Confidence: high for selected source branches, stores and numerical graph;
ordinary host reciprocal-estimate/exceptional FP behavior remains an explicit
numeric boundary. No complete Update1 or physical frame is claimed here.

`PhysicalPlayerHiLOD::Update1_ProcessInput`82e03504..36c4 consumes the saved
PhysOut category captured at2008, not the separately retained current state.
Category OFFBOARD500 takes Skeleton's actual2d80 animation-to-world translation,
copies all four lanes into HiLOD+e0, and adds fixed `kUpdateTime_73` to timeOffboard
160. This constant at8236e734 has bits3c888889; it is not this frame's input dt.
All other categories call `Skateboard::CalculateGroundPos`82b2f1b0 and clear time.
The time is published to Processed+a10 at3580.

CalculateGroundPos uses the actual four wheel-body positions and the retained
Reckoning ground basis. Its existing leaf preserves the source mean/minimum and
rotation/FMA order. Native execution does not require a completed Reckoning phase:
the actual constructor82dd7ab8 establishes the ground matrix before the first
Update1. A new explicitly current-owner route reads that retained constructed
matrix, while the existing completed-phase API keeps its original stricter
contract. No synthetic completion token or substitute ground plane is introduced.

`Skeleton::GetEffectiveAnimToWorldTransform`82b3ba48 first copies all four columns
of the retained2d80 matrix, then reads the same Processed8cc word through Skeleton's
input binding. Bit8 multiplies X and Z columns, including W, by -1. It preserves
Y and translation. This is distinct from board-flip8c4. Rawbae4/bae8 are vmulfp,
not sign-bit XOR. The result is copied to Processed+c0..f0 at35b0..35c8.

The history update runs only when saved category is neither AIR200 nor
GROUNDTRICK600 and signed actual HiLOD framesSinceJump128 is positive. It reads
Processed dt93c and writes current all-state positione0 into persistent groundd0
at3608, before testing Processed framesSinceTeleport928. A count at most1 retains
the prior deltaf0 and damped velocity100, but does not undo that position store.

For count greater than1, source3634 samples the reciprocal estimate of dt. The
four-lane delta is current minus the captured previous ground position; its
publication occurs at366c. With damping bits3f733333:

- Compute complement as the separately rounded `1.0 - damping`.
- Compute each weighted delta as `delta * complement`.
- Refine the explicit reciprocal seed twice with the local negative-FMA
  residual/Newton graph. Each residual is `-reciprocal.mul_add(dt,-1.0)`, then
  the refinement is `reciprocal.mul_add(residual,reciprocal)`.
- Multiply refined reciprocal by weighted delta, then use one FMA for
  `old_damped * damping + velocity` on every lane.

Raw3664/3674 and367c/3680 identify the two refinements;3688 is the final FMA with
A=old_damped, C=damping, B=velocity. IDA's VMX128 high source-A aliases require
the same correction documented in the existing numeric evidence; no unexplained
register-shaped implementation is used. The broader shared reciprocal helper
uses an algebraically equivalent negated multiplier and explicitly excludes VMX
exact-zero parity. This local graph instead preserves VNMSUB's negated fused
result: dt2/seed0.5 produces residual negative zero. Finite nonzero refinements
normally reconverge; no final velocity difference is claimed from that fixture.
Host estimate, denormal, and NaN-payload behavior remain explicit limitations.
Numeric failure at the estimate preserves
the earlier ground-position store and stops before delta/damped publication.

All categories then publish HiLODd0->Processed1e0 at36ac,
HiLODe0->Processed1f0 at36b8, and HiLOD100->Processed200 at36c4. The existing1f0
owner is reused as a single vector; no PhysOut-origin metadata is fabricated for
this HiLOD producer. Execution stops before PrepareToolkitInput36c8.

HiLOD's existing four-vector storage has full known lanes. An unproduced W from a
typed Skeleton translation is an explicit error before that HiLOD store, never
silently replaced with zero. The effective-matrix payload can retain unknown W.
Processed Reset writes c0 identity at391a8..b8,1e0 zero at39244,200 zero at39254,
and timeOffboard+a10 zero at39584.

Focused tests cover four-lane mirroring with preserved unknown W, lazy reciprocal
sampling across the teleport boundary, failure before delta publication, and
four-lane damping arithmetic, exact-zero residual sign, and a final fused versus
defused reciprocal discriminator. Root owns workspace and actual-corpus gates; this
agent ran formatting and diff checks, not a parallel Cargo process.
