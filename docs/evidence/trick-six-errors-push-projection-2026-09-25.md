# Six runtime gaps and push-foot projection correction

Source fixes only.
No build or runtime test was performed for this batch. Rustfmt parsed the
edited standalone modules. Added expression-order and foot-projection regression
tests have not been executed.

## Runtime gaps

- Motion callback684 / 82cb0ef0: ChooseRandomSketchyLanding.Begin. Fresh IDA
  confirms GetRandomInt825fe1a0 -> RandValue82a6dbc8 on the retained Motion RNG,
  signed absolute value, remainder5, decimal construction attribute. End82cb0e18
  removes that same loaded name. Update825187c0 is empty. Reuses existing
  NativeSkaterRandom and mounted construction-attribute operations; no new RNG
  algorithm or seed. Production Simulation and graph calls share msLocked via
  one retained gate. INT_MIN preserves wrapping negation and the native invalid
  choice assertion, represented by a host error rather than silently clamped.
- Motion828f4d18: Into1333 -> Holding1273 (828f4490) -> Manual1027.
  Loaded ancestry gives37 occurrences; the final two are AttachIntent2429 and
  PlayAnimation2430. Added the exact existing occurrence sequence to Update.
- Action828ed168: Coffin18 -> OnBoard6, then packed slots19..22: two existing
  CreateMGIntentFromAGIntent updates and two CreateConstMGIntent updates.
- Action expression48: Pre828ff990 case0x30 uses
  `(c58 || c57) && ((c38 && !c42) || (c39 && c42))`, with lazy cached reads.
  Shared labels287/212/213 confirm the mirroring tail. Sustain82906048 case0x30
  uses `c38 || c39`, omitting entry and mirror gates.
- Action828edcf0: RocketAir154 -> Regular158, then packed slot23 constant.
- Action828eda60: FingerFlip204 -> Switch104, then slots23/24 constants.
  Original folded aliases of these Action thunks share the same packed update
  sequence. Dispatch is guarded by original address, count and OnBoard prefix;
  actual retained callbacks remain responsible for payload/instance checks.

## Corrected previous GetFootPos interpretation

Fresh raw PPC and debug types contradict the September23 evidence claiming
deck-X then flipped deck-Z. That old interpretation and its tests were wrong.

`PhysOut_SkateboardReckoning.mTransform` begins at0; Matrix44Affine has xAxis0,
yAxis16,zAxis32. GetFootPos825f5100 reads yAxis at825f52f4 (+0x10) and zAxis
at825f5280 (+0x20). It subtracts deck position from the selected foot position
and negates Z when SkateboardMotion.mSkateboardFlipped is true.

The IDA VMX128 VA aliases are misleading. Raw words decode as:

| Address | Word | Operands |
|---|---|---|
|825f52dc|177ff477|v59 = v127 - v126 (foot minus deck)|
|825f52f8|173bd1b5|v57 = dot3(v59,v58), where v58 is deck Y|
|825f52fc|171be9b7|v56 = dot3(v59,v125), where v125 is flipped deck Z|
|825f5300|16f8c825|v55 = permute(v56,v57,v0)|

The actual constant at82fd6e90 is
`00010203 14151617 00010203 00010203`: first output is forward distance,
second is height. Corrected the existing shared helper, not an additional one.
MongoPushFootToFar82caa870 still compares live threshold strictly against
result.x; PushOut82cafa80 still writes OutDistanceX then OutDistanceY.
Regression distinguishes sideways/forward/height, flipped boards and a90-degree
world rotation. No threshold, animation speed, or push impulse was tuned.

Confidence high for axis selection, permutation, branch and callback ordering.
Exact Xenon dot-product rounding remains unproved. This is a concrete defect
affecting push exits/out animation, but whether it explains every intermittent
push interruption requires an upcoming test. PushCycle82cd6590 and
existing held-push/BreakOutOfPush logic were inspected without speculative edits.
