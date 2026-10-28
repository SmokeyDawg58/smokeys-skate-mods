# Reached BlendSpace runtime

September22, root implementation independently source/code reviewed. Confidence:
high for constructor/timing within explicit host allocation and numeric boundaries.
Validation pending first registered gate; not full BlendSpace recovery.

Actual fresh neutral frame2 now selects DataBase.GetAnimTree82d7def0 record6,
after ChannelBlendAnim. Existing Andale parser already retains the source
controller/volume bytes; reuse it by original record offset and retain the same
database/header occurrence. Packed child words are read from that same record,
not re-encoded from display names. No alternate animation database or pose owner.

Push82d71c30 uses native112-byte strict pool check, before Init82d6bf98.
Init retains first volume set/volume; three zeroed vectors have maxDimensions+1
coordinates. Attributes and projected point set currentDimensions coordinate1,
coefficients set coordinate0=1. Clip slots remain unknown until SetClip82d6cc70,
including explicit null child results. Recursive child failure does not trigger
PhaseBlend's different pop policy. Per-volume properties start false/false/-1/-1.

GetLength82d6cce8 visits current-volume indexes, calls each child before reading
its coefficient, and accumulates scalar FMA atcd54. Zero weight does not skip a
child. SetSpeed82d6d020 visits every declared child in declaration order, retaining
earlier changes if a later native null dereference becomes a host typed failure.
SetTime82d6cf88 computes native scalar time/length, calls active children in order
with childLength*ratio, and does not write parent time.

AddTime82d6cd70 allocates scratch before reading children; source rawce48..74
resets property ZERO on every length iteration, not each corresponding property.
Weighted length uses FMAce70. Reciprocalce9c then two distinct scalar multiplies
ceac/cec0 produce child dt. Output is_playing remains untouched; expiry values
are scaled and parent time is derived from first active child before scratch pop.
Failed callbacks preserve the reached property/location prefix. Three focused
tests discriminate these cases and source active-index versus declaration order.

The debug overlay global debugLine=5 at82d6cf60 is explicitly outside this
simulation projection; no claim of debug-state parity. Eval's trimmed early
return82d77314 is preserved. Remaining Eval, parameterization, attributes and
pose-stream dispatch stop with named typed errors, not substituted clip playback.

Further source audit: SetAttributes82d79a00 must retain original matching writes,
then choose source-default optimized3D and asynchronous jobs. Both optimization
(initializer82f8de60/table82fd5628) and jobs(ddc0/5620) default true. Slicing3D
also defaults true(deb0/562c); generic dimensions use the separate algorithm.
RegisterBSJobData82d6b948, Dispatch82d6b9e0, Job82d73520, Wait82d79118,
Post82d79030 must reuse the existing Animatable job/count/code owners.
The full slicing/parameterization kernel is still pending; disabling defaults,
replacing it with ordinary blending or assuming synchronous completion is invalid.
