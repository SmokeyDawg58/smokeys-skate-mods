# Private Hall of Meat impact/scoring bridge repair

Confidence: high for the omitted contact input and authored field selection;
no claim of complete Skate 3 Hall of Meat runtime parity.

S2 SkeletonCollision FillAudio projects the pose-derived per-part velocity
change onto each main/specific contact normal and takes its absolute value.
HoM UpdateBones82df5980 selects that acceleration when
kUseBoneAccelerationForForce is true. The S3 bridge forwarded force but left
both acceleration fields at zero. Export the existing S3
SkeletonPhysicalRecord.velocity_changes and project the first 24 physical
parts, retaining the existing separate face/neck and groin/hips contacts.
Do not divide by dt again or fabricate a force from speed.

The embedded S3 collection Hash_9E4C471BC6EAB615/default contains 25 authored
bone rows of 84 bytes. Correction2026-10-26: the original adapter wrongly assumed
shared fields kept their offsets. S3 inserts six curves. Match stable field hashes:
S2 offsets2100/2180/2260/2340/2420/2500 map to S3 offsets2420/2500/2580/2820/2900/2980.
The previous endpoint test repeated that same wrong mapping and did not establish semantics.
Select the corrected S3 thresholds,
link weights, bone points and PointNegGraphData8 samples only for S3 scoring.
Scalar lookup uses the same S3 default collection. Native S2 mode is unchanged.
Data remains in the external embedded asset pack, not hardcoded tuning.

Scope limits: scoring lifecycle/aggregation and body-tweak/entity tables still
use the host implementation. The native S3 wipeout control/profile/response
runtime is already active. Its native bounce limit must not be removed and
called parity without reference evidence. Complete cross-game challenge and
ragdoll parity remains unverified; focused evidence must name what was tested.
