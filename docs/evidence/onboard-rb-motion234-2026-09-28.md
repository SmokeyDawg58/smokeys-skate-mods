# Onboard RB and Motion expression234

The previous build can grab and move DMOs offboard. Reported:
98 ticks onboard RB -> FinishGround(GrabbableObjectSearchPending);
3364 ticks -> Motion(PendingExpression(234)).

## Source and reuse

Onboard RB reaches PhysicsGround.CheckForGrabbableObject82d9faa8. It searches
GSQ_VEHICLE=2, not GSQ_DMO=4. Ground finish retains its native prefix/timer
stores then calls that search; RB released still invalidates the query.

- Processed effective transform+80, distinct from animation+c0; source PDB
  offset checked. Reuse existing effective_transform_field accessor.
- GlobalAttributeCollection+188 is physics_state_skitching/default. Its class
  032cd3de521b4ecd is already selected, and the constructed Skitch state has
  an instance for this same collection. Reuse it instead of another attribute
  owner. Raw insrdi operands confirm half-dimensions978b2a7533b6675d, then
  offset8e3c1298e8072745. Existing read_floats handles native absent-field DDA.
- Reuse GrabBox.from_player / InitializeGrabbableBoxTransform82d8cc28 with
  usePitch=true. Radius is halfDimensions.z + linked kSearchRadius.
- PlayerGrabSpline.StartGrabSplineQuery radius82dc7220 -> deferred radius
 827b12b8. Added radius as an explicit variant in the EXISTING fixed16 query
 queue. Same future readiness reset, five-result maximum, position publication
 after enqueue, sorting and removal. Existing DMO box path stays in that queue.
- Ground.ConsiderGrabSplineQueryResults82d9a9c0 now consumes prior validated
  results: reset grab flag/descriptor/height (not old time or object ID),
  native cursor iteration and endpoint orientation, existing CanGrabSpline,
  reskitch delay/ID gate, box-local X=0 chord intersection, height, approach
  time, data requests and accepted descriptor/hand-collision disable2.
  Reciprocal refinements, transform/FMA order and scalar branches checked
  against decoded VMX, including82d9ac98..ad74 and82d9ae24..6c. No endpoint
  interpolation clamp; the accepted candidate requests data twice in source.
- Existing shared numeric/vector and clamp/geometry implementations are reused.

GrabSplineQuery827b1ed8 chooses authored vehicles by linked cvar. Authored
LWVehicleMan.GrabSplineQuery82d22e80 samples feed flag, entity map and linked
max-object-size + radius before traversal. The selected vehicle EntityMan is
initialized and empty. New provider validates that actual owner/map, producing
no hits; missing, uninitialized or populated owners remain explicit errors.
Legacy choice uses the existing vehicle near-query owner. No fabricated empty
result for a populated world; no DMO substitution for the vehicle query.

Populated vehicle spline feeding/data, traffic and Skitch104 gameplay are NOT
implemented by this fix. This supports onboard RB in the present empty-traffic
build and connects its source query/candidate flow. DMO movement remains the
separate working offboard path.

## Motion234

Live IDA: Pre jump-table8290aa90 +234*4 =8290ae38 ->82910c0c.
Case82910c2c reads condition341 (sConditions+554), calls
StateConditionDirect_ShouldLeaveSlide, caches/returns the unnegated result.
Sustain table8291e254 +234*4 =8291e5fc ->829221c8, constant true.
Existing ShouldLeaveSlide condition adapter and retained slide timers reused.
Added regression for false/true/error propagation and no Sustain reads.

## Linked source tuning

New unique references897..903, no hardcoded replacement tuning:

- kSearchRadius: init82f8f800/ref830d97d4, default5, Physics_States_Skitching.
- kDrawGrabbableObjectTest: init82f8f860/ref830d998c, false, Physics_States_PhysicsGround.
- kUseAuthoredVehicleGrabSplines: init82f699c8/ref83052250, true, AI_Vehicle.
- kFeedVehicleGrabSplines: init82f8b558/ref830a6eb8, true, AI_Vehicle.
- kSkitchMaxEntryAngle: init82f8f8b0,60 degrees.
- kSkitchMaxHorizontalEntryAngle: init82f8f910,40 degrees.
- kSkitchMinApproachSpeed: init82f8f970,.5.

Raw initializer r10=0 confirmed for float callbacks. Existing max-vehicle-size
reference reused. Native debug-overlay requirement remains behind its flag;
debug geometry rendering remains outside the selected presentation path.

## Validation

12 changed Rust files syntax-parsed with scoped rustfmt. Registration table is
a904-entry permutation0..903. Updated Ground branch-order regression and added
Motion234 regression; NOT executed. No Cargo or game launch.
To check: rebuild, hold/release RB while onboard, recheck offboard DMO grabs,
and repeat the action that reached expression234. Native Xenon numeric bit
limitations remain as documented by the shared numeric helpers.
