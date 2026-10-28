# Held-X interruptions: captured Bumped transitions

## Root cause corrected after rebuilt diagnostic capture

The capture at lines2326..2342 shows forward push delta1.5 capped as authored,
deck acceleration[86.97064,0.19858012,-1.9568481], X scale1.5, magnitude130.47078
against threshold100, then1281->276. The error was the consumer's component
selection, not the push force or bump tuning.

Full IDA PhysOut type confirms +38 is mAnimation; +04 is mSkateboardMotion.
IsBumped82c9ce7c loads PhysOut+38, then ce8c reads that component+70.
SetBumpCoefficients82cb1814/1824 uses the same pair of offsets.
Animation+70 is mDeckGroundAcceleration, whereas Motion+70 is the distinct
world-space mDeckAcceleration. The earlier note below incorrectly identified
the outer pointer and is superseded by this complete type/offset check.

Corrected shared try_motion_bump_acceleration to require/read Animation's
existing deck_ground_acceleration. Both consumers now use the native field.
The producer already exists: UpdateDeckGroundAcceleration82e299b0 vicinity
(decompile requested at82e29a7c) transforms to deck space, zeros Y, transforms
back to world, then to ground space and zeros Y, publishing Animation+70 at
82e29b5c..64. Existing conditioner animation update invokes that producer.
No duplicate computation, threshold adjustment, or bump suppression added.

Extended the existing conditioner-output regression with distinct Motion and
Animation values, asserting that the bump accessor returns Animation's value.
Rustfmt parse-only passed. Compilation and runtime verification remain
pending; runtime success is not yet claimed.

A playtest reproduced first/third push interruptions in artifacts/push-interruption.log
(226886 bytes at inspection, September 25 17:51). At lines 1069 and 1282 Motion
leaves push state1281 for state276 Bumped. Conditions308 IsBumped and309
SurfaceSlope are both true. XInput samples retain buttons16384 (X); no grab
trigger input appears at these transitions. No IK, collision-stop, or
BreakOutOfPush diagnostic appears. This identifies the actual exit path, not
yet the cause of excessive acceleration or a verified gameplay fix.

IDA checks this turn:
- SUPERSEDED component identification: IsBumped82c9cd68 was described as Motion+70;
  it actually reads Animation+70 through PhysOut+38 (see correction above).
  The remaining scalar arithmetic was correctly identified: it
  scales X using796a224100aaf608, compares magnitude against2edafe74f922c817.
- Global constructor828b3060..307c: bumps collection7823ada8e7c896aa,
  stored at Global+158; matches existing binding.
- IDA PhysOut_SkateboardMotion type confirms deck acceleration at70.
- Skateboard.FillPhysOut82b2f960,82b2fdb4: CollisionInfo starts Body+40;
  reads CollisionInfo+1d0 = acceleration[6], matching existing adapter.
- SkateboardBody post motion82b36f28..37094: per-part velocity delta / dt,
  stores new velocity; existing host kernel follows this ordering.

Extended existing SKATE2_TRACE_PUSH_INTERRUPTION diagnostics with raw/scaled
bump acceleration, tuning scale, magnitude, threshold and result, plus deck
old/new velocity and dt at the producer. No gameplay arithmetic or thresholds
changed. Rustfmt parse-only passed for both files. No compile or runtime test
performed. The next capture should additionally
enable existing SKATE2_TRACE_PUSH_FORCE to correlate acceleration with push
force. Still needs root-cause investigation; do not report pushing fixed.
