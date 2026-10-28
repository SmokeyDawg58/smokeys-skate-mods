# Grind-in-air adjustment: dependency packet

2026-09-08 read-only author survey following the landing spring.
No implementation or independent full numerical audit is accepted here.
Confidence is high for audited lifecycle, defaults, keys and partial writes;
medium for numerical bodies until operand-by-operand VMX review.

## Source identities and owner

| Function | Span | SHA256 |
|---|---|---|
| Constructor | 82dcf350..82dcf4f8 | 753b5f1f4742077ba2241b2745fb0616b35735012b23641c74071049c9c95b57 |
| Start | 82d991c0..82d992a0 | f57f54f4a6da2eb924b779e161c4269d5122443c9e3d58615933724c30dc9e18 |
| UpdateGrindAdjust | 82dd1518..82dd1678 | 884c2a68b76c3ae9874541857e70cffc581ae0a397b12259dfa9004586815255 |
| Skeleton consumer | 82b4cd80..82b4d194 | 0ef620ff650e39c3fb6c1ea35b091de968f55e83c8033262e62c90ce9fa58573 |
| InitTargets | 82dc9a70..82dca67c | 9a27dbea295d19c692e0d59bb6237022d2652da3cc6cfeb4b224042e0a40e3cf |

GrindAirAdjust is a separately constructed HiLOD-owned0xe90 collaborator shared
with Skeleton and KnownAir. It retains six targets, captured grind input,
started/activated/adjusting,12 predicted decks, five contact offsets,12x5
predicted contacts, five intersection/frame/distance results,12 predicted
angles, two grind frames, accumulated/per-frame position and angle adjustments,
and chosen-contact/type/history. Preserve one authority.

Constructor clears flags, COM history, adjustments, projected deck Z, choice
fields and distances; grind/velocity transforms are identity. It calls
InitTargets. Prediction arrays are not all initialized until their actual
producer stages. Start's six copied target matrices originate in unwritten
caller stack storage, but the surveyed active kernels do not read those copies.
Do not assign fabricated values or claim unused fields fully reconstructed.

## Actual lifecycle

KnownAir::Enter82d9f828..faa8 clears activation/targeting/apex and calls
InitTrajectoryInfo82d992a0..9728. Only an actual TrajectorySelector grind target
causes primitive/orientation capture and Start82d9f9b4, followed by targeting.
Start stores started=true before input copy, clears chosen-valid, writes chosen
and last types=-1, zeroes four adjustment vectors, then reruns InitTargets.
It neither activates nor clears adjusting.

KnownAir::Update82d99820 computes remaining frames from rounded
(collisionTimeC4-timeInStateB4)/processedDt minus AirAlignFrames, with source
fsel lower bound2. Every update reads global physics_airstates key
9d86b4c6ce677cff before the activation flag test. If flagD4 is clear and either
apexD0 is set or remaining frames are ordered-less-than the threshold, it copies
started to activated and latchesD4. Time advances later at99ab8.
UpdatePostPhysics82d99b78 obtains actual board-body velocity and latches apex
on ordered0>velocity.Y. Exit99728..981c clears started/activated at9980c/10
after trajectory/reckoning calls; adjusting remains until Skeleton updates.
An ad hoc activation setter would omit these producers.

## Active graph and services

Input fields: processed deck+40, velocity+190, up+220, angular velocity+2d0,
dt+93c, and captured primitive/orientations. UpdateGrindAdjust does not use its
AnimToWorld argument; the following Skeleton consumer uses the actual roots.

1. Initialize only output.required=false; if activated, read assist-active.
2. flags8d0&08000000 or the big-endian halfword condition corresponding to
   flags8c8&00010000 clears accumulated position/angle, then adjusting=false.
3. Otherwise clear chosen-valid, copy chosen type to last type and initialize
   five frame/distance pairs to -1/0.
4. CalculateGrindTransform, ProjectPoints, SelectContactPoint, then SelectGrind.
5. If chosen-valid==1, UpdateOffset then UpdateAngle.
6. Commit adjusting=output.required after both operations.

Prediction visits2..12 frames and five contacts. Contact selection takes the
first strict plane-sign crossing; exactly-on-plane endpoints do not cross.
Eligibility rejects unordered frame/distance/angle comparisons. Type selection
visits0..5 with the source previous-tipslide preference at2; equal distances
keep the earlier choice. Full intermediate arithmetic still needs raw review.

Existing numerical primitives cover NormalizeSafe824b18e0, ACos824b0278,
SignedAngle829b0628, Angle::Normalize825ece78, combined SinCos82584f60,
standalone Sin824b0060 (currently private in normal displacement) and
ReckoningNumeric::truncate_i32 for predicted frame indexing. No external
collision query occurs inside this active collaborator.

## Lazy attributes and output uncertainty

InitTargets makes28 ordered reads across global physics_grinds_air+A8,
physicsdeck+94 and physicstrucks+DC. Repeated keys remain repeated. Frame-count
key97593d0d4f342621 stores before the2..12 assertion. Contacts are[1,4,0,2,2,3],
angles[90,90,90,0,0,0]; range/distance/factor writes visit2,0,1,3,4,5.
Correction, independently raw-reviewed 2026-09-12: Sin receives deck angle
key00e949f6d567bb10 multiplied by radians constant3c8efa35@823557f4.
82dc9ce4 computes that product;9ce8 overwrites stack90, and9cec reloads
that same stack90 before9cf0 copies it to the splatted Sin argument. The prior
literal-radians interpretation missed this mandatory alias store/reload.

ProjectPoints reads6aa4b4ae2e34a5a5. UpdateOffset mutates per-frame adjustment
before cap06b096eae5923b51, then accumulated adjustment before cap8f02c2c1544320bc.
UpdateAngle copies output direction before18463541b99b6c11. Missing scalar
storage uses DefaultDataArea4. Global grind-air class1cfb42e0240b5b72 binds at
828b2458..248c. The complete28-read key sequence remains to transcribe and
independently review before implementing the target provider.

| Cvar | Reference | Constructor default |
|---|---|---|
| DrawGrindAssistInfo | 830db4d4 | false |
| GrindAssistActive | 830db524 | true |
| ApplyOffset | 830dbcb4 | true |
| ApplyAngleOffset | 830db9d4 | true |

Initializers82f97840/90/e0/7930 prove these defaults. Offset-enabled writes only
offset and required; angle-enabled writes direction, Euler and required. With
exactly one enabled, the Skeleton consumer reads other unwritten outputs.
Represent output fields as explicitly unknown until written and refuse at their
first actual consumption while preserving completed GrindAir mutations. Both
disabled still execute prediction/selection but leave required=false.

Skeleton's consumer performs two combined SinCos calls and builds rotation/
translation using actual roots and the mapped deck pivot. It writes full offset
X/Y/P/Z, full JustSet, first count, second count, then Y restamp/JustSet/count.
Preserve both late count failure prefixes. Existing grind.rs XYZ geometry and
callback-based selection do not implement the retained KnownAir lifecycle.

Proposed implementation split: actual lifecycle/owner/target provider, then
prediction/selection, then accumulated adjustment and the typed Skeleton
consumer. Each needs a complete source packet, separate review and meaningful
original-owner validation before removing the current capability stop.
