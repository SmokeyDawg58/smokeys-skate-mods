# Live KnownAir trajectory binding, September 25

Playtesting showed camera height following an ordinary ollie, restricted airborne
spin, and less height on a quick flick than the original debug build. Scripted
runtime testing was used for this investigation.
Work remains solo.

## Reproduced cause and source check

The initial release replay completed 120 frames but stayed in Air200 for its
entire airborne interval. Prediction instrumentation then showed every onboard
candidate had duration0, maximum errors0/0 and no collision result. Selector
minimum time was also0. This was not evidence of absent terrain geometry.

`WallRideTrajectoryAttributes` already bound the correct physics_trajectory class
429a8f4385dd539d/default d7edbd362d7d2152. However, the shared selected original
asset loader omitted that class. AttributeRuntimeVault loads only selected
classes, so the real live binding was null and generated-wrapper fallback reads
returned zero. The positive prediction horizon was capped to zero before the
world query could walk any trajectory segments.

Fresh IDA checks:

- Generated constructor828b1068: raw1088/108c/1094 combines the class key above;
  calls FindCollection82976528 and Instance82975bb0; null layout uses24c fallback.
- Global constructor828b2914..291c constructs this same wrapper and stores it
  at GlobalAttributeCollection+e8.
- Instance82975bb0 caches the supplied collection's layout, without substituting
  a different collection.
- CalcMaxTime82dc0118 reads cached layout+1ac as cap. Private launch reads
  +1b0/+1b4 as the start/end sweep errors.
- Selector.Update82dc6528 selects the completed candidate and compares its
  collision time to layout+1e8; it publishes ready only after COM processing.
- CameraRig.UpdateSubjectState82e3fea0 and UpdateReferenceHeight82e3fb80 use
  valid trajectory/air state and the authored follow-in-air flag. No camera
  height clamp or forced physics-state transition was added.

Fix: expose the existing wrapper's class constant and include that class in
selected_original_asset_classes. Reuse the same Vault and selector; do not add
another trajectory implementation or substitute hardcoded tuning.

## Release validation

`cargo build --release -p skate2-pc -j 1` passed. Logs live in artifacts:

- knownair-camera-ollie-20260925.log: original Air200-only replay.
- knownair-prediction-trace-20260925.log: zero-duration root-cause trace.
- knownair-loaded-trajectory-build-20260925.log: corrected release build.
- knownair-loaded-trajectory-ollie-20260925.log: 120 completed frames,
  KnownAir201 at23..50, then Ground100; board rise approximately0.3483m.
- knownair-loaded-trajectory-high-20260925.log: 150 completed frames,
  KnownAir201 at51..94, then Ground100; board rise approximately1.0983m.
- knownair-loaded-trajectory-spin-20260925.log: 150 completed frames,
  KnownAir201 at51..95, then Ground100; board rise approximately1.0420m.

The first corrected quick-flick query has duration3.0722554, errors0.25/1.2,
collision time0.46988258 and selector minimum0.25. It hits the actual ground.
A hands-on look at the high ollie/spin found it much closer to Skate 2.
These are native-port observations, not quantitative original-versus-port parity.
Exact original quick-ollie height and camera matching still need comparison.

Optional SKATE2_TRACE_TRAJECTORY records the existing query inputs/results and
selector winner; SKATE2_TRACE_SELECTED_STATES also records board kinematics.
Normal launches do not enable them. The existing --live-ollie-demo now supports
`--live-ollie-hold 30` and --live-ollie-spin
for real raw-pad input replay, without changing physics. Default hold remains2.

Existing mounted bootstrap regression now requires the real trajectory layout
and positive cap/errors. Existing ollie and crouch corpus checks now require
KnownAir201 rather than accepting Air200-only flights. A held-left-stick high
ollie regression observes actual board heading and landing. Test execution
results will be appended after the focused test build completes.

Prior documented limits remain: active footplants, parked grind adjustment,
nonempty camera-trigger narrowphase and native terrain unloading are incomplete.

## Focused regression results

Mounted bootstrap PASS; fresh flatground Ollie PASS; quick/held Ollie and Nollie
PASS (board rises0.3471/1.1087m and0.4077/1.1100m respectively); high-ollie held-left-stick spin PASS, measured
277.82425degrees before landing. The earlier crouch assertion still expected
Air200 and failed despite a successful KnownAir jump; corrected that stale
assertion, rebuilt the test binary and reran successfully. No gameplay change
was needed for that assertion. Logs: knownair-bootstrap-regression-20260925.log,
knownair-ollie-regression-20260925.log, knownair-crouch-final-regression-20260925.log,
knownair-spin-regression-20260925.log. Test builds passed with existing warnings.
