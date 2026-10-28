# SkeletonIK Fill output leaves

Fresh raw and PDB review, 2026-09-12. New unregistered draft files:

- `skeleton_ik_fill_phys_out.rs` and its five-test child, proposed child of
  `skeleton_ik_state` named `fill_phys_out`.
- `skeleton_ik_fill_runtime.rs`, proposed child of `skeleton_ik`; borrows the
  SAME retained attributes, constructor geometry and continuing state.
- `physical_player_phys_out_skeleton_ik_fill.rs`, proposed child of output
  `construct`; implements the sink against its existing Skeleton allocation.

No parent registrations, shared edits, Cargo, or original-data execution have
been performed. The outer Fill draft delegates to this actual leaf via its
required adapter; it does not supply a default or replacement IK output owner.

## Actual containing order

Skeleton.Fill594d8 captures Hips23 before IK.Fill594e4. IK.Fill82b56590 calls
HandGrabInfo82b4a270, FeetDeckInteraction82b53bc0, then guards and writes
IK+c70/+c71 to output Skeleton+241/+242. The final flags use delayed source
closures so a failed guard does not read those fields. A failed child prevents
the following child and final flag stores; previously completed writes remain.

## HandGrabInfo

Loads IK.part_to_deck[2].P at730 and [3].P at770 before validating actual
GlobalAttributeCollection. Raw a2a0 captures the global pointer first, a2a4/ac
load those private matrices, and a2b0 branches around the null assertion.
The immutable-global adapter combines capture/validation after its private
loads: no observable callback occurs between native pointer capture and those
loads, and native singleton replacement races are outside its admission policy.
This does not claim that native pointer capture itself follows the private loads.
Geometry half-width+c60 and total-half-length+c68
are then read; no constructor geometry default is supplied. Form
`[width,0,length,0] + Global.physics_skeleton_default.layout[20..30]`.
Both hands test in Y/X/Z order: strict ordered upper>coordinate, then
coordinate>lower. Lower is upper sign-bit XOR, not a multiply. NaN, equality
and a degenerate extent reject the hand. Packed W does not participate.
Compute both booleans before guarded output+246(left)/+247(right) writes.

Global+ec is a distinct generated instance from Skeleton+24, even though the
class/default match. Fresh constructor proof: allocation828b2940,
generated constructor2954, publication+ec2960. Generated ctor828b10e0 selects
class **9f862c2eaa016ce5**; caller supplies default **d7edbd362d7d2152**.
Absent cached layout gets DefaultDataArea70 at1134/38. Binding that owner once
in actual session bootstrap remains a required parent amendment; no per-call
new global instance or substitution of the Skeleton-owned instance is allowed.

## FeetDeckInteraction

1. Read same geometry width/total-length. Lookup actual IK.mAttribs key
   **77dd00c18946c6cf**, index0 at3c1c, missing-pointer DefaultDataArea16.
   Existing `SkeletonIkVaultAttributes::vector` provides this exact path.
2. From same State.PhysRecord, capture deck P,J,K,I in that order at
   3c60/6c/70/78; read same Processed+93c dt once at3c84. Then capture right
   foot19.P at State+b00 (3ca8) and left foot15.P at State+a00 (3cc8).
   This is the actual physics record, not a fresh GetPartTransform attribute query.
3. Transpose XYZ into rows `[Ix,Jx,Kx,0]`, `[Iy,Jy,Ky,0]`, `[Iz,Jz,Kz,0]`.
   Native merges explicitly produce zero W, regardless of source basis W.
   Inverse translation is ordinary `(0-Pz)*rowZ`, then FMA `(0-Py)*rowY+prior`,
   then FMA `(0-Px)*rowX+prior`. Each foot transforms by rowX*footX+translation,
   then rowY*footY+prior, then rowZ*footZ+prior. Actual source point W is unused.
   Standard VMX raw3d44: A10(rowX), C8(leftX), B9(translation);3d50 A11(rowY),
   C4(leftY), B27;3d5c A12(rowZ), C2(leftZ), B25. Preserve FMA operand order.
4. Capture same last-left-foot deck history3d0c, then retained normal physics
   velocities **15/19**, Skeleton+27c0/+2800, at3d10/14. First dt reciprocal
   estimate3d28 occurs before mutation. Each chain is two native
   `residual = -(seed.mul_add(dt,-1))`, `seed=seed.mul_add(residual,seed)`.
   This is not a scalar divide and has no zero-dt bypass.
5. Form left delta; capture same last-right history3d8c and form right delta.
   Store SAME history members at3d94/98 BEFORE second estimate3dac. A second
   estimate failure retains both updated histories. Compute first deck velocity
   using first refined inverse. Sample first write guard3db8 after second seed,
   then complete second refinement/velocity before checking that guard.
6. Guarded writes: left deck velocity+c0, right+d0, captured world velocities
   +e0/+f0. World velocity W remains its actual known/unknown carrier; computed
   deck W follows native row-zero arithmetic and may include exceptional results.
   No arbitrary full-vector assertion or homogenization occurs.
7. Both foot tests use strict ordered abs(X)<extentX, abs(Y)<extentY,
   abs(Z)<extentZ. Raw left3e9c/3eac are BO4 BI24 (NOTLT), final3ec0 is
   orderedLT. Right3ed8/3ee8/3ef8 are NOTLT. Thus unordered always rejects;
   no decompiler-derived `!(abs>=extent)` shortcut is used. Publish+244/+245.
8. Read live **kDrawContactInfo**, ref8308e780, initializer82f7c468,
   initializer table82fd4540. Section `Physics_Skeleton`, label
   `kDrawContactInfo`, title `Draw Contact Info`, defaultfalse, null callback.
   This reference is not yet registered in the shared host cvar owner.
   Enabled path requires actual DebugOverlay FOUR times, then calls verified
   shared BLR825187c0 at3fd8,4030,4098,4100. Even terminal no-ops retain their
   singleton prerequisites. Disabled path performs no overlay queries.

## Required integration and limits

The new aggregate adapter preserves actual state rather than accepting caller
snapshots of last-foot positions. Parent registration must mount all three
children and supply same normal physics-history records, actual Processed dt,
the retained global+ec capability, live cvar, and existing DebugOverlay slot.
No simulation-owned field is duplicated. Missing geometry/global/cvar/debug
owners fail explicitly at their source access. The containing outer source
trait is parameterized by `N: ReckoningNumeric`, so actual child numeric errors
can be retained in its associated source error without boxing or erasing them.

Tests cover source query/store order, correct velocity domains, unknown source
W versus generated row W, strict NaN/equality bounds, missing global before
output, second-estimate failure retaining both histories, and output rejection
after both estimates/history writes. They are unrun drafts. Host reciprocal
estimate/exceptional payload policy remains the existing explicit numeric
boundary; no Xenon bit-parity or live full-Fill claim is made.
