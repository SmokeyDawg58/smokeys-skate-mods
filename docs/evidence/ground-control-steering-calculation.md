# Ground control steering calculation

Read-only source packet and unregistered draft, September 12, 2026.
PhysicsGround.UpdateSkateboard82d9a088 calls Toolkit_CalcSteeringTilt82b27ae0.
This is not UpdateSteeringTilt82b29de0, whose retained application already exists.
The whole calculation has no cvar read and no grind-specific branch/callee.

Actual inputs already retained in the same Processed owner:
turn980 and hardTurn99c from ordered animation attributes; absBodySpeed948
and flippedControlsScalar94c from PrepareToolkit; truckTightness9c4 from
animation input; balance9a0 from existing physics state; push_contact flag
8c4/08000000 from ordered attributes. No missing input is replaced by zero.
Ground.fields.push_scalar and damped_turn already exist. Ground passes both
addresses at a07c/a080, while the native leaf supports either pointer being null.

## Ordered numerical and owner operations

1. Read turn7b18. Steering scalar lookup7b54 and value7b74, then hardTurn7b7c.
   Hard multiplier is fused abs(hardTurn)*scalar+1 at7b98. Nonzero hardTurn
   (including NaN) selects turn's fsel sign: +1 for ordered nonnegative, -1
   otherwise. Zero hardTurn preserves turn, including negative zero.
2. Only nonnull dampedTurn fetches coefficient7bf0. Remaining=1-coefficient,
   old field read7c0c, rounded coefficient*turn7c10, fused remaining*old+current
   7c14; store7c18. The absolute-turn branch uses ordered BLT; it preserves -0
   and NaN sign bits rather than applying fabs.
3. Speed divisor lookup7c84/value7c9c, then actual absBodySpeed7ca8. Native
   scalar fdivs7cac, two fsel clamps7cb4/7cbc: unordered selects +1.
   Steering cached layout graph0/20 at7cf0 uses normalized speed. Fresh layout
   graph40/60 at7d34 uses absolute turn. Existing PointGraph8 evaluates both.
4. Only nonnull pushScalar fetches increase7d88, decrease7de0, minimum7e38,
   in that order. Old push read7e54 precedes flags7e58. Set push_contact takes
   old-decrease; otherwise old+increase. Upper clamp7e78 is BO4/BI25 NOT GT;
   lower clamp7e88 is BO4/BI24 NOT LT. NaN remains selected, not clamped.
   Store7e90 precedes truck, manual and final control reads.
5. Capture truckTightness7ea0 before scalar lookup7edc. Fused scalar*T+1 at
   7ef8 precedes balance read7efc, then subtract T at7f04. Nonzero balance,
   including NaN, fetches global Manual+c8 scalar7f4c. Otherwise factor1.
6. Final steering scalar lookup7fa4/value7fbc precedes flippedControls7fc0.
   Separate rounded products7fc4..7fe0: flipped*scalar, *manual, *tightness,
   *push, *turnGraph, *speedGraph, *hardMultiplier, *signedTurn. No reassociation.

## Existing attribute owners and exact keys

Reuse SkateboardForceRuntime.steering_attributes, the already bound generated
Steering class3b76fd332f0d4d06/defaultd7edbd362d7d2152. Constructor828b0d20
uses DefaultDataArea80 for its cached layout. Two graph ranges0..40 and40..80
fit that exact layout. Reuse the board constructor's SkateboardAttributes
Manual instance, already bound classa0d7351efa176078/default at Global+c8.
No new per-call global instance or duplicate manual collection is required.
All scalar queries index0, null data falls back to the proven zero Default4.

| Lookup | Class | Key |
|---|---|---|
|7b54|Steering|dbd1d2af088ba302|
|7bf0|Steering|c40be048fab4f0b3|
|7c84|Steering|6481a86a7e6d5efc|
|7d88|Steering|db8f95ac3f0de414|
|7de0|Steering|e6b6a5e874f45937|
|7e38|Steering|be1a0232c2b2358b|
|7edc|Steering|365838a8b87eeda6|
|7f4c|Manual|4c9e095bdc2f1de5|
|7fa4|Steering|f0fc892ea70430cb|

Draft files: skateboard_steering_calculate.rs (one full algorithm),
skateboard_steering_calculate_attributes.rs (borrowed existing instances),
skeleton_processed_steering_calculate.rs (source-late actual retained fields),
and focused tests. They are unregistered during the parent gate.
Future live call must borrow the SAME Ground fields and actual constructed
board providers after a074, then retain returned tilt for COM-length/wobble
successors. It must not install turn or truck damping before those successors.
