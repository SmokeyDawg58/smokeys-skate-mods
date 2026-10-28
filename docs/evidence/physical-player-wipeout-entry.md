# Retained WipeoutGround entry and body velocity prerequisites

Date: 2026-09-12. Author: ground_path. Independent root
review, registration, and integration gates pending at authorship.

## Source and scope

Connected IDA debug-build decompilation, raw PPC disassembly and PDB layouts:

- `PhysState_WipeoutGround` constructor82d9fcb8 and Enter82d8eb18.
- HiLOD allocation/call82e04864..4890, publication to+270 at82e0489c,
  between Air and Ground state construction.
- `SkeletonBody::ScaleVelocityAlongVector`82b3c000;
  `ClampVelocityMagnitude`82b3c160 (Skeleton wrapper82b45de0).
- Single-argument `SkateboardBody::SetForceTurnAngle`82b24cd8;
  `HookDisableAngularDrive`82b24c98 and Linear82b24cb8.
- Processed Reset82b39200 and Wipeout request read82d8ec38.
- Wipeout cvar initializers82f8fad0/82f8fb30/82f8fb90.

This entry uses the same retained state, pair, request ledger, board hook,
truck drives, Skeleton current animation, hip hook, collision controller and
actual backend Parts. It does not itself select or publish the Wipeout state.
The outer source transition and actual request producer remain integration
requirements; diagnostic invocation is not continuous bail physics.

## Constructor correction from raw stores

PDB size0x150: attributes+20, four timers+30..3c, direction+40,
impulse+50, body angular velocity+60, flags+70..73, timeOver+74,
drive weights+78/7c, impulseFrames+80, respawn flags+84..8a,
collision prediction+90..13f, estimated time+140.

An earlier decompiler-only reading incorrectly left direction, body angular
velocity and trajectory time unknown. Raw constructor proves their defaults:

- fd4c loads -1; fd54 stores stack+50; fd90/fd94 splat it into the
  prediction trajectory's time vector, so all four lanes are -1.
- fd5c loads zero. fd64/fd6c/fd74/fd78 initialize all of stack+60..6c,
  then fda4 writes the complete direction vector at state+40.
- fd7c/fd80/fd88/fd8c initialize all of stack+70..7c, then fe0c writes
  the complete body angular velocity vector at state+60.

Both latter vectors are full positive zero. The prediction's position,
velocity and gravity are also zero. Prediction readiness is false;
estimated time is FLT_MAX. Prediction result fields remain unknown, as do
state impulse+50, impulse flag+73 and impulse frames+80. These cells are not
invented as zero. The retained prediction uses the identical shared layout
already proved for FootPlant, with an independent value and readiness bit.

## Entry ordering and partial effects

Enter clears actual Skeleton truck tilt at eb64 before local prefix stores
eb70..ebb8. The prefix resets the documented timers/flags/weights and body
angular velocity, preserving direction, impulse and late suffix cells.

Calls occur in source order: actual SetWipeout ebc0; single-argument force
turn zero ebdc; angular hook disable ebf4; linear hook disable ec0c; disable
IK ec14; angular hip soft0.5 ec24; linear hip soft0 ec30.

The simple turn overload copies the angle to front/back and sets forceTurn.
It preserves both contact timers, unlike overload82b24cf0. Both hook disable
methods mean HARD type2 followed by zero spring/damping/strength, not type0.
Existing hooked dynamics Arcs are mutated so downstream solver consumers see
the same changes. No replacement detached hook payload is created.

The direct request6 (HeavyLanding) read at ec38 uses the actual request ledger.
Constructor/Reset do not initialize those arrays. Missing initialization is
an explicit error after the source prefix, not an implicit false request.
Actual successful Update2 clear establishes the ledger. When request6 is
true, ec60/ec64 capture cvar targets; ec68 reads upper value before ec6c
reads lower. Direction is the same Processed up vector at+220. A fresh max
velocity cvar read follows, then the mandatory Part clamp at ec9c.

Processed+8c4 bit3 branches to the LivingWorld impulse path. A known false
flag continues; unknown stops explicitly. A true flag currently reports
PendingLivingWorldImpulse after preceding effects. The live PushRagdoll
displacement path must be reconstructed before that branch can complete;
parking spawned world objects does not authorize dropping a real impulse.

The supported false branch executes actual controller Ragdoll8 at ed5c and
Skeleton linear drag0 at ed68 (all24 Parts through the existing drag owner).
It then copies Processed+8c8 bit13 to impulse flag ed84, the same Processed
wipeout impulse+330 to local+50 ed88, impulseFrames0 ed90, estimatedFLT_MAX
ed94, and prediction readinessfalse ed98. The impulse representation retains
packed-W uncertainty instead of fabricating a full vector.

