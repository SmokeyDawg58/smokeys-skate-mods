# Ground history retains fourth-lane knowledge

Original3 (September20) completed selected consecutive frame0, then stopped frame1
in pre-solve GroundInput(UnknownPositionW). This is an ownership/representation
boundary, not a missing XYZ position or justification for a homogeneous W constant.

## Producer evidence

Attached Part transforms retain XYZ while the packed engine-address fourth lanes
are unknown (dynamic_assembly_part_transform.rs147..169). Processed deck captures
that typed transform; DeckZ/heading feeds retained Reckoning calculation82dd3da0.
The first constructor transform is known identity. After actual frame0 Reckoning,
cross/normalization preserve unknown basis W. Frame1 CalculateGroundPos82b2f1b0
reads those exact current Reckoning columns, rather than constructor defaults.

Fresh raw82b2f3bc/f3c4/f3c8 splats local X/Y/Z only; f3c0 rounds R*x, f3cc adds
U*y with FMA, f3d0 adds A*z with FMA, f3d4 stores the full resulting vector.
Output W depends on basis W but never feeds XYZ. Wheel-position W is not needed.
Offboard Update1 82e03534/3538 instead copies the full Skeleton translation,
which likewise must retain its actual knowledge.

## Consumer evidence and integration

Update1_ProcessInput82e03504..36c4 uses full-vector copies and lane-wise math,
with no W reduction, W splat, W predicate or other W-to-XYZ dependency. Four
PhysicalHiLodPrefix fields now use the existing SkeletonRootVector:
ground_position, ground_position_all_states, ground_position_delta and
ground_velocity_damped. Constructor values remain known four-lane zero.
The narrowing lanes(position) gate is removed; no replacement W is supplied.

Store/sampling order remains position3538/356c, time-offboard3580, effective
transform35b0..c8, history position3608, frame count3624, estimate/refinement,
delta366c, damped velocity368c, and processed copies36ac/36b8/36c4.
Decoded3658 computes new minus old independently per lane. The scalar dt is
splat from Processed+93c lane0. Damping is original0.95, complement1-0.95.
XYZ retains delta*complement, reciprocal*weighted, old.mul_add(damping,velocity).
W uses the identical arithmetic only when its inputs are known. A later known
position does not restore an unknown velocity-history W. Reciprocal estimate
failure still precedes delta publication; frames<=1 skip the history update.
The existing two-Newton reciprocal and negated fused residual are unchanged.

ProcessedGroundPosition and prior_output already retain typed vectors. Existing
collision input consumes XYZ and Air-state input consumes Y; outer-locality
forwards typed data. Consumers that actually require W retain their own boundary.

## Review and validation

Runtime-audit independently reread original raw producer/consumer and accepted
root integration at high confidence. Source-review independently traced the
attached Part -> DeckZ -> Reckoning basis -> CalculateGroundPos route. Existing
known-four-lane, signed-zero/FMA and failure-prefix tests are preserved. Added
regression compares unknown-W XYZ bit patterns with known-W execution and checks
persistent unknown damped W. No native per-tick numeric parity claim is added.
Consecutive1 runtime rerun and full strict gates are pending.
