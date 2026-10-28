# Actual deck target angular displacement

2026-09-12, source-backed **unregistered draft**, confidence high for lane and
branch graph. No Cargo was run. Estimate/reduction host policy remains explicit;
this is not a Xenon bitwise or full force/solver parity claim.

Caller `PhysicsGround::UpdateSkateboard82d9a5d8` follows positive collision
force+torque copies and the live collision-duration store. Outer
`Skateboard::TargetAngularDisplacement82b37e98` only forwards its actual
SkateboardBody; body implementation is **82b361e0..36514**, half-open.
Earlier broad checksum ending3660c included following function bytes; exact
function SHA256 is
`54fbe0fec9cf6b1f9a1df567af589a1c974ff198df957d605636beafe09d79ef`.

## Source algorithm and owners

The original follows Body+18 Assembly, Assembly+8 part list, Part6+28c rigid
body. This is the same real retained deck occurrence already used by force
application. No reconstruction from serialized inertia is needed: current
RigidBodySlot has omega+30, inverse tensor full+70/split+80, torque+a0 and
cooldown+ac. `kUpdateTime@8225c488` is bits3c888889 and is independently loaded
for both divisions. It is not the variable current frame DT.

1. Read omega, multiply by fixed DT, then Math.NormalizeSafe(request,+0)36260.
2. Dot normalized request with angular motion; multiply normalized vector by
   this component. Subtract that projection from requested displacement.
3. Ordered `0 > dot(correction,request)` selects full-known zero; else retains
   correction (unordered retains correction). Then ordered
   `0 > dot(projection,request)` selects the ORIGINAL request; else selects the
   preceding correction/zero. Crucially v62 at362e4 is the already multiplied
   projection, not the former normalized vector. This avoids amplifying the
   requested correction when the board rotates the opposite way.
4. Capture split, oldTorqueX, full, oldTorqueY. Full tensor reload363b0 remains
   separate from its inverse construction. Packed columns: X=[xx,xy,xz],
   Y=[xy,yy,yz], Z=[xz,yz,zz]. W fields are inverse mass/state, not geometric
   values; no cross/dot3 XYZ depends on them.
5. Cofactors use the source existing YZXW negative-FMA cross graph:
   Ycofactor=cross(Z,X), Xcofactor=cross(Y,Z), Zcofactor=cross(X,Y). Source
   first fixed-DT reciprocal estimate occurs363dc, before determinant dot
   at36408. Determinant is dot(X,Xcofactor). Cofactors are transposed into
   inverse columns, then multiplied by twice-refined determinant reciprocal.
   Determinant reciprocal estimate3643c follows first DT estimate/refinement.
6. Selected request * first refined inverseDT36460; multiply by inverse
   tensor with X multiply, Y FMA, Z FMA. Fresh second DT estimate36480 must
   remain even though the value is identical. Multiply inverse-tensor result
   by that twice-refined inverseDT364b4, then multiply by the cached original
   inverse tensor (same X/Y/Z fused ordering). Never algebraically cancel
   these operations or substitute a diagonal-only tensor.
7. Final torque Z = accelerationZ + oldZ, Y = accelerationY + oldY; store Y;
   X = oldX + accelerationX; store X, then Z; cooldown=0. Y/Z operand order
   differs from ApplyLocalTorqueToDeck. No queue append or omega replacement.

All reciprocal residuals use native final negation of rounded FMA. No generic
inverse routine, quaternion or clamping policy is substituted. Numeric errors
occur before the first torque store and leave torque/cooldown unchanged.

## Required narrow shared integration

Draft `simulation_rigid_body_target_angular.rs` is intended as rigid_body_pool
child `target_angular`, with `simulation_rigid_body_target_angular_tests.rs`.
`dynamic_assembly_skater_target_angular.rs` is skater_pool child
`target_angular`: verifies actual backend/member, gets Part6 observation, and
forwards the requested typed vector to the SAME backend.
`simulation_assembly_deck_torque.rs` is a backend child containing only the
two direct pool forwards for this method and the previous local torque draft.
This also supplies the previously proposed fake-torque backend bridge.

One additional shared change is proposed, **not applied**: extract a callback
entry `try_normalize_safe_with_refinement(vector,fallback,numeric,refine)` from
existing Reckoning `try_normalize_safe`; keep the existing wrapper's current
policy. Math.NormalizeSafe824b1988/1998 uses VNMSUB final-negation residual;
the current common drive refinement instead uses negative multiplicand FMA.
The target draft supplies native sqrt refinement through the proposed common
body, preserving one normalization algorithm and avoiding an unrelated global
numerical-policy change. Root was notified before any shared mutation.

Focused unrun tests cover actual omega overspeed→zero, opposing omega→original
request, second DT estimate sampled independently (controlled zero seed
discriminates algebraic cancellation), late numerical failure preserving
torque/cooldown, and non-diagonal cached tensor with absent source Inertia.
Unknown request W and known NaN W never become fabricated retained lanes.
The ordinary collision caller can consume this leaf only after its preceding
retained collision field writes and duration read; the downstream queue order
is documented in `ground-fake-torque-collision-continuation.md`.

## Independent root review before registration

Root reread raw82b361e0..36514 in full, decoded VMX128 VA extensions independently, and checked permutation bytes8225c4d0/c4e0. Projection is already multiplied at36298 before the second dot362e4. The cofactor transpose, three reciprocal calls (DT/determinant/DT), original cached tensor reuse, and Y/X/Z/cooldown stores match the draft. Raw NormalizeSafe824b18e0..19ac confirms ordered positive squared-minus-epsilon gate and final-negation VNMSUB at1988/1998. Root also checked LocalTorque82b35728..357ec full raw and its distinct old-plus-new Y/Z operands. Registration and execution remain pending.