## Actual body velocity arithmetic

ScaleVelocityAlongVector82b3c000 visits Parts1..14 with upper scalar and
Parts15..23 with lower scalar, in ascending order. Root0 is untouched. For
each real Part: dot3(direction,velocity), projected=direction*dot,
residual=velocity-projected, result=FMA(projected,scalar,residual). Direction
is not normalized. The existing actual XYZ setter preserves packed body-link
W and invalidates the body's affected frame certificate.

Clamp82b3c160 asserts all four max lanes positive at c1ac..c1d8 before body
reads. This caller splats one scalar, so a single ordered scalar comparison
is equivalent. It squares max, then visits Parts1..23 ascending. Only ordered
dot3(v,v)>maxSquared enters the store branch; equality and NaN skip it.

Raw numerical graph for the entered branch:

1. c21c reciprocal-square-root estimate seed r.
2. c248 square r*r; c24c half r*0.5; c250 VNMSUB residual
   `-squared.mul_add(r*r,-1)`; c254 FMA `half*residual+r`.
3. c258..c264 repeat that refinement. c268 multiplies squared by refined r.
   c26c selects positive zero when squared equals zero.
4. c270 reciprocal estimate q of length. c274 residual
   `-q.mul_add(length,-1)`; c278 FMA `q*residual+q`.
   c27c/c280 repeat reciprocal refinement.
5. c284 multiply reciprocal by max, c288 multiply velocity by that scalar,
   c28c restore packed W from the original, c290 store.

VNMSUB is the final-negated fused expression; replacing it with a differently
signed FMA can change signed-zero behavior. The injected ReckoningNumeric
boundary retains the established host estimate policy; no new claim of Xenon
estimate-bit equivalence is made. Real body reads/stores happen per Part,
so a late unavailable Part or numeric failure leaves earlier effects intact.

## Reset provenance and checks

Processed Reset3926c establishes up+220 from gJVector; wipeoutImpulse+330
uses the zero vector (pointer392cc, store39378). Both are marked Reset, not
live PhysOut/Update1 output. The request getter distinguishes unconstructed,
constructed with unknown arrays, initialized false, actual requested true,
and post-clear false.

Focused tests cover raw ctor known/unknown cells, late entry cells preserved
by the prefix, simple-turn signed zero and untouched NaN/contact timers,
unnormalized scaling, and clamp equality/NaN/store arms. The real ragdoll
corpus compares the complete actual request ledger before/after to ensure
ragdoll itself does not mutate it. Root owns all Cargo and corpus gates.

## Producer dependency audit

There is no Physics Wipeout Update method. Actual state post-physics methods
schedule the detectors: Ground82d8e9b4 calls CheckForGroundWipeout82de09c0;
Air82d8b2d0 calls CheckForAirWipeout82de10b8 with useCOM=false.
The latter calls CheckForBadLandingWipeout82dd9528 at82de16f4. Its ordinary
landing branch is the concrete HeavyLanding6 writer: flag98cc, impact value
98d0, counter increment98dc. This requires actual board contact results,
original global wipeout tuning, prior velocity, and trajectory-selector
targeting state; preceding tangent-angle/PointGraph processing also executes.
It is not a controller-button or animation request6 writer.

HiLOD IsWipeoutRequested82e00a50 first checks IsRunoutRequested82dffe88,
then the actual enabled request ledger, then Processed+8d0 bit6 animation
request. Update5 calls it82e05438 and writes PhysOut.State+38 at546c.
Update4 calls it82e01d68 between state post-physics and Skeleton's
PostWipeoutCheck call; its return is the otherwise hidden bool argument.

The state selector's Processed+8c4 bit18 has a separate direct producer:
Update1 raw82e02384 `rlwimi r4,r5,15,13,13` copies prior bit3 into bit18.
Later Skeleton.ProcessAnimAttributes82b59de8 matches actual WipeoutAtt:
raw82b5adb8 loads+8c4, adbc ORs0x40000, adc0 stores it. The final suffix
ba0c/ba10/ba14 also ORs this bit when AnimOutPhysIn livingworld impulse is
active. Thus a selector fixture toggling bit18 cannot prove input-driven
bailing. A full runtime path still needs actual Motion attribute output,
GetPhysUpdateData/AnimOutPhysIn publication and ProcessAnimAttributes, or
the real post-physics collision detectors and resulting animation feedback.
