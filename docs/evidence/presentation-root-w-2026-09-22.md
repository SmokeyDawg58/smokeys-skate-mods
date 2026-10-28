# Root-position W transport admission

Original frame0 reached `Missing("known root position W")`. This is not proof
of an omitted physical constant. Confidence high for the following raw dataflow.

## Actual unknown producer and stores

Skeleton UpdateRootTransforms82b42938 calls actual velocity getter42a70, then
full-lane velocity*dt+position at42aa0/42aa4. Its velocity W carries an intrusive
native link and remains semantically unknown in the host projection. The affine
translation composition42f24..42fbc retains that P.W; normalization43154 copies
the position vector. No W=0/1 store was found on this route.

Skeleton Fill585a4 loads root P and585a8 stores it into the same PhysOut Skeleton
root matrix. Submittable GetRootMatrix825f04a0 returns that retained matrix.
WriteData loads position at827dc314 and stores header+90 at827dc328. Earlier
packet comment saying c324 was the position store was incorrect: c324 stores
an unrelated quaternion scratch constant; this patch corrects that comment.

## Proven consumer liveness

ReadSkater827ddb60 reads local hips/board and the relative compressed pose, never
header quaternion/position128..160. UpdateSkater827d9e10 also does not use incoming
position W in any cross-lane reduction:

- Same-header branch: a13c loads position; a124 establishes all-lane1.0; a168
  vrlimi mask1 replaces W; a16c publishes position with known W=1.
- Distinct-header branch:9e74/9e7c load the two position vectors;9ffc subtract and
  a040 fused interpolation are lane-wise; a074 replaces W with proven1.0 before
  a07c publication. Quaternion reductions operate on separate root quaternion
  vectors loaded9e64/9e68, not these position vectors.

The native packed bytes may therefore contain unknown W while these consumers'
observable pose values do not depend on it. This does not license setting the
physical producer W to1 or treating arbitrary unknown lanes as harmless.

## Bounded implementation

`RootPositionWTransport::RequireKnown` retains the old strict boundary.
`ReadSkaterOpaque{bits}` is explicit host admission only for the proven reader;
original-data caller supplies deterministic bits0 as an opaque carrier. Known
source W always wins and its exact bits?including signed zero/NaNs?are preserved.
The root XYZ and quaternion code/sample order are unchanged.

Each `SubmittedPresentationFrame` owns the actual gathered-frame generation and
a retained byte-owner identity token, and an ordered per-header Vec<Option<u32>> of source W knowledge. This metadata moves
with successful submission and is retained across subsequent frames. Its typed
root-position accessor first validates the exact gather generation and header
bounds, then returns the actual XYZ plus source Option W. It NEVER decodes opaque
carrier bytes as a known W. Future consumers must require known_lanes or prove
native W overwrite. Foreign-owner and recycled gather descriptors fail before reading metadata. Compressed-view admission
also requires the relative offset to lie beyond every remaining header, so a
wrapped/malformed offset cannot turn an opaque header lane into decoder input.

The producer also retains opaque byte ranges for each gather allocation. Successful
writes clear only the overwritten prefix; current unknown header W ranges are
recorded exactly. ReadFrame checks the actual Q4/Q3/T window plus the native
two-byte lookahead against these ranges. Thus a shorter later frame cannot turn
a previous opaque header lane in retained tail memory into live decoder input;
that case stays a typed unknown-source boundary.

No original-data bytes, concatenated decoder padding, physical W constant or
full-frame claim are introduced. Native compression remains outside the existing
explicit uncompressed host admission.

## Validation

Focused tests check strict rejection, explicit opaque carrier with None retained,
known signed-zero bits overriding a different carrier, typed unknown root reads,
header index bounds, foreign-owner rejection, opaque-tail lookahead rejection,
and stale generation rejection for the metadata-bearing frame.
Original frame probe compares typed W knowledge against the SAME current PhysOut
root source before consuming the frame. Author patch application/format parsing
checked; Cargo and original-data validation remain root-owned.
